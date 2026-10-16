//! Public compiler contracts for entity-bound storage.
#![cfg(feature = "sea-orm-storage")]

#[test]
fn storage_accepts_supported_declarations() {
    trybuild::TestCases::new().pass("tests/ui/storage/pass/*.rs");
}
#[test]
fn storage_rejects_invalid_declarations() {
    trybuild::TestCases::new().compile_fail("tests/ui/storage/fail/*.rs");
}
