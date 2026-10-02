//! Compile contracts for static guard policy declarations.
#![cfg(all(feature = "http", feature = "jwt"))]
#[test]
fn static_policies_accept_typed_sources_and_authorization() {
    trybuild::TestCases::new().pass("tests/ui-seal/pass/*.rs");
}
#[test]
fn static_policies_reject_invalid_declarations() {
    trybuild::TestCases::new().compile_fail("tests/ui-seal/fail/*.rs");
}
