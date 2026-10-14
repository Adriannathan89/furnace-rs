//! Debug mode request logging at the HTTP server boundary.

#![cfg(feature = "http")]

use std::process::Command;
use std::time::Duration;

use furnace_rs_common::axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use furnace_rs_common::core::{ConfigBuilder, Furnace, TomlSource};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn run_fixture(configuration: &str) -> String {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("furnace.toml");
    std::fs::write(&path, configuration).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "request_logging_fixture", "--nocapture"])
        .env("FURNACE_DEBUG_TEST_CONFIG", &path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn debug_mode_logs_timestamp_final_status_and_path_for_each_request() {
    let output = run_fixture("[furnace]\nmode = 'debug'\n");
    let lines: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with('['))
        .collect();
    assert_eq!(lines.len(), 4, "{output}");
    for (line, expected) in lines.iter().zip([
        "201 | GET     | /debug-created",
        "500 | DELETE  | /debug-error",
        "404 | OPTIONS | /debug-missing",
        "408 | POST    | /debug-upload",
    ]) {
        let (timestamp, request) = line
            .strip_prefix("[debug] ")
            .unwrap()
            .split_once(" | ")
            .unwrap();
        assert_eq!(timestamp.len(), 23, "{line}");
        chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S%.3f")
            .expect("request timestamp must be a valid date and time");
        assert_eq!(request, expected);
        assert_eq!(
            line.match_indices('|')
                .map(|(index, _)| index)
                .collect::<Vec<_>>(),
            [32, 38, 48]
        );
    }
    assert!(!output.contains("secret=value"));
}

#[test]
fn requests_are_silent_without_debug_mode() {
    for configuration in ["", "[furnace]\nmode = 'release'\n"] {
        let output = run_fixture(configuration);
        assert!(
            !output.lines().any(|line| line.starts_with('[')),
            "{output}"
        );
    }
}

#[tokio::test]
async fn request_logging_fixture() {
    let Some(path) = std::env::var_os("FURNACE_DEBUG_TEST_CONFIG") else {
        return;
    };
    let config = ConfigBuilder::new()
        .source(TomlSource::file(path))
        .build()
        .unwrap();
    let debug = config.get("furnace.mode") == Some("debug");
    let application = Furnace::builder_with_config(config).build().await.unwrap();
    let router = Router::new()
        .route("/debug-created", get(|| async { StatusCode::CREATED }))
        .route(
            "/debug-error",
            furnace_rs_common::axum::routing::delete(|| async {
                StatusCode::INTERNAL_SERVER_ERROR
            }),
        )
        .route(
            "/debug-upload",
            post(|_: furnace_rs_common::axum::Json<serde_json::Value>| async { StatusCode::OK }),
        );
    // Reserve an ephemeral address before starting the public server entry point.
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let server = tokio::spawn(furnace_rs_common::serve_router(
        application,
        router,
        address,
    ));
    for (method, path, status) in [
        ("GET", "/debug-created?secret=value", "201"),
        ("DELETE", "/debug-error", "500"),
        ("OPTIONS", "/debug-missing", "404"),
    ] {
        let mut stream = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match tokio::net::TcpStream::connect(address).await {
                    Ok(stream) => break stream,
                    Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
                }
            }
        })
        .await
        .unwrap();
        stream
            .write_all(
                format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        assert!(response.starts_with(format!("HTTP/1.1 {status}").as_bytes()));
    }
    if debug {
        // The body deadline replaces the extractor rejection with the final 408.
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(b"POST /debug-upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 10\r\nConnection: close\r\n\r\n{").await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(15), stream.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        assert!(response.starts_with(b"HTTP/1.1 408"));
    }
    server.abort();
    let _ = server.await;
}
