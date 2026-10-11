//! A successful test means the invalid application was rejected by rustc.
#[test]
fn missing_logger_is_reported_during_cargo_test() {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let compiler = if String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .starts_with("1.94.")
    {
        "msrv"
    } else {
        "stable"
    };
    trybuild::TestCases::new().compile_fail(format!("tests/ui/{compiler}/missing_logger.rs"));
}
