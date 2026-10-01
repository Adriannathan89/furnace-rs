//! The breaking HTTP surface rejects retired registration and guard targets.
#![cfg(all(feature = "http", feature = "jwt"))]
#[test]
fn retired_http_api_is_rejected() {
    trybuild::TestCases::new().compile_fail("tests/ui-legacy/fail/*.rs");
}
