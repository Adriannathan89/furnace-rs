//! Public compiler contracts for entity-bound storage.
#![cfg(feature = "sea-orm-storage")]

#[test]
fn storage_accepts_supported_declarations() {
    trybuild::TestCases::new().pass("tests/ui/storage/pass/*.rs");
}
#[test]
fn storage_rejects_invalid_declarations() {
    let version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("rustc must be available");
    assert!(version.status.success());
    let version = String::from_utf8_lossy(&version.stdout);
    let msrv = version.starts_with("rustc 1.94.");
    let modern_relation_diagnostics = rustc_at_least(&version, (1, 99));
    let root = std::path::Path::new("tests/ui/storage/fail");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    paths.sort();
    let cases = trybuild::TestCases::new();
    for path in paths {
        let name = path.file_name().unwrap();
        if name.to_str() == Some("invalid_selector.rs") && modern_relation_diagnostics {
            cases.compile_fail(root.join("rust_1_99").join(name));
        } else if msrv
            && matches!(
                name.to_str(),
                Some("invalid_selector.rs" | "legacy_loader.rs")
            )
        {
            cases.compile_fail(root.join("msrv").join(name));
        } else {
            cases.compile_fail(path);
        }
    }
}

fn rustc_at_least(version: &str, minimum: (u32, u32)) -> bool {
    let Some(version) = version.split_whitespace().nth(1) else {
        return false;
    };
    let mut components = version.split('.');
    let Some(major) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(minor) = components
        .next()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return false;
    };
    (major, minor) >= minimum
}
