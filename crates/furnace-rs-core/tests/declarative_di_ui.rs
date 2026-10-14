//! Compile-time registration, visibility, and dependency contracts.
#[test]
fn declared_roots_check_dependencies_without_running() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/declarative_di/pass/*.rs");
    let version = rustc_version();
    if version == (1, 94) {
        cases.compile_fail("tests/ui/declarative_di/fail/msrv/*.rs");
    } else {
        cases.compile_fail("tests/ui/declarative_di/fail/stable/*.rs");
        // Rust 1.99 changed E0423's wording for a type alias in value position.
        if version >= (1, 99) {
            cases.compile_fail("tests/ui/declarative_di/fail/rust-1.99/*.rs");
        } else {
            cases.compile_fail("tests/ui/declarative_di/fail/pre-1.99/*.rs");
        }
    }
}

fn rustc_version() -> (u32, u32) {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).unwrap();
    let mut parts = version.split_whitespace().nth(1).unwrap().split('.');
    (
        parts.next().unwrap().parse().unwrap(),
        parts.next().unwrap().parse().unwrap(),
    )
}
