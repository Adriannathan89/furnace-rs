//! Dependency fields must explicitly opt into injection in every managed macro.
#![cfg(feature = "http")]

#[test]
fn managed_dependencies_require_injection_markers() {
    trybuild::TestCases::new().compile_fail("tests/ui-inject/fail/*.rs");
}
