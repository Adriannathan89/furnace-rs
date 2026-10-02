//! Compile tests for the public FURNACE attribute macros.

#[test]
fn core_attributes_accept_supported_shapes() {
    trybuild::TestCases::new().pass("tests/ui/pass/*.rs");
}

#[test]
fn core_attributes_reject_unsupported_shapes() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/fail/*.rs");
    let version = rustc_version();
    if version >= (1, 99) {
        tests.compile_fail("tests/ui/fail/rust-1.99/*.rs");
    } else {
        tests.compile_fail("tests/ui/fail/pre-1.99/*.rs");
    }
    if version == (1, 94) {
        tests.compile_fail("tests/ui/fail/msrv/*.rs");
    } else {
        tests.compile_fail("tests/ui/fail/stable/*.rs");
    }
}

fn rustc_version() -> (u32, u32) {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .expect("rustc must be available for compile tests");
    assert!(output.status.success(), "rustc --version failed");
    let output = String::from_utf8(output.stdout).expect("rustc version must be UTF-8");
    let version = output
        .split_whitespace()
        .nth(1)
        .expect("rustc must report its version");
    let mut components = version.split('.');
    let major = components
        .next()
        .unwrap()
        .parse()
        .expect("Rust major version");
    let minor = components
        .next()
        .unwrap()
        .parse()
        .expect("Rust minor version");
    (major, minor)
}
