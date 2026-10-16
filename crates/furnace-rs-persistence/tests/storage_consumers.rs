//! Independent consumers exercise feature gates and Cargo dependency renaming.
#![cfg(feature = "sea-orm-storage")]
use std::{path::PathBuf, process::Command};

#[test]
fn independent_consumers_validate_features_and_renamed_dependencies() {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = package.join("../../target/storage-consumers");
    for (name, expected) in [
        ("core", None),
        ("facade", None),
        ("renamed", None),
        ("renamed_core", None),
        ("version204", None),
        (
            "disabled",
            Some("entity-bound storage requires the `sea-orm-storage` feature"),
        ),
        (
            "missing",
            Some("entity-bound storage requires a direct `furnace-rs-persistence` dependency"),
        ),
    ] {
        let output = Command::new(env!("CARGO"))
            .args(["check", "--offline", "--manifest-path"])
            .arg(package.join(format!("tests/consumers/storage_{name}/Cargo.toml")))
            .env("CARGO_TARGET_DIR", &target)
            .output()
            .expect("consumer cargo check must run");
        let stderr = String::from_utf8_lossy(&output.stderr);
        match expected {
            None => assert!(output.status.success(), "consumer {name}: {stderr}"),
            Some(message) => {
                assert!(
                    !output.status.success(),
                    "negative consumer {name} unexpectedly succeeded"
                );
                assert!(
                    stderr.contains(message),
                    "consumer {name} failed for another reason: {stderr}"
                );
            }
        }
    }
}
