//! Socket macro contracts and public facade expansion.
#![cfg(feature = "socket")]
#[test]
fn socket_registration_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui-socket/pass/*.rs");
    cases.compile_fail("tests/ui-socket/fail/*.rs");
    let version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .unwrap();
    if String::from_utf8_lossy(&version.stdout).starts_with("rustc 1.94.") {
        cases.compile_fail("tests/ui-socket/fail/msrv/*.rs");
    } else {
        cases.compile_fail("tests/ui-socket/fail/stable/*.rs");
    }
}
