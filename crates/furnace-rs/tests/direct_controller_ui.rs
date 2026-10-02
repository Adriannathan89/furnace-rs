//! Compile contracts for inherent controller handlers.
#![cfg(feature = "http")]
#[test]
fn direct_controller_rejects_invalid_declarations() {
    trybuild::TestCases::new().compile_fail("tests/ui-direct/fail/*.rs");
}
