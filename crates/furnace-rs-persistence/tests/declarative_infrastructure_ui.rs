//! Infrastructure metadata resolves across persistence, common and facade crates.
#![cfg(feature = "sea-orm-postgres")]
#[test]
fn database_and_logger_contracts_match_root_imports() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/declarative_infrastructure/pass/*.rs");
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    if String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .starts_with("1.94.")
    {
        cases.compile_fail("tests/ui/declarative_infrastructure/fail/msrv/*.rs");
    } else {
        cases.compile_fail("tests/ui/declarative_infrastructure/fail/stable/*.rs");
    }
}
