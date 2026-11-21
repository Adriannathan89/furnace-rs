//! Fixed-window rate limiting, exercised through a plain Axum router.
#![allow(missing_docs)]
#![cfg(feature = "rate-limit")]

use furnace_rs_common::{
    ValidatedCredential,
    axum::{
        Router,
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Request, StatusCode, header},
        middleware::{Next, from_fn, from_fn_with_state},
        response::Response,
        routing::get,
    },
};
use furnace_rs_extra::{
    Backend, BackendError, BackendFuture, MemoryBackend, RateLimitState, RedisExtra, WindowHit,
    rate_limit_middleware,
};
use std::{net::SocketAddr, time::Duration};
use tower::ServiceExt;

fn app(extra: &RedisExtra, limit: u64, guarded: bool, route: &str) -> Router {
    let state = RateLimitState::new(extra, limit, 60, guarded, route);
    Router::new().route(
        "/p",
        get(|| async { "ok" }).route_layer(from_fn_with_state(state, rate_limit_middleware)),
    )
}

fn memory() -> RedisExtra {
    RedisExtra::with_backend(MemoryBackend::default())
}

async fn hit(app: &Router, ip: Option<&str>, auth: Option<&str>) -> Response {
    let mut request = Request::builder().uri("/p");
    if let Some(value) = auth {
        request = request.header(header::AUTHORIZATION, value);
    }
    let mut request = request.body(Body::empty()).unwrap();
    if let Some(ip) = ip {
        let addr: SocketAddr = format!("{ip}:1234").parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(addr));
    }
    app.clone().oneshot(request).await.unwrap()
}

fn header_of(response: &Response, name: &str) -> String {
    response.headers()[name].to_str().unwrap().to_owned()
}

#[tokio::test]
async fn allows_up_to_limit_then_429() {
    let app = app(&memory(), 2, false, "r");
    let first = hit(&app, Some("1.1.1.1"), None).await;
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(header_of(&first, "x-ratelimit-remaining"), "1");
    let second = hit(&app, Some("1.1.1.1"), None).await;
    assert_eq!(header_of(&second, "x-ratelimit-remaining"), "0");

    let third = hit(&app, Some("1.1.1.1"), None).await;
    assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(header_of(&third, "retry-after").parse::<u64>().unwrap() >= 1);
    assert_eq!(header_of(&third, "x-ratelimit-limit"), "2");
    assert_eq!(header_of(&third, "x-ratelimit-remaining"), "0");
    let body = to_bytes(third.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        serde_json::json!({"error": {"code": "too_many_requests", "message": "rate limit exceeded"}})
    );
}

#[tokio::test(start_paused = true)]
async fn window_resets() {
    let app = app(&memory(), 1, false, "r");
    hit(&app, Some("1.1.1.1"), None).await;
    assert_eq!(
        hit(&app, Some("1.1.1.1"), None).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    tokio::time::advance(Duration::from_secs(61)).await;
    assert_eq!(
        hit(&app, Some("1.1.1.1"), None).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn ips_are_separate() {
    let app = app(&memory(), 1, false, "r");
    assert_eq!(
        hit(&app, Some("1.1.1.1"), None).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        hit(&app, Some("2.2.2.2"), None).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        hit(&app, Some("1.1.1.1"), None).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn missing_peer_uses_unknown_bucket() {
    let app = app(&memory(), 1, false, "r");
    assert_eq!(hit(&app, None, None).await.status(), StatusCode::OK);
    assert_eq!(
        hit(&app, None, None).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn public_endpoint_ignores_authorization() {
    let app = app(&memory(), 2, false, "r");
    hit(&app, Some("1.1.1.1"), Some("Bearer garbage-1")).await;
    hit(&app, Some("1.1.1.1"), Some("Bearer garbage-2")).await;
    let third = hit(&app, Some("1.1.1.1"), Some("Bearer garbage-3")).await;
    assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// Stands in for a Passport guard: installs the credential it "validated".
async fn fake_guard(mut request: Request<Body>, next: Next) -> Response {
    if let Some(value) = request.headers().get("x-validated").cloned() {
        request
            .extensions_mut()
            .insert(ValidatedCredential::new(value.to_str().unwrap()));
    }
    next.run(request).await
}

fn guarded_app(extra: &RedisExtra, limit: u64) -> Router {
    let state = RateLimitState::new(extra, limit, 60, true, "g");
    Router::new().route(
        "/p",
        get(|| async { "ok" })
            .route_layer(from_fn_with_state(state, rate_limit_middleware))
            .route_layer(from_fn(fake_guard)),
    )
}

async fn guarded_hit(app: &Router, validated: &str, junk: &str) -> StatusCode {
    let request = Request::builder()
        .uri("/p")
        .header("x-validated", validated)
        // Headers the guard did not validate must not influence the bucket.
        .header(header::AUTHORIZATION, format!("Bearer {junk}"))
        .header(header::COOKIE, format!("junk={junk}"))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn guarded_endpoint_buckets_by_validated_credential_only() {
    let app = guarded_app(&memory(), 1);
    assert_eq!(guarded_hit(&app, "token-a", "1").await, StatusCode::OK);
    assert_eq!(guarded_hit(&app, "token-b", "2").await, StatusCode::OK);
    assert_eq!(
        guarded_hit(&app, "token-a", "3").await,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn guarded_endpoint_without_validated_credential_falls_back_to_peer() {
    let app = app(&memory(), 1, true, "r");
    hit(&app, Some("1.1.1.1"), Some("Bearer A")).await;
    let second = hit(&app, Some("1.1.1.1"), Some("Bearer B")).await;
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test(start_paused = true)]
async fn hanging_backend_fails_open_without_hanging() {
    let app = app(&RedisExtra::with_backend(Hang), 1, false, "r");
    let response = tokio::time::timeout(Duration::from_secs(5), hit(&app, None, None))
        .await
        .expect("a stalled backend must not stall the request");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn routes_do_not_share_counters() {
    let extra = memory();
    let (a, b) = (
        app(&extra, 1, false, "route-a"),
        app(&extra, 1, false, "route-b"),
    );
    assert_eq!(
        hit(&a, Some("1.1.1.1"), None).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        hit(&b, Some("1.1.1.1"), None).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        hit(&a, Some("1.1.1.1"), None).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

/// Never answers, like a blackholed Redis.
struct Hang;

impl Backend for Hang {
    fn get<'a>(&'a self, _: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        Box::pin(std::future::pending())
    }
    fn set_ex<'a>(&'a self, _: &'a str, _: Vec<u8>, _: Duration) -> BackendFuture<'a, ()> {
        Box::pin(std::future::pending())
    }
    fn incr_window<'a>(&'a self, _: &'a str, _: Duration) -> BackendFuture<'a, WindowHit> {
        Box::pin(std::future::pending())
    }
}

struct Down;

impl Backend for Down {
    fn get<'a>(&'a self, _: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        Box::pin(async { Err(BackendError::new("down")) })
    }
    fn set_ex<'a>(&'a self, _: &'a str, _: Vec<u8>, _: Duration) -> BackendFuture<'a, ()> {
        Box::pin(async { Err(BackendError::new("down")) })
    }
    fn incr_window<'a>(&'a self, _: &'a str, _: Duration) -> BackendFuture<'a, WindowHit> {
        Box::pin(async { Err(BackendError::new("down")) })
    }
}

#[tokio::test]
async fn failing_backend_fails_open() {
    let app = app(&RedisExtra::with_backend(Down), 1, false, "r");
    for _ in 0..3 {
        let response = hit(&app, Some("1.1.1.1"), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key("x-ratelimit-limit"));
    }
}
