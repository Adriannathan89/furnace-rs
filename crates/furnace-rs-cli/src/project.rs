//! Cargo workspace discovery and deterministic package and binary selection.

use std::path::{Path, PathBuf};

use cargo_metadata::{MetadataCommand, Package, TargetKind};

use crate::{
    command::TargetSelection,
    diagnostic::{CliError, FURNACE200, FURNACE201},
};

pub(crate) struct CargoProject {
    metadata: cargo_metadata::Metadata,
    invocation_root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedPackage {
    invocation_root: PathBuf,
    target_directory: PathBuf,
    workspace_root: PathBuf,
    package_root: PathBuf,
    manifest_path: PathBuf,
    package_id: cargo_metadata::PackageId,
    package_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedApplication {
    package: ResolvedPackage,
    binary_name: String,
    furnace_version: Option<semver::Version>,
    local_packages: Vec<(cargo_metadata::PackageId, PathBuf)>,
}

impl CargoProject {
    /// Reads Cargo metadata from the invocation directory and retains workspace identity.
    pub(crate) fn load(root: impl AsRef<Path>) -> Result<Self, CliError> {
        let invocation_root = root.as_ref().to_path_buf();
        let metadata = MetadataCommand::new()
            .current_dir(&invocation_root)
            .exec()
            .map_err(metadata_error)?;

        Ok(Self {
            metadata,
            invocation_root,
        })
    }

    /// Loads the same project metadata without blocking a runtime worker.
    ///
    /// Dropping this future kills the direct Cargo subprocess. Dependency
    /// resolution and invocation-directory configuration match [`Self::load`].
    pub(crate) async fn load_async(root: impl AsRef<Path>) -> Result<Self, CliError> {
        let invocation_root = root.as_ref().to_path_buf();
        let mut metadata_command = MetadataCommand::new();
        metadata_command.current_dir(&invocation_root);
        let output = tokio::process::Command::from(metadata_command.cargo_command())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(cargo_metadata::Error::from)
            .map_err(metadata_error)?;
        let metadata = parse_metadata_output(output).map_err(metadata_error)?;

        Ok(Self {
            metadata,
            invocation_root,
        })
    }

    /// Selects the named, root, or unambiguous default workspace package.
    pub(crate) fn resolve_package(
        &self,
        package: Option<&str>,
    ) -> Result<ResolvedPackage, CliError> {
        let selected = match package {
            Some(name) => self
                .metadata
                .workspace_packages()
                .into_iter()
                .find(|candidate| candidate.name == name)
                .ok_or_else(|| unknown_package(name, self.workspace_package_names()))?,
            None => match self.metadata.root_package() {
                Some(package) => package,
                None => self.resolve_default_package()?,
            },
        };

        Ok(self.resolved_package(selected))
    }

    /// Selects the application package and binary and records a direct Furnace version.
    pub(crate) fn resolve_application(
        &self,
        selection: &TargetSelection,
    ) -> Result<ResolvedApplication, CliError> {
        let package = self.resolve_package(selection.package.as_deref())?;
        let selected = &self.metadata[package.package_id()];
        let binary_name = resolve_binary(selected, selection.binary.as_deref())?;

        let reachable = self.reachable_package_ids(package.package_id());
        let local_packages = self
            .metadata
            .packages
            .iter()
            .filter(|package| package.source.is_none() && reachable.contains(&package.id))
            .map(|package| (package.id.clone(), package_root(package)))
            .collect();

        Ok(ResolvedApplication {
            local_packages,
            furnace_version: self.direct_furnace_version(selected),
            package,
            binary_name,
        })
    }

    /// Returns local packages reachable from the selected application for source watching.
    pub(crate) fn local_watch_roots(&self, application: &ResolvedApplication) -> Vec<PathBuf> {
        let reachable = self.reachable_package_ids(application.package().package_id());
        let mut roots = self
            .metadata
            .packages
            .iter()
            .filter(|package| package.source.is_none() && reachable.contains(&package.id))
            .map(package_root)
            .collect::<Vec<_>>();
        let selected_root = canonical_path(application.package().package_root());

        if !roots.contains(&selected_root) {
            roots.push(selected_root);
        }

        roots.sort();
        roots.dedup();

        roots
    }

    /// Returns all local workspace package roots for watcher exclusions.
    pub(crate) fn local_package_roots(&self) -> Vec<PathBuf> {
        let mut roots = self
            .metadata
            .packages
            .iter()
            .filter(|package| package.source.is_none())
            .map(package_root)
            .collect::<Vec<_>>();
        roots.sort();
        roots.dedup();

        roots
    }

    /// Requires exactly one Cargo default member when the workspace root is virtual.
    fn resolve_default_package(&self) -> Result<&Package, CliError> {
        let candidates = self.metadata.workspace_default_packages();

        match candidates.as_slice() {
            [package] => Ok(*package),
            [] => Err(CliError::new(
                FURNACE200,
                "Cargo application target is ambiguous",
                "no default workspace package can be selected",
            )
            .with_subject("workspace")
            .with_suggestion("pass --package <package>")),
            _ => {
                let names = sorted_package_names(candidates);

                Err(CliError::new(
                    FURNACE200,
                    "Cargo application target is ambiguous",
                    format!(
                        "more than one package can be selected: {}",
                        names.join(", ")
                    ),
                )
                .with_subject("workspace")
                .with_suggestion("pass --package <package>"))
            }
        }
    }

    /// Copies stable package identity and resolved paths from Cargo metadata.
    fn resolved_package(&self, package: &Package) -> ResolvedPackage {
        let manifest_path = package.manifest_path.as_std_path().to_path_buf();
        let package_root = manifest_path
            .parent()
            .expect("Cargo manifests always have a parent directory")
            .to_path_buf();

        ResolvedPackage {
            invocation_root: self.invocation_root.clone(),
            target_directory: self.metadata.target_directory.as_std_path().to_path_buf(),
            workspace_root: self.metadata.workspace_root.as_std_path().to_path_buf(),
            package_root,
            manifest_path,
            package_id: package.id.clone(),
            package_name: package.name.clone(),
        }
    }

    /// Returns workspace package names in sorted order.
    fn workspace_package_names(&self) -> Vec<String> {
        sorted_package_names(self.metadata.workspace_packages())
    }

    /// Finds the selected package’s direct furnace-rs or furnace-rs-grpc dependency version.
    fn direct_furnace_version(&self, package: &Package) -> Option<semver::Version> {
        let node = self
            .metadata
            .resolve
            .as_ref()?
            .nodes
            .iter()
            .find(|node| node.id == package.id)?;

        node.deps.iter().find_map(|dependency| {
            self.metadata
                .packages
                .iter()
                .find(|candidate| {
                    candidate.id == dependency.pkg
                        && matches!(
                            candidate.name.as_str(),
                            "furnace-rs" | "furnace-rs-grpc" | "furnace-rs-socket"
                        )
                })
                .map(|candidate| candidate.version.clone())
        })
    }

    /// Traverses dependency edges starting from the selected package.
    fn reachable_package_ids(
        &self,
        selected: &cargo_metadata::PackageId,
    ) -> Vec<cargo_metadata::PackageId> {
        let mut reachable = vec![selected.clone()];
        let Some(resolve) = &self.metadata.resolve else {
            return reachable;
        };

        let mut index = 0;

        while index < reachable.len() {
            let package_id = reachable[index].clone();

            if let Some(node) = resolve.nodes.iter().find(|node| node.id == package_id) {
                for dependency in &node.deps {
                    if !reachable.contains(&dependency.pkg) {
                        reachable.push(dependency.pkg.clone());
                    }
                }
            }

            index += 1;
        }

        reachable
    }
}

impl ResolvedPackage {
    /// Returns the directory whose Cargo configuration applies to this invocation.
    pub(crate) fn invocation_root(&self) -> &Path {
        &self.invocation_root
    }

    /// Returns Cargo's configured build output directory.
    pub(crate) fn target_directory(&self) -> &Path {
        &self.target_directory
    }

    /// Returns the Cargo workspace root directory.
    pub(crate) fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns the selected package directory.
    pub(crate) fn package_root(&self) -> &Path {
        &self.package_root
    }

    /// Returns the selected package manifest path.
    pub(crate) fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    /// Returns Cargo’s exact identity for the selected package.
    pub(crate) fn package_id(&self) -> &cargo_metadata::PackageId {
        &self.package_id
    }

    /// Returns the selected Cargo package name.
    pub(crate) fn package_name(&self) -> &str {
        &self.package_name
    }
}

impl ResolvedApplication {
    /// Resolves local build-script input paths relative to their owning package.
    pub(crate) fn local_package_root(&self, id: &cargo_metadata::PackageId) -> Option<&Path> {
        self.local_packages
            .iter()
            .find(|(package, _)| package == id)
            .map(|(_, root)| root.as_path())
    }

    /// Returns the resolved package associated with this binary.
    pub(crate) fn package(&self) -> &ResolvedPackage {
        &self.package
    }

    /// Returns the selected binary target name.
    pub(crate) fn binary_name(&self) -> &str {
        &self.binary_name
    }

    /// Returns the direct Furnace dependency version when present.
    pub(crate) fn furnace_version(&self) -> Option<&semver::Version> {
        self.furnace_version.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn test_identity(binary_name: &str) -> Self {
        Self {
            package: ResolvedPackage {
                invocation_root: PathBuf::from("/tmp/furnace-dev-state"),
                target_directory: PathBuf::from("/tmp/furnace-dev-state/target"),
                workspace_root: PathBuf::from("/tmp/furnace-dev-state"),
                package_root: PathBuf::from("/tmp/furnace-dev-state"),
                manifest_path: PathBuf::from("/tmp/furnace-dev-state/Cargo.toml"),
                package_id: serde_json::from_value(serde_json::Value::String(format!(
                    "furnace-dev-state {binary_name}"
                )))
                .expect("test package identity is valid"),
                package_name: "furnace-dev-state".into(),
            },
            binary_name: binary_name.into(),
            furnace_version: None,
            local_packages: Vec::new(),
        }
    }
}

/// Matches `MetadataCommand::exec` output handling and retained source errors.
fn parse_metadata_output(
    output: std::process::Output,
) -> Result<cargo_metadata::Metadata, cargo_metadata::Error> {
    if !output.status.success() {
        return Err(cargo_metadata::Error::CargoMetadata {
            stderr: String::from_utf8(output.stderr)?,
        });
    }

    let stdout = std::str::from_utf8(&output.stdout)?
        .lines()
        .find(|line| line.starts_with('{'))
        .ok_or(cargo_metadata::Error::NoJson)?;

    MetadataCommand::parse(stdout)
}

/// Preserves the stable diagnostic shared by synchronous and asynchronous loads.
fn metadata_error(error: cargo_metadata::Error) -> CliError {
    CliError::new(
        FURNACE201,
        "Cargo metadata could not be loaded",
        "failed to read Cargo project metadata",
    )
    .with_source(error)
}

/// Builds a selection diagnostic listing workspace package choices.
fn unknown_package(name: &str, package_names: Vec<String>) -> CliError {
    let available = if package_names.is_empty() {
        "the workspace contains no packages".to_owned()
    } else {
        format!("available packages: {}", package_names.join(", "))
    };

    CliError::new(
        FURNACE200,
        "Cargo application target is ambiguous",
        format!("no workspace package is named `{name}`; {available}"),
    )
    .with_subject(format!("package `{name}`"))
    .with_suggestion("pass --package <package>")
}

/// Selects the requested binary or requires an unambiguous package target.
fn resolve_binary(package: &Package, binary: Option<&str>) -> Result<String, CliError> {
    let binaries = package
        .targets
        .iter()
        .filter(|target| target.kind.contains(&TargetKind::Bin))
        .collect::<Vec<_>>();

    if let Some(binary) = binary {
        return binaries
            .iter()
            .find(|target| target.name == binary)
            .map(|target| target.name.clone())
            .ok_or_else(|| {
                CliError::new(
                    FURNACE200,
                    "Cargo application target is ambiguous",
                    format!(
                        "package `{}` has no binary target named `{binary}`",
                        package.name
                    ),
                )
                .with_subject(format!("package `{}`", package.name))
                .with_suggestion("pass --bin <binary>")
            });
    }

    if let Some(default_run) = &package.default_run {
        return binaries
            .iter()
            .find(|target| target.name == *default_run)
            .map(|target| target.name.clone())
            .ok_or_else(|| {
                CliError::new(
                    FURNACE200,
                    "Cargo application target is ambiguous",
                    format!(
                        "package `{}` declares default-run `{default_run}`, but no matching binary exists",
                        package.name
                    ),
                )
                .with_subject(format!("package `{}`", package.name))
                .with_suggestion("pass --bin <binary>")
            });
    }

    match binaries.as_slice() {
        [binary] => Ok(binary.name.clone()),
        [] => Err(CliError::new(
            FURNACE200,
            "Cargo application target is ambiguous",
            format!("package `{}` has no binary targets", package.name),
        )
        .with_subject(format!("package `{}`", package.name))
        .with_suggestion("pass --bin <binary>")),
        _ => {
            let mut names = binaries
                .iter()
                .map(|target| target.name.clone())
                .collect::<Vec<_>>();
            names.sort();

            Err(CliError::new(
                FURNACE200,
                "Cargo application target is ambiguous",
                format!(
                    "package `{}` has more than one binary target: {}",
                    package.name,
                    names.join(", ")
                ),
            )
            .with_subject(format!("package `{}`", package.name))
            .with_suggestion("pass --bin <binary>"))
        }
    }
}

/// Sorts package names for deterministic diagnostics and selection output.
fn sorted_package_names(packages: Vec<&Package>) -> Vec<String> {
    let mut names = packages
        .into_iter()
        .map(|package| package.name.clone())
        .collect::<Vec<_>>();
    names.sort();

    names
}

/// Returns the selected package directory.
fn package_root(package: &Package) -> PathBuf {
    let manifest = package.manifest_path.as_std_path();
    let root = manifest
        .parent()
        .expect("Cargo manifests always have a parent directory");

    canonical_path(root)
}

/// Uses the filesystem canonical path when available and otherwise preserves the input path.
fn canonical_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_package_and_one_binary_need_no_selectors() {
        let project = CargoProject::load(fixture("single")).unwrap();
        let app = project
            .resolve_application(&TargetSelection::default())
            .unwrap();
        assert_eq!(app.package().package_name(), "single-app");
        assert_eq!(app.binary_name(), "single-app");
    }

    #[tokio::test]
    async fn async_metadata_retains_sync_selection_and_invocation_identity() {
        for name in ["single", "multiple/api"] {
            let root = fixture(name);
            let synchronous = CargoProject::load(&root).unwrap();
            let asynchronous = CargoProject::load_async(&root).await.unwrap();
            let selection = TargetSelection::default();
            assert_eq!(
                asynchronous.resolve_application(&selection).unwrap(),
                synchronous.resolve_application(&selection).unwrap(),
            );
            assert_eq!(
                asynchronous.local_package_roots(),
                synchronous.local_package_roots(),
            );
        }
    }

    #[tokio::test]
    async fn async_metadata_preserves_the_failed_cargo_error_source() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Cargo.toml"), "invalid manifest").unwrap();
        let error = CargoProject::load_async(root.path()).await.err().unwrap();
        assert_eq!(error.code(), FURNACE201);
        assert!(matches!(
            std::error::Error::source(&error)
                .unwrap()
                .downcast_ref::<cargo_metadata::Error>(),
            Some(cargo_metadata::Error::CargoMetadata { .. })
        ));
    }

    #[test]
    fn default_run_wins_and_explicit_bin_overrides_it() {
        let project = CargoProject::load(fixture("multiple/api")).unwrap();
        let default = project
            .resolve_application(&TargetSelection::default())
            .unwrap();
        assert_eq!(default.binary_name(), "server");

        let worker = project
            .resolve_application(&TargetSelection {
                package: None,
                binary: Some("worker".into()),
            })
            .unwrap();
        assert_eq!(worker.binary_name(), "worker");
    }

    #[test]
    fn a_virtual_workspace_requires_package_selection_when_default_members_are_ambiguous() {
        let project = CargoProject::load(fixture("multiple")).unwrap();
        let error = project
            .resolve_application(&TargetSelection::default())
            .unwrap_err();
        assert_eq!(error.code(), FURNACE200);
        assert!(error.to_string().contains("--package"));
    }

    #[test]
    fn local_watch_roots_include_reachable_path_dependencies_only() {
        let workspace = tempfile::tempdir().unwrap();
        let root = workspace.path();
        let app = root.join("app");
        let dependency = root.join("dependency");
        let unrelated = root.join("unrelated");

        for package in [&app, &dependency, &unrelated] {
            std::fs::create_dir_all(package.join("src")).unwrap();
            std::fs::write(package.join("src/lib.rs"), "").unwrap();
        }

        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\", \"unrelated\"]\nresolver = \"3\"\n",
        )
        .unwrap();
        std::fs::write(
            app.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\ndependency = { path = \"../dependency\" }\n",
        )
        .unwrap();
        std::fs::write(app.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(
            dependency.join("Cargo.toml"),
            "[package]\nname = \"dependency\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            unrelated.join("Cargo.toml"),
            "[package]\nname = \"unrelated\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();

        let project = CargoProject::load(root).unwrap();
        let application = project
            .resolve_application(&TargetSelection {
                package: Some("app".into()),
                binary: None,
            })
            .unwrap();

        assert_eq!(
            project.local_watch_roots(&application),
            vec![canonical(&app), canonical(&dependency)],
        );
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/run")
            .join(name)
    }

    fn canonical(path: &std::path::Path) -> std::path::PathBuf {
        path.canonicalize().unwrap()
    }
}
