//! Socket macro contracts and public facade expansion.
#![cfg(feature = "socket")]
#[test]
fn socket_registration_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui-socket/pass/*.rs");
    cases.compile_fail("tests/ui-socket/fail/*.rs");
}
