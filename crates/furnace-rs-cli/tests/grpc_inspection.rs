//! Process-level static protobuf inspection contracts.
use assert_cmd::Command;
use serde_json::Value;
use std::path::PathBuf;
fn fixture(name: &str) -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("furnace"));
    command.current_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/inspection")
            .join(name),
    );
    command
}
#[test]
fn grpc_without_registered_services_reports_empty_array() {
    let output = fixture("standard")
        .args(["grpc", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        format_args!(
            "{} {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["command"], "grpc");
    assert_eq!(value["data"]["services"], serde_json::json!([]));
}
#[test]
fn grpc_command_human_and_json_report_all_shapes_and_policies() {
    let output = fixture("grpc")
        .args(["grpc", "--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        format_args!(
            "{} {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["command"], "grpc");
    let service = &value["data"]["services"][0];
    assert_eq!(service["name"], "inspection.v1.Transport");
    assert!(
        service["implementation"]
            .as_str()
            .unwrap()
            .ends_with("::Service")
    );
    let methods = service["methods"].as_array().unwrap();
    assert_eq!(
        methods
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Bidi", "ClientStream", "ServerStream", "Unary"]
    );
    for (method, client, server) in [
        (0, true, true),
        (1, true, false),
        (2, false, true),
        (3, false, false),
    ] {
        assert_eq!(methods[method]["client_streaming"], client);
        assert_eq!(methods[method]["server_streaming"], server);
        assert_eq!(
            methods[method]["path"],
            format!(
                "/inspection.v1.Transport/{}",
                methods[method]["name"].as_str().unwrap()
            )
        );
    }
    assert!(methods[3]["protection"].is_null());
    assert_eq!(methods[0]["protection"]["strategy"], "jwt");
    assert_eq!(
        methods[0]["protection"]["roles"],
        serde_json::json!({"mode":"any", "values":["reader", "admin"]})
    );
    assert_eq!(
        methods[0]["protection"]["permissions"],
        serde_json::json!({"mode":"all", "values":["transport:read"]})
    );
    assert!(
        methods[0]["protection"]["predicates"][0]
            .as_str()
            .unwrap()
            .ends_with("allowed")
    );
    let human = fixture("grpc").arg("grpc").output().unwrap();
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    for text in [
        "inspection.v1.Transport",
        "unary",
        "server streaming",
        "client streaming",
        "bidirectional",
        "public",
        "reader",
    ] {
        assert!(human.contains(text), "missing {text}: {human}");
    }
}
#[test]
fn grpc_inspection_never_constructs_or_connects_or_reads_tls_material() {
    for command in ["grpc", "graph", "doctor"] {
        let output = fixture("grpc")
            .args([command, "--format", "json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{command}: {}",
            format_args!(
                "{} {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            )
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("inspection-secret-sentinel"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("missing-private-key.pem"));
    }
}
#[test]
fn grpc_selectors_unknown_arguments_and_static_errors_follow_existing_rules() {
    let selected = fixture("grpc")
        .args([
            "grpc",
            "--package",
            "inspection-grpc-fixture",
            "--bin",
            "inspection-grpc-fixture",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(selected.status.success());
    let unknown = fixture("grpc")
        .args(["grpc", "--format", "json", "--unknown"])
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    let value: Value = serde_json::from_slice(&unknown.stdout).unwrap();
    assert_eq!(value["command"], "grpc");
    assert_eq!(value["ok"], false);
    let invalid = fixture("grpc")
        .env("FURNACE_GRPC__SERVER__PORT", "0")
        .args(["doctor", "--format", "json"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let value: Value = serde_json::from_slice(&invalid.stdout).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert!(value["data"]["checks"].is_array());
    assert!(
        value["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "FURNACE151")
    );
}
