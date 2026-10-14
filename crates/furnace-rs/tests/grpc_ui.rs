//! Protobuf service registration attributes and rooted DI contracts.
#![cfg(feature = "grpc")]
#[test]
fn grpc_service_attribute_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui-grpc/pass/*.rs");
    #[cfg(feature = "jwt")]
    cases.pass("tests/ui-grpc/pass-jwt/*.rs");
    cases.compile_fail("tests/ui-grpc/fail/*.rs");
}
