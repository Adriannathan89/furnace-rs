//! Exercise real HTTP responses and the load generator's accounting and exits.
use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{StatusCode, header},
    response::Response,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    net::SocketAddr,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

async fn invoke(
    address: SocketAddr,
    case: &str,
    operations: usize,
    concurrency: usize,
) -> (std::process::Output, Value) {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("report.json");
    let output_path = path.clone();
    let case = case.to_owned();
    let output = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_furnace-bench"))
            .args([
                "measure",
                "--scenario",
                &case,
                "--url",
                &format!("http://{address}"),
                "--operations",
                &operations.to_string(),
                "--concurrency",
                &concurrency.to_string(),
                "--seed",
                "42",
                "--output",
            ])
            .arg(output_path)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        path.exists(),
        "benchmark must write JSON for contract/transport failures; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output,
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
    )
}
async fn fixture(
    app: Router,
    case: &str,
    operations: usize,
    concurrency: usize,
) -> (std::process::Output, Value) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let result = invoke(address, case, operations, concurrency).await;
    task.abort();
    result
}
async fn hello(body: &'static str, case: &str) -> (std::process::Output, Value, usize) {
    let peers = Arc::new(Mutex::new(BTreeSet::new()));
    let captured = peers.clone();
    let app = Router::new().route(
        "/",
        get(move |ConnectInfo(peer): ConnectInfo<SocketAddr>| {
            let peers = captured.clone();
            async move {
                peers.lock().unwrap().insert(peer);
                body
            }
        }),
    );
    let (output, report) = fixture(app, case, 21, 3).await;
    let connections = peers.lock().unwrap().len();
    (output, report, connections)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn measures_every_response_and_reuses_one_connection_per_worker() {
    let (output, report, connections) = hello("Hello, world!", "hello").await;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report["http_responses"], 21);
    assert_eq!(report["status_counts"]["200"], 21);
    assert_eq!(report["operations"], 21);
    assert_eq!(report["completed_operations"], 21);
    assert_eq!(report["error_count"], 0);
    assert_eq!(report["passed"], true);
    assert_eq!(connections, 3);
    assert!(report["duration_seconds"].as_f64().unwrap() > 0.0);
    assert!(report["p99_ms"].as_f64().unwrap() >= report["p50_ms"].as_f64().unwrap());
    #[cfg(target_os = "linux")]
    assert!(report["client_cpu_seconds"].as_f64().unwrap() >= 0.0);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_incorrect_content_even_with_successful_statuses() {
    let (output, report, _) = hello("incorrect", "hello").await;
    assert!(!output.status.success());
    assert_eq!(report["http_responses"], 21);
    assert_eq!(report["error_count"], 21);
    assert_eq!(report["completed_operations"], 0);
    assert_eq!(report["passed"], false);
    assert!(!report["error_examples"].as_array().unwrap().is_empty());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connection_churn_never_reuses_an_idle_connection() {
    let (output, report, connections) = hello("Hello, world!", "connection-churn").await;
    assert!(output.status.success());
    assert_eq!(report["completed_operations"], 21);
    assert_eq!(connections, 21);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_statuses_and_redirects_are_not_hidden() {
    for status in [
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::MOVED_PERMANENTLY,
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let captured = calls.clone();
        let app = Router::new()
            .route(
                "/",
                get(move || {
                    let calls = captured.clone();
                    async move {
                        calls.fetch_add(1, Ordering::Relaxed);
                        (status, [(header::LOCATION, "/ok")], "Hello, world!")
                    }
                }),
            )
            .route("/ok", get(|| async { "Hello, world!" }));
        let (output, report) = fixture(app, "hello", 7, 3).await;
        assert!(!output.status.success());
        assert_eq!(report["http_responses"], 7);
        assert_eq!(report["error_count"], 7);
        assert_eq!(report["status_counts"][status.as_u16().to_string()], 7);
        assert_eq!(calls.load(Ordering::Relaxed), 7);
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_responses_fail_with_null_percentiles() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let (output, report) = invoke(address, "hello", 3, 2).await;
    assert!(!output.status.success());
    assert_eq!(report["http_responses"], 0);
    assert_eq!(report["completed_operations"], 0);
    assert_eq!(report["attempted_requests"], 3);
    assert_eq!(report["passed"], false);
    assert!(report["error_count"].as_u64().unwrap() >= 3);
    assert_eq!(report["p50_ms"], Value::Null);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn latency_waits_for_the_complete_response_body() {
    let app = Router::new().route(
        "/",
        get(|| async {
            let stream = futures_util::stream::once(async {
                tokio::time::sleep(Duration::from_millis(60)).await;
                Ok::<_, std::io::Error>("Hello, world!")
            });
            Response::new(Body::from_stream(stream))
        }),
    );
    let (output, report) = fixture(app, "hello", 1, 1).await;
    assert!(output.status.success());
    assert!(report["p50_ms"].as_f64().unwrap() >= 50.0);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auth_operations_count_both_dependent_responses() {
    let logins = Arc::new(AtomicUsize::new(0));
    let captured = logins.clone();
    let app = Router::new()
        .route(
            "/auth/login",
            post(move |axum::Json(body): axum::Json<Value>| {
                let count = captured.clone();
                async move {
                    assert_eq!(
                        body,
                        json!({"username":"demo","password":"correct-horse-battery-staple"})
                    );
                    count.fetch_add(1, Ordering::Relaxed);
                    axum::Json(json!({"access_token":"synthetic-token"}))
                }
            }),
        )
        .route(
            "/auth/me",
            get(|headers: axum::http::HeaderMap| async move {
                assert_eq!(headers["authorization"], "Bearer synthetic-token");
                axum::Json(json!({"id":1,"username":"demo"}))
            }),
        );
    let (output, report) = fixture(app, "auth", 7, 3).await;
    assert!(output.status.success());
    assert_eq!(report["completed_operations"], 7);
    assert_eq!(report["http_responses"], 14);
    assert_eq!(report["status_counts"]["200"], 14);
    assert_eq!(logins.load(Ordering::Relaxed), 7);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_login_payload_cannot_count_as_a_completed_operation() {
    let app = Router::new().route(
        "/auth/login",
        post(|| async { axum::Json(json!({"access_token":""})) }),
    );
    let (output, report) = fixture(app, "auth", 7, 3).await;
    assert!(!output.status.success());
    assert_eq!(report["completed_operations"], 0);
    assert_eq!(report["http_responses"], 7);
    assert_eq!(report["passed"], false);
    assert!(report["error_count"].as_u64().unwrap() >= 7);
}
#[test]
fn rejects_zero_work_and_unknown_arguments_without_starting_servers() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("report.json");
    for (arguments, message) in [
        (
            vec![
                "measure",
                "--scenario",
                "hello",
                "--url",
                "http://127.0.0.1:1",
                "--operations",
                "0",
            ],
            "positive",
        ),
        (vec!["compare", "--runs", "0"], "positive"),
        (vec!["compare", "--unknown-option"], "unknown argument"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_furnace-bench"))
            .args(arguments)
            .arg("--output")
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
        assert!(!path.exists());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborted_uploads_really_send_the_partial_body_before_recovery() {
    use futures_util::StreamExt;
    let aborted = Arc::new(AtomicUsize::new(0));
    let captured = aborted.clone();
    let app = Router::new()
        .route(
            "/auth/login",
            post(move |request: axum::extract::Request| {
                let count = captured.clone();
                async move {
                    if request
                        .headers()
                        .get(header::CONTENT_LENGTH)
                        .is_some_and(|v| v == "1048576")
                    {
                        let mut stream = request.into_body().into_data_stream();
                        let mut received = Vec::new();
                        let mut incomplete = false;
                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(chunk) => received.extend_from_slice(&chunk),
                                Err(_) => {
                                    incomplete = true;
                                    break;
                                }
                            }
                        }
                        assert!(incomplete, "the declared upload must not complete");
                        assert_eq!(received, vec![b'x'; 1024]);
                        count.fetch_add(1, Ordering::Relaxed);
                        return (
                            StatusCode::BAD_REQUEST,
                            axum::Json(json!({"error":"incomplete"})),
                        );
                    }
                    (
                        StatusCode::OK,
                        axum::Json(json!({"access_token":"synthetic-token"})),
                    )
                }
            }),
        )
        .route(
            "/auth/me",
            get(|| async { axum::Json(json!({"id":1,"username":"demo"})) }),
        );
    let (output, report) = fixture(app, "aborted-upload", 7, 3).await;
    assert!(output.status.success(), "{}", report);
    assert_eq!(report["completed_operations"], 7);
    assert_eq!(report["http_responses"], 14);
    tokio::time::timeout(Duration::from_secs(1), async {
        while aborted.load(Ordering::Relaxed) != 7 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("every partial upload must actually arrive");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn boundary_checks_exact_sizes_and_requires_signaled_closure() {
    for signal_close in [true, false] {
        let lengths = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
        let captured = lengths.clone();
        let app = Router::new()
            .route("/auth/login", post(move |request: axum::extract::Request| {
                let lengths = captured.clone();
                async move {
                    let bytes = axum::body::to_bytes(request.into_body(), 3*1024*1024).await.unwrap();
                    let value: Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(value["password"], "short");
                    *lengths.lock().unwrap().entry(bytes.len()).or_insert(0) += 1;
                    let oversized = bytes.len() > 2_097_152;
                    let mut response = Response::builder().status(if oversized {413} else {422});
                    if oversized && signal_close { response = response.header(header::CONNECTION, "close"); }
                    response.body(Body::from(json!({"error":{"code":if oversized {"payload_too_large"} else {"validation_error"}}}).to_string())).unwrap()
                }
            }))
            .route("/auth/me", get(|| async {
                (StatusCode::UNAUTHORIZED,[(header::WWW_AUTHENTICATE,"Bearer")],axum::Json(json!({"error":{"code":"unauthorized"}})))
            }));
        let (output, report) = fixture(app, "body-limit-boundary", 6, 2).await;
        assert_eq!(output.status.success(), signal_close, "{report}");
        assert_eq!(report["http_responses"], 12);
        assert_eq!(report["error_count"], if signal_close { 0 } else { 2 });
        assert_eq!(
            *lengths.lock().unwrap(),
            [(2_097_151, 2), (2_097_152, 2), (2_097_153, 2)]
                .into_iter()
                .collect()
        );
    }
}
