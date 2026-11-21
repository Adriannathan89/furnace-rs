//! Redis-backed response cache for `#[cache(ttl = N)]` endpoints.

use crate::{
    Backend, RedisExtra,
    backend::bounded,
    identity::{canonical_query, fingerprint},
};
use furnace_rs_common::axum::{
    body::{Body, Bytes, HttpBody, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{sync::Arc, time::Duration};

/// Largest body that is cached; bigger or unbounded bodies pass through.
const MAX_BODY: usize = 1_048_576;

/// Per-endpoint cache settings, passed to [`cache_middleware`] as state.
#[derive(Clone)]
pub struct CacheState {
    backend: Arc<dyn Backend>,
    ttl: Duration,
}

impl CacheState {
    /// Caches successful GET responses for `ttl_secs` seconds.
    pub fn new(extra: &RedisExtra, ttl_secs: u64) -> Self {
        Self {
            backend: extra.backend().clone(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }
}

/// Serves cached GET responses and stores fresh 2xx ones; fails open on backend errors.
///
/// The key covers method, path, sorted query and a fingerprint of the
/// `Authorization`/`Cookie` headers, so one caller never sees another's entry.
pub async fn cache_middleware(
    State(state): State<CacheState>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() != Method::GET {
        return next.run(request).await;
    }
    let key = format!(
        "furnace:cache:GET:{}?{}:{}",
        request.uri().path(),
        canonical_query(request.uri().query()),
        fingerprint(request.headers()).unwrap_or_else(|| "anon".into()),
    );

    match bounded(state.backend.get(&key)).await {
        Ok(Some(bytes)) => {
            if let Some(response) = decode(&bytes) {
                return response;
            }
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "cache read failed; calling handler"),
    }

    let response = next.run(request).await;
    let cacheable = response.status().is_success()
        && !response.headers().contains_key(header::SET_COOKIE)
        && response
            .body()
            .size_hint()
            .upper()
            .is_some_and(|n| n <= MAX_BODY as u64);
    if !cacheable {
        return response;
    }

    let (parts, body) = response.into_parts();
    let Ok(body) = to_bytes(body, MAX_BODY).await else {
        tracing::warn!("cache could not buffer a response body");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let encoded = encode(parts.status, parts.headers.get(header::CONTENT_TYPE), &body);
    if let Err(error) = bounded(state.backend.set_ex(&key, encoded, state.ttl)).await {
        tracing::warn!(%error, "cache write failed");
    }
    Response::from_parts(parts, Body::from(body))
}

/// `u16` status, `u16` content-type length (0 = none), content-type, body.
fn encode(status: StatusCode, content_type: Option<&HeaderValue>, body: &[u8]) -> Vec<u8> {
    let content_type = content_type.map_or(&[][..], HeaderValue::as_bytes);
    let mut out = Vec::with_capacity(4 + content_type.len() + body.len());
    out.extend_from_slice(&status.as_u16().to_be_bytes());
    out.extend_from_slice(&u16::try_from(content_type.len()).unwrap_or(0).to_be_bytes());
    out.extend_from_slice(content_type);
    out.extend_from_slice(body);
    out
}

/// Returns `None` for any short or inconsistent input, which the caller treats as a miss.
fn decode(bytes: &[u8]) -> Option<Response> {
    let status = StatusCode::from_u16(u16::from_be_bytes(bytes.get(..2)?.try_into().ok()?)).ok()?;
    let len = usize::from(u16::from_be_bytes(bytes.get(2..4)?.try_into().ok()?));
    let content_type = bytes.get(4..4 + len)?;
    let body = Bytes::copy_from_slice(bytes.get(4 + len..)?);
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    if len > 0 {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_bytes(content_type).ok()?,
        );
    }
    Some(response)
}
