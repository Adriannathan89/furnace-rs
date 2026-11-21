//! Redis-backed fixed-window rate limiting for `#[rate_limit(..)]` endpoints.

use crate::{Backend, RedisExtra, identity::fingerprint};
use furnace_rs_common::axum::{
    Json,
    extract::{ConnectInfo, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};

const LIMIT_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const REMAINING_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-remaining");

/// Per-endpoint limit settings, passed to [`rate_limit_middleware`] as state.
#[derive(Clone)]
pub struct RateLimitState {
    backend: Arc<dyn Backend>,
    limit: u64,
    window: Duration,
    guarded: bool,
    route: Arc<str>,
}

impl RateLimitState {
    /// Allows `limit` requests per `per_secs`-second window for each client of `route`.
    ///
    /// Set `guarded` only for endpoints behind a Passport guard: their clients
    /// are told apart by credential, others by peer IP, so unauthenticated
    /// callers cannot mint new buckets with made-up credentials.
    pub fn new(extra: &RedisExtra, limit: u64, per_secs: u64, guarded: bool, route: &str) -> Self {
        Self {
            backend: extra.backend().clone(),
            limit,
            window: Duration::from_secs(per_secs),
            guarded,
            route: route.into(),
        }
    }

    fn identity(&self, request: &Request) -> String {
        if self.guarded
            && let Some(credential) = fingerprint(request.headers())
        {
            return credential;
        }
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map_or_else(|| "unknown".into(), |peer| peer.0.ip().to_string())
    }
}

/// Counts the request and answers `429` once the window's limit is exceeded.
///
/// Backend errors are logged and the request proceeds (fail open).
pub async fn rate_limit_middleware(
    State(state): State<RateLimitState>,
    request: Request,
    next: Next,
) -> Response {
    let key = format!("furnace:rl:{}:{}", state.route, state.identity(&request));
    let hit = match state.backend.incr_window(&key, state.window).await {
        Ok(hit) => hit,
        Err(error) => {
            tracing::warn!(%error, "rate limit check failed; allowing request");
            return next.run(request).await;
        }
    };

    let mut response = if hit.count > state.limit {
        let body = serde_json::json!({
            "error": {"code": "too_many_requests", "message": "rate limit exceeded"}
        });
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, HeaderValue::from(hit.reset_secs))],
            Json(body),
        )
            .into_response()
    } else {
        next.run(request).await
    };
    let headers = response.headers_mut();
    headers.insert(LIMIT_HEADER, HeaderValue::from(state.limit));
    headers.insert(
        REMAINING_HEADER,
        HeaderValue::from(state.limit.saturating_sub(hit.count)),
    );
    response
}
