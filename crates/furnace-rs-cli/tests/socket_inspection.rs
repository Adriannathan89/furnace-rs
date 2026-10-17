//! Static socket inspection must not construct providers or contact infrastructure.
use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
#[test]
fn socket_reports_selected_endpoints_without_runtime_side_effects() {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("furnace"))
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inspection/socket"),
        )
        .args(["socket", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["command"], "socket");
    let endpoints = v["data"]["endpoints"].as_array().unwrap();
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0]["path"], "/chat");
    assert_eq!(endpoints[0]["mode"], "events");
    assert_eq!(endpoints[0]["handlers"][0]["name"], "echo");
}
