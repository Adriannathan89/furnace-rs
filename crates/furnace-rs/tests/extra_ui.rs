//! Compile contracts for the `cache` and `rate_limit` attributes.
#![cfg(feature = "extra")]
#[test]
fn extra_attributes_reject_invalid_declarations() {
    trybuild::TestCases::new().compile_fail("tests/ui-extra/*.rs");
}
