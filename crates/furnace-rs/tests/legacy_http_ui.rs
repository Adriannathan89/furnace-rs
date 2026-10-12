//! The breaking HTTP surface rejects retired registration and guard targets.
#![cfg(all(feature = "http", feature = "jwt"))]
#[test]
fn retired_http_api_is_rejected() {
    let msrv = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).starts_with("rustc 1.94."))
        .unwrap_or(false);
    let cases = if msrv {
        "tests/ui-legacy/fail/msrv/*.rs"
    } else {
        "tests/ui-legacy/fail/stable/*.rs"
    };
    trybuild::TestCases::new().compile_fail(cases);
}
