//! Optional Redis-backed endpoint caching and rate limiting for furnace-rs.
//!
//! Register a [`RedisExtra`] service with the application builder, then mark
//! controller endpoints with `#[cache(ttl = N)]` and
//! `#[rate_limit(limit = N, per = S)]`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod backend;
#[cfg(feature = "cache")]
mod caching;
mod identity;
#[cfg(feature = "rate-limit")]
mod limiting;
mod redis_backend;
mod service;

pub use backend::{Backend, BackendError, BackendFuture, MemoryBackend, WindowHit};
#[cfg(feature = "cache")]
pub use caching::{CacheState, cache_middleware};
#[cfg(feature = "rate-limit")]
pub use limiting::{RateLimitState, rate_limit_middleware};
pub use service::{RedisExtra, RedisExtraError};

/// Endpoint attributes `#[cache(ttl = N)]` and `#[rate_limit(limit = N, per = S)]`.
pub use furnace_rs_common::{cache, rate_limit};

/// Exposes the framework-neutral core boundary to extensions.
pub use furnace_rs_core as core;
