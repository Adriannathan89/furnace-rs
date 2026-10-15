//! Project watch-root selection and mapping filesystem events to dev-loop impacts.

use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::{
    diagnostic::{CliError, FURNACE220},
    project::{CargoProject, ResolvedApplication},
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ChangeImpact {
    None,
    Restart,
    Rebuild,
}

impl ChangeImpact {
    /// Combines impacts using rebuild over restart over no change.
    pub(crate) fn merge(impacts: impl IntoIterator<Item = Self>) -> Self {
        impacts.into_iter().max().unwrap_or(Self::None)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WatchRoot {
    path: PathBuf,
    recursive: bool,
}

impl WatchRoot {
    /// Returns the filesystem root registered with the watcher.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Reports whether the watcher traverses descendants of this root.
    pub(crate) const fn recursive(&self) -> bool {
        self.recursive
    }
}

#[derive(Clone)]
pub(crate) struct WatchSet {
    workspace_root: PathBuf,
    package_root: PathBuf,
    target_directory: PathBuf,
    local_roots: Vec<PathBuf>,
    excluded_package_roots: Vec<PathBuf>,
    notify_roots: Vec<WatchRoot>,
    build_inputs: Vec<PathBuf>,
    base_roots: Vec<WatchRoot>,
}

impl WatchSet {
    /// Builds reachable source roots and excludes unrelated local packages.
    pub(crate) fn for_application(
        project: &CargoProject,
        application: &ResolvedApplication,
    ) -> Self {
        let workspace_root = canonical_path(application.package().workspace_root());
        let package_root = canonical_path(application.package().package_root());
        let local_roots = project.local_watch_roots(application);
        let excluded_package_roots = project
            .local_package_roots()
            .into_iter()
            .filter(|root| !local_roots.contains(root))
            .collect::<Vec<_>>();
        let mut notify_roots = local_roots
            .iter()
            .cloned()
            .map(|path| WatchRoot {
                path,
                recursive: true,
            })
            .collect::<Vec<_>>();

        if !local_roots
            .iter()
            .any(|root| workspace_root.starts_with(root))
        {
            notify_roots.push(WatchRoot {
                path: workspace_root.clone(),
                recursive: false,
            });
        }
        notify_roots.sort_by(|left, right| left.path.cmp(&right.path));

        let initial = Self {
            workspace_root,
            package_root,
            target_directory: canonical_path(application.package().target_directory()),
            build_inputs: Vec::new(),
            base_roots: notify_roots.clone(),
            local_roots,
            excluded_package_roots,
            notify_roots,
        };
        initial.with_build_inputs(Vec::new())
    }

    /// Replaces explicitly declared inputs without broadening ordinary package file filtering.
    pub(crate) fn with_build_inputs(&self, inputs: Vec<PathBuf>) -> Self {
        let mut next = self.clone();
        next.notify_roots = self.base_roots.clone();
        next.build_inputs = inputs
            .into_iter()
            .map(|path| canonical_path(&path))
            .collect();
        for input in &next.build_inputs {
            if next
                .notify_roots
                .iter()
                .any(|root| root.recursive && input.starts_with(&root.path))
            {
                continue;
            }
            let (path, recursive) = if input.is_dir() {
                (input.clone(), true)
            } else {
                let mut parent = input.parent();
                while parent.is_some_and(|path| !path.is_dir()) {
                    parent = parent.and_then(Path::parent);
                }
                let Some(parent) = parent else {
                    continue;
                };
                // Missing parent directories require recursive events when they appear.
                (parent.to_path_buf(), input.parent() != Some(parent))
            };
            if let Some(root) = next.notify_roots.iter_mut().find(|root| root.path == path) {
                root.recursive |= recursive;
            } else {
                next.notify_roots.push(WatchRoot { path, recursive });
            }
        }
        // A recursive parent owns its descendants. Separate overlapping inotify
        // registrations can otherwise remove one another during replacement.
        next.notify_roots = next
            .notify_roots
            .iter()
            .filter(|root| {
                !next.notify_roots.iter().any(|parent| {
                    parent.path != root.path
                        && parent.recursive
                        && root.path.starts_with(&parent.path)
                })
            })
            .cloned()
            .collect();
        next.notify_roots
            .sort_by(|left, right| left.path.cmp(&right.path));
        next
    }

    /// Returns the paths and recursion modes passed to the filesystem watcher.
    pub(crate) fn roots(&self) -> &[WatchRoot] {
        &self.notify_roots
    }

    /// Maps a path to no change, restart, or rebuild while filtering ignored roots.
    pub(crate) fn classify(&self, path: impl AsRef<Path>) -> ChangeImpact {
        let path = path.as_ref();
        if path.starts_with(&self.target_directory) || is_ignored_path(path) {
            return ChangeImpact::None;
        }

        if self
            .build_inputs
            .iter()
            .any(|input| path == input || path.starts_with(input))
        {
            return ChangeImpact::Rebuild;
        }
        if self.owner_is_excluded(path) {
            return ChangeImpact::None;
        }

        if path.parent() == Some(self.package_root.as_path())
            && matches!(path.file_name(), Some(name) if name == OsStr::new(".env") || name == OsStr::new("furnace.toml"))
        {
            return ChangeImpact::Restart;
        }

        if self.is_rebuild_path(path) {
            ChangeImpact::Rebuild
        } else {
            ChangeImpact::None
        }
    }

    /// Identifies graph changes that require fresh Cargo metadata and registrations.
    pub(crate) fn reloads_project(&self, event: &Event) -> bool {
        self.classify_event(event) == ChangeImpact::Rebuild
            && event.paths.iter().any(|path| {
                matches!(path.file_name(), Some(name) if name == OsStr::new("Cargo.toml") || name == OsStr::new("Cargo.lock"))
                    && self.classify(path) == ChangeImpact::Rebuild
            })
    }

    /// Merges the impacts of changed paths and ignores access-only events.
    pub(crate) fn classify_event(&self, event: &Event) -> ChangeImpact {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                ChangeImpact::merge(event.paths.iter().map(|path| self.classify(path)))
            }
            EventKind::Access(_) | EventKind::Other | EventKind::Any => ChangeImpact::None,
        }
    }

    /// Checks whether the most specific owning package is outside the selected dependency graph.
    fn owner_is_excluded(&self, path: &Path) -> bool {
        self.owning_root(path)
            .is_some_and(|(_, reachable)| !reachable)
    }

    /// Finds the deepest reachable or excluded local package root containing a path.
    fn owning_root(&self, path: &Path) -> Option<(&Path, bool)> {
        self.local_roots
            .iter()
            .map(|root| (root.as_path(), true))
            .chain(
                self.excluded_package_roots
                    .iter()
                    .map(|root| (root.as_path(), false)),
            )
            .filter(|(root, _)| path.starts_with(root))
            .max_by_key(|(root, _)| root.components().count())
    }

    /// Recognizes workspace manifests and selected source, migration, and schema files.
    fn is_rebuild_path(&self, path: &Path) -> bool {
        if path == self.workspace_root.join("Cargo.toml")
            || path == self.workspace_root.join("Cargo.lock")
        {
            return true;
        }

        let is_reachable_source = self
            .owning_root(path)
            .is_some_and(|(_, reachable)| reachable);
        let is_build_input = matches!(path.extension(), Some(extension) if extension == OsStr::new("rs") || extension == OsStr::new("proto"));
        let is_package_manifest = path.file_name() == Some(OsStr::new("Cargo.toml"))
            && self
                .owning_root(path)
                .is_some_and(|(root, reachable)| reachable && path.parent() == Some(root));
        let is_selected_migration = path.starts_with(self.package_root.join("migrations"));
        let is_selected_schema = path == self.package_root.join("src/schema.rs")
            || path.starts_with(self.package_root.join("src/schema"));

        (is_reachable_source && is_build_input)
            || is_package_manifest
            || is_selected_migration
            || is_selected_schema
    }
}

pub(crate) struct WatchEvents {
    sender: Option<tokio::sync::mpsc::UnboundedSender<Result<Event, notify::Error>>>,
    _watcher: Option<notify::RecommendedWatcher>,
    receiver: tokio::sync::mpsc::UnboundedReceiver<Result<Event, notify::Error>>,
}

impl WatchEvents {
    /// Registers notify watchers for every configured root and forwards their events.
    pub(crate) fn start(watch_set: &WatchSet) -> Result<Self, CliError> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let watcher = Self::create_watcher(watch_set, sender.clone())?;
        Ok(Self {
            sender: Some(sender),
            _watcher: Some(watcher),
            receiver,
        })
    }

    fn create_watcher(
        watch_set: &WatchSet,
        sender: tokio::sync::mpsc::UnboundedSender<Result<Event, notify::Error>>,
    ) -> Result<notify::RecommendedWatcher, CliError> {
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })
        .map_err(watch_error)?;
        for root in watch_set.roots() {
            let mode = if root.recursive() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            watcher.watch(root.path(), mode).map_err(watch_error)?;
        }
        Ok(watcher)
    }

    #[cfg(test)]
    pub(crate) fn synthetic(
        receiver: tokio::sync::mpsc::UnboundedReceiver<Result<Event, notify::Error>>,
    ) -> Self {
        Self {
            sender: None,
            _watcher: None,
            receiver,
        }
    }

    /// Updates registrations on the existing event stream before dropping obsolete roots.
    pub(crate) fn replace_roots(
        &mut self,
        previous: &WatchSet,
        next: &WatchSet,
    ) -> Result<(), CliError> {
        if previous.roots() == next.roots() {
            return Ok(());
        }
        let Some(sender) = &self.sender else {
            return Ok(());
        };
        // Register a fresh watcher first. Swapping only after success keeps the
        // previous stream alive and avoids backend overlap/refcount assumptions.
        let watcher = Self::create_watcher(next, sender.clone())?;
        self._watcher = Some(watcher);
        Ok(())
    }

    /// Receives one filesystem event and maps watcher failures to CLI diagnostics.
    pub(crate) async fn recv(&mut self) -> Result<Event, CliError> {
        match self.receiver.recv().await {
            Some(Ok(event)) => Ok(event),
            Some(Err(error)) => Err(watch_error(error)),
            None => Err(CliError::new(
                FURNACE220,
                "File watcher failed",
                "the watch event channel closed unexpectedly",
            )),
        }
    }
}

