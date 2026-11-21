//! Fixed-window rate limiting, exercised through a plain Axum router.
#![allow(missing_docs)]
#![cfg(feature = "rate-limit")]

use furnace_rs_common::axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
    middleware::from_fn_with_state,
    response::Response,
    routing::get,
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

#[tokio::test]
async fn guarded_endpoint_separates_credentials() {
    let app = app(&memory(), 1, true, "r");
    assert_eq!(
        hit(&app, Some("1.1.1.1"), Some("Bearer A")).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        hit(&app, Some("1.1.1.1"), Some("Bearer B")).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        hit(&app, Some("1.1.1.1"), Some("Bearer A")).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
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
