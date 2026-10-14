//! Protobuf service registration attributes and rooted DI contracts.
#![cfg(feature = "grpc")]
#[test]
fn grpc_service_attribute_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui-grpc/pass/*.rs");
    #[cfg(feature = "jwt")]
    cases.pass("tests/ui-grpc/pass-jwt/*.rs");
    cases.compile_fail("tests/ui-grpc/fail/*.rs");
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .unwrap();
    if String::from_utf8_lossy(&output.stdout).starts_with("rustc 1.94.") {
        cases.compile_fail("tests/ui-grpc/fail/msrv/*.rs");
    } else {
        cases.compile_fail("tests/ui-grpc/fail/stable/*.rs");
    }
}
