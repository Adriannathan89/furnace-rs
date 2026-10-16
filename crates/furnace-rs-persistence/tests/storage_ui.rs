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
    let msrv = String::from_utf8(version.stdout)
        .unwrap()
        .starts_with("rustc 1.94.");
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
        if msrv
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
