//! Socket macro contracts and public facade expansion.
#![cfg(feature = "socket")]
use std::{path::PathBuf, process::Command};

#[test]
fn socket_registration_contracts() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui-socket/pass/*.rs");
    cases.compile_fail("tests/ui-socket/fail/*.rs");
}

#[test]
fn socket_gateway_requires_an_injector() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("tests/consumers/socket_injector/Cargo.toml");
    // Check the actual trait error instead of snapshotting rustc's unordered
    // suggestions for other Injector implementations. Check a valid control too
    // so an unrelated dependency or macro failure cannot satisfy this test.
    for missing_injector in [false, true] {
        let mut features = Vec::new();
        if cfg!(feature = "jwt") {
            features.push("jwt");
        }
        if missing_injector {
            features.push("missing-injector");
        }
        let mut command = Command::new(env!("CARGO"));
        command
            .args([
                "check",
                "--offline",
                "--message-format=json",
                "--manifest-path",
            ])
            .arg(&manifest)
            .env(
                "CARGO_TARGET_DIR",
                root.join("../../target/macro-consumers"),
            );
        if !features.is_empty() {
            command.args(["--features", &features.join(",")]);
        }
        let output = command
            .output()
            .expect("the socket consumer check should start");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.success(),
            !missing_injector,
            "unexpected compilation result (missing_injector={missing_injector})\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        if missing_injector {
            let has_injector_error = stdout
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .any(|record| {
                    record["reason"] == "compiler-message"
                        && record["message"]["level"] == "error"
                        && record["message"]["code"]["code"] == "E0277"
                        && record["message"]["message"]
                            .as_str()
                            .is_some_and(|message| message.contains("Gateway: Injector"))
                });
            assert!(
                has_injector_error,
                "missing Injector did not produce the intended trait error\nstdout:\n{stdout}\nstderr:\n{stderr}"
            );
        }
    }
}