/// Filters build output, VCS data, editor temporary files, and Finder metadata.
fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if name == OsStr::new("target") || name == OsStr::new(".git"))
    }) || path.file_name().is_some_and(|name| {
        name == OsStr::new(".DS_Store")
            || ["~", ".swp", ".swx", ".tmp"]
                .iter()
                .any(|suffix| name.to_string_lossy().ends_with(suffix))
    })
}

/// Uses a canonical path when available and otherwise keeps the supplied path.
fn canonical_path(path: &Path) -> PathBuf {
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    // Inputs may not exist yet. Resolve their existing ancestor (including
    // symlinks and ..) before appending the missing suffix.
    for ancestor in path.ancestors().skip(1) {
        if let Ok(root) = ancestor.canonicalize()
            && let Ok(suffix) = path.strip_prefix(ancestor)
        {
            let mut resolved = root;
            for component in suffix.components() {
                match component {
                    Component::CurDir => {}
                    Component::ParentDir => {
                        resolved.pop();
                    }
                    component => resolved.push(component.as_os_str()),
                }
            }
            return resolved;
        }
    }
    path.to_path_buf()
}

/// Wraps a notify failure in the stable file-watcher diagnostic.
fn watch_error(error: notify::Error) -> CliError {
    CliError::new(
        FURNACE220,
        "File watcher failed",
        "could not watch project files",
    )
    .with_source(error)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{ChangeImpact, WatchSet};

    #[test]
    fn classifies_relevant_paths_and_ignores_generated_and_editor_paths() {
        let workspace = tempfile::tempdir().unwrap();
        let workspace_root = workspace.path().to_path_buf();
        let root = workspace_root.join("app");

        for directory in [
            root.join("src/schema"),
            root.join("migrations/1"),
            workspace_root.join("target/debug"),
            workspace_root.join(".git"),
        ] {
            fs::create_dir_all(directory).unwrap();
        }

        let watch_set = WatchSet {
            target_directory: workspace_root.join("target"),
            build_inputs: Vec::new(),
            base_roots: Vec::new(),
            workspace_root: workspace_root.clone(),
            package_root: root.clone(),
            local_roots: vec![root.clone()],
            excluded_package_roots: Vec::new(),
            notify_roots: Vec::new(),
        };

        assert_eq!(
            watch_set.classify(root.join("src/main.rs")),
            ChangeImpact::Rebuild
        );
        assert_eq!(
            watch_set.classify(root.join("src/schema/user.rs")),
            ChangeImpact::Rebuild
        );
        assert_eq!(
            watch_set.classify(root.join("migrations/1/up.sql")),
            ChangeImpact::Rebuild
        );
        assert_eq!(
            watch_set.classify(root.join("Cargo.toml")),
            ChangeImpact::Rebuild
        );
        assert_eq!(
            watch_set.classify(workspace_root.join("Cargo.lock")),
            ChangeImpact::Rebuild
        );
        assert_eq!(
            watch_set.classify(root.join("furnace.toml")),
            ChangeImpact::Restart
        );
        assert_eq!(watch_set.classify(root.join(".env")), ChangeImpact::Restart);
        assert_eq!(
            watch_set.classify(workspace_root.join("target/debug/app")),
            ChangeImpact::None
        );
        assert_eq!(
            watch_set.classify(workspace_root.join(".git/index")),
            ChangeImpact::None
        );
        assert_eq!(
            watch_set.classify(root.join("src/main.rs~")),
            ChangeImpact::None
        );
    }

    #[test]
    fn declared_inputs_override_package_exclusions_and_merge_recursion_modes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let app = root.join("app");
        let excluded = root.join("sibling");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&excluded).unwrap();
        let initial = WatchSet {
            workspace_root: root.clone(),
            package_root: app.clone(),
            target_directory: root.join("target"),
            local_roots: vec![app.clone()],
            excluded_package_roots: vec![excluded.clone()],
            notify_roots: vec![
                super::WatchRoot {
                    path: root.clone(),
                    recursive: false,
                },
                super::WatchRoot {
                    path: app,
                    recursive: true,
                },
            ],
            build_inputs: Vec::new(),
            base_roots: Vec::new(),
        };
        let mut initial = initial;
        initial.base_roots = initial.notify_roots.clone();
        let explicit = initial.with_build_inputs(vec![excluded.join("input.data")]);
        assert_eq!(
            explicit.classify(excluded.join("input.data")),
            ChangeImpact::Rebuild
        );
        assert_eq!(
            explicit.classify(excluded.join("other.data")),
            ChangeImpact::None
        );
        let recursive = initial.with_build_inputs(vec![root.clone()]);
        assert_eq!(
            recursive
                .roots()
                .iter()
                .filter(|entry| entry.path() == root)
                .count(),
            1
        );
        assert!(
            recursive
                .roots()
                .iter()
                .find(|entry| entry.path() == root)
                .unwrap()
                .recursive()
        );
    }

    #[tokio::test]
    async fn watcher_retains_workspace_and_member_events_after_recursion_changes() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let app = root.join("app");
        let sibling = root.join("sibling");
        fs::create_dir_all(app.join("src")).unwrap();
        fs::create_dir_all(&sibling).unwrap();
        let base_roots = vec![
            super::WatchRoot {
                path: root.clone(),
                recursive: false,
            },
            super::WatchRoot {
                path: app.clone(),
                recursive: true,
            },
        ];
        let initial = WatchSet {
            workspace_root: root.clone(),
            package_root: app.clone(),
            target_directory: root.join("target"),
            local_roots: vec![app.clone()],
            excluded_package_roots: Vec::new(),
            notify_roots: base_roots.clone(),
            base_roots,
            build_inputs: Vec::new(),
        };
        let mut events = super::WatchEvents::start(&initial).unwrap();
        let upgraded = initial.with_build_inputs(vec![root.clone()]);
        assert_eq!(
            upgraded.roots().len(),
            1,
            "recursive parent replaces descendant registrations"
        );
        events.replace_roots(&initial, &upgraded).unwrap();
        let input = sibling.join("input.data");
        fs::write(&input, "one").unwrap();
        expect_event(&mut events, &input).await;

        let downgraded = upgraded.with_build_inputs(Vec::new());
        events.replace_roots(&upgraded, &downgraded).unwrap();
        let source = app.join("src/main.rs");
        fs::write(&source, "fn main() {} ").unwrap();
        expect_event(&mut events, &source).await;
        let manifest = root.join("Cargo.toml");
        fs::write(&manifest, "[workspace]").unwrap();
        expect_event(&mut events, &manifest).await;
    }

    #[tokio::test]
    async fn watches_creation_of_missing_external_declared_inputs() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let app = root.join("app");
        let shared = root.join("shared");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&shared).unwrap();
        let base_roots = vec![super::WatchRoot {
            path: app.clone(),
            recursive: true,
        }];
        let initial = WatchSet {
            workspace_root: root.clone(),
            package_root: app.clone(),
            target_directory: root.join("target"),
            local_roots: vec![app.clone()],
            excluded_package_roots: Vec::new(),
            notify_roots: base_roots.clone(),
            base_roots,
            build_inputs: Vec::new(),
        };
        let input = shared.join("new.input");
        let next = initial.with_build_inputs(vec![app.join("../shared/new.input")]);
        assert_eq!(next.classify(&input), ChangeImpact::Rebuild);
        let mut events = super::WatchEvents::start(&initial).unwrap();
        events.replace_roots(&initial, &next).unwrap();
        fs::write(&input, "created").unwrap();
        expect_event(&mut events, &input).await;

        let missing_parent =
            initial.with_build_inputs(vec![app.join("missing/../../shared/second.input")]);
        assert_eq!(
            missing_parent.classify(shared.join("second.input")),
            ChangeImpact::Rebuild
        );
    }

    async fn expect_event(events: &mut super::WatchEvents, path: &std::path::Path) {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if events
                    .recv()
                    .await
                    .unwrap()
                    .paths
                    .iter()
                    .any(|event| event == path)
                {
                    break;
                }
            }
        })
        .await
        .expect("filesystem registration must remain active");
    }

    #[test]
    fn rebuild_dominates_restart_in_a_change_batch() {
        assert_eq!(
            ChangeImpact::merge([ChangeImpact::Restart, ChangeImpact::Rebuild]),
            ChangeImpact::Rebuild,
        );
    }

    #[test]
    fn excluded_nested_package_owns_its_source_path() {
        let workspace = tempfile::tempdir().unwrap();
        let workspace_root = workspace.path().to_path_buf();
        let root = workspace_root.join("app");
        let excluded = root.join("examples/tool");
        let watch_set = WatchSet {
            target_directory: workspace_root.join("target"),
            build_inputs: Vec::new(),
            base_roots: Vec::new(),
            workspace_root,
            package_root: root.clone(),
            local_roots: vec![root],
            excluded_package_roots: vec![excluded.clone()],
            notify_roots: Vec::new(),
        };

        assert_eq!(
            watch_set.classify(excluded.join("src/main.rs")),
            ChangeImpact::None
        );
    }

    #[test]
    fn classifies_both_rename_paths_and_ignores_access_events() {
        let workspace = tempfile::tempdir().unwrap();
        let workspace_root = workspace.path().to_path_buf();
        let root = workspace_root.join("app");
        let watch_set = WatchSet {
            target_directory: workspace_root.join("target"),
            build_inputs: Vec::new(),
            base_roots: Vec::new(),
            workspace_root,
            package_root: root.clone(),
            local_roots: vec![root.clone()],
            excluded_package_roots: Vec::new(),
            notify_roots: Vec::new(),
        };

        let rename = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::Both,
            )),
            paths: vec![root.join(".env"), root.join("src/main.rs")],
            attrs: notify::event::EventAttributes::default(),
        };
        assert_eq!(watch_set.classify_event(&rename), ChangeImpact::Rebuild);

        let access = notify::Event::new(notify::EventKind::Access(notify::event::AccessKind::Any));
        assert_eq!(watch_set.classify_event(&access), ChangeImpact::None);
    }
}
