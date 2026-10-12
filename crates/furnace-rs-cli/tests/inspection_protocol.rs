//! Process-level private inspection protocol coverage.

use std::{fs, path::Path};

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn incompatible_direct_furnace_version_is_rejected_before_launch() {
    let current = semver::Version::parse(furnace::FRAMEWORK_VERSION).unwrap();
    for incompatible in [
        semver::Version::new(current.major + 1, current.minor, 0),
        semver::Version::new(current.major, current.minor + 1, 0),
    ] {
        assert_rejected_before_launch(&incompatible, &current);
    }
}

fn assert_rejected_before_launch(incompatible: &semver::Version, current: &semver::Version) {
    let project = tempdir().expect("temporary project should be created");
    write_project(project.path(), incompatible);
    let marker = project.path().join("launched");

    let mut command = Command::cargo_bin("furnace").expect("CLI binary should build");
    command
        .current_dir(project.path())
        .env("INSPECTION_LAUNCH_MARKER", &marker)
        .arg("routes")
        .assert()
        .code(1)
        .stderr(contains("FURNACE203"))
        .stderr(contains(incompatible.to_string()))
        .stderr(contains(format!(
            "direct FURNACE {}.{} dependency",
            current.major, current.minor
        )));

    assert!(!marker.exists(), "incompatible application was launched");
}

fn write_project(root: &Path, version: &semver::Version) {
    fs::create_dir_all(root.join("app/src")).expect("application source directory should exist");
    fs::create_dir_all(root.join("furnace/src"))
        .expect("local FURNACE source directory should exist");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"furnace\"]\ndefault-members = [\"app\"]\nresolver = \"3\"\n",
    )
    .expect("workspace manifest should be written");
    fs::write(
        root.join("furnace/Cargo.toml"),
        format!("[package]\nname = \"furnace-rs\"\nversion = \"{version}\"\nedition = \"2024\"\n"),
    )
    .expect("FURNACE manifest should be written");
    fs::write(root.join("furnace/src/lib.rs"), "").expect("FURNACE source should be written");
    fs::write(
        root.join("app/Cargo.toml"),
        "[package]\nname = \"inspection-version-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nfurnace = { package = \"furnace-rs\", path = \"../furnace\" }\n",
    )
    .expect("application manifest should be written");
    fs::write(
        root.join("app/src/main.rs"),
        "fn main() { if let Ok(marker) = std::env::var(\"INSPECTION_LAUNCH_MARKER\") { std::fs::write(marker, \"launched\").unwrap(); } }\n",
    )
    .expect("application source should be written");
}
