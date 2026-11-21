//! Response caching behaviour, exercised through a plain Axum router.
#![allow(missing_docs)]
#![cfg(feature = "cache")]

use furnace_rs_common::axum::{
    Json, Router,
    body::{Body, to_bytes},
    http::{HeaderValue, Method, Request, StatusCode, header},
    middleware::from_fn_with_state,
    response::{IntoResponse, Response},
    routing::{MethodRouter, get},
};
use furnace_rs_extra::{
    Backend, BackendError, BackendFuture, CacheState, MemoryBackend, RedisExtra, WindowHit,
    cache_middleware,
};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
    time::Duration,
};
use tower::ServiceExt;

fn app(extra: &RedisExtra, route: MethodRouter) -> Router {
    Router::new().route(
        "/p",
        route.route_layer(from_fn_with_state(
            CacheState::new(extra, 60),
            cache_middleware,
        )),
    )
}

fn counting(counter: &Arc<AtomicUsize>) -> MethodRouter {
    let counter = counter.clone();
    let handler = move || {
        let counter = counter.clone();
        async move {
            counter.fetch_add(1, SeqCst);
            Json(json!({"ok": true}))
        }
    };
    get(handler.clone()).post(handler)
}

async fn send(app: &Router, method: Method, uri: &str, auth: Option<&str>) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(value) = auth {
        request = request.header(header::AUTHORIZATION, value);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn get_(app: &Router, uri: &str, auth: Option<&str>) -> Response {
    send(app, Method::GET, uri, auth).await
}

#[tokio::test]
async fn second_get_is_served_from_cache() {
    let counter = Arc::new(AtomicUsize::new(0));
    let app = app(
        &RedisExtra::with_backend(MemoryBackend::default()),
        counting(&counter),
    );
    let first = get_(&app, "/p", None).await;
    let second = get_(&app, "/p", None).await;
    assert_eq!(counter.load(SeqCst), 1);
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(second.headers()[header::CONTENT_TYPE], "application/json");
    let (a, b) = (
        to_bytes(first.into_body(), usize::MAX).await.unwrap(),
        to_bytes(second.into_body(), usize::MAX).await.unwrap(),
    );
    assert_eq!(a, b);
}

#[tokio::test]
async fn query_order_does_not_matter() {
    let counter = Arc::new(AtomicUsize::new(0));
    let app = app(
        &RedisExtra::with_backend(MemoryBackend::default()),
        counting(&counter),
    );
    get_(&app, "/p?a=1&b=2", None).await;
    get_(&app, "/p?b=2&a=1", None).await;
    assert_eq!(counter.load(SeqCst), 1);
}

#[tokio::test]
async fn different_credentials_do_not_share() {
    let counter = Arc::new(AtomicUsize::new(0));
    let app = app(
        &RedisExtra::with_backend(MemoryBackend::default()),
        counting(&counter),
    );
    get_(&app, "/p", Some("Bearer A")).await;
    get_(&app, "/p", Some("Bearer B")).await;
    get_(&app, "/p", None).await;
    assert_eq!(counter.load(SeqCst), 3);
    get_(&app, "/p", Some("Bearer A")).await;
    assert_eq!(counter.load(SeqCst), 3);
}

#[tokio::test]
async fn set_cookie_is_not_cached() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let route = get(move || {
        let c = c.clone();
        async move {
            c.fetch_add(1, SeqCst);
            ([(header::SET_COOKIE, HeaderValue::from_static("a=b"))], "x")
        }
    });
    let app = app(&RedisExtra::with_backend(MemoryBackend::default()), route);
    get_(&app, "/p", None).await;
    get_(&app, "/p", None).await;
    assert_eq!(counter.load(SeqCst), 2);
}

#[tokio::test]
async fn oversized_body_is_not_cached() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let route = get(move || {
        let c = c.clone();
        async move {
            c.fetch_add(1, SeqCst);
            vec![0u8; 1_048_576 + 1]
        }
    });
    let app = app(&RedisExtra::with_backend(MemoryBackend::default()), route);
    let first = get_(&app, "/p", None).await;
    assert_eq!(
        to_bytes(first.into_body(), usize::MAX).await.unwrap().len(),
        1_048_577
    );
    get_(&app, "/p", None).await;
    assert_eq!(counter.load(SeqCst), 2);
}

#[tokio::test]
async fn non_2xx_is_not_cached() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let route = get(move || {
        let c = c.clone();
        async move {
            c.fetch_add(1, SeqCst);
            StatusCode::NOT_FOUND.into_response()
        }
    });
    let app = app(&RedisExtra::with_backend(MemoryBackend::default()), route);
    get_(&app, "/p", None).await;
    get_(&app, "/p", None).await;
    assert_eq!(counter.load(SeqCst), 2);
}

/// Counts every backend call and delegates to memory.
struct Recording(MemoryBackend, Arc<AtomicUsize>);

impl Backend for Recording {
    fn get<'a>(&'a self, key: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        self.1.fetch_add(1, SeqCst);
        self.0.get(key)
    }
    fn set_ex<'a>(&'a self, key: &'a str, value: Vec<u8>, ttl: Duration) -> BackendFuture<'a, ()> {
        self.1.fetch_add(1, SeqCst);
        self.0.set_ex(key, value, ttl)
    }
    fn incr_window<'a>(&'a self, key: &'a str, window: Duration) -> BackendFuture<'a, WindowHit> {
        self.1.fetch_add(1, SeqCst);
        self.0.incr_window(key, window)
    }
}

#[tokio::test]
async fn non_get_bypasses() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::new(AtomicUsize::new(0));
    let extra = RedisExtra::with_backend(Recording(MemoryBackend::default(), calls.clone()));
    let app = app(&extra, counting(&counter));
    send(&app, Method::HEAD, "/p", None).await;
    send(&app, Method::POST, "/p", None).await;
    assert_eq!(calls.load(SeqCst), 0);
    assert_eq!(counter.load(SeqCst), 2);
}

/// Fails every call, or (when `corrupt`) serves undecodable bytes on read.
struct Broken {
    corrupt: bool,
}

impl Backend for Broken {
    fn get<'a>(&'a self, _: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        let corrupt = self.corrupt;
        Box::pin(async move {
            if corrupt {
                Ok(Some(vec![0xFF]))
            } else {
                Err(BackendError::new("down"))
            }
        })
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
    let counter = Arc::new(AtomicUsize::new(0));
    let app = app(
        &RedisExtra::with_backend(Broken { corrupt: false }),
        counting(&counter),
    );
    assert_eq!(get_(&app, "/p", None).await.status(), StatusCode::OK);
    assert_eq!(get_(&app, "/p", None).await.status(), StatusCode::OK);
    assert_eq!(counter.load(SeqCst), 2);
}

#[tokio::test]
async fn corrupt_entry_is_a_miss() {
    let counter = Arc::new(AtomicUsize::new(0));
    let app = app(
        &RedisExtra::with_backend(Broken { corrupt: true }),
        counting(&counter),
    );
    assert_eq!(get_(&app, "/p", None).await.status(), StatusCode::OK);
    assert_eq!(counter.load(SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn entry_expires_after_ttl() {
    let counter = Arc::new(AtomicUsize::new(0));
    let app = app(
        &RedisExtra::with_backend(MemoryBackend::default()),
        counting(&counter),
    );
    get_(&app, "/p", None).await;
    tokio::time::advance(Duration::from_secs(61)).await;
    get_(&app, "/p", None).await;
    assert_eq!(counter.load(SeqCst), 2);
}
