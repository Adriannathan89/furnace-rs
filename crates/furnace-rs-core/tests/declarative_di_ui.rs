//! Compile-time registration, visibility, and dependency contracts.
#[test]
fn declared_roots_check_dependencies_without_running() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/declarative_di/pass/*.rs");
    let output = std::process::Command::new("rustc").arg("--version").output().unwrap();
    assert!(output.status.success());
    let version = String::from_utf8(output.stdout).unwrap();
    if version.split_whitespace().nth(1).unwrap().starts_with("1.94.") {
        cases.compile_fail("tests/ui/declarative_di/fail/msrv/*.rs");
    } else {
        cases.compile_fail("tests/ui/declarative_di/fail/stable/*.rs");
    }
}
