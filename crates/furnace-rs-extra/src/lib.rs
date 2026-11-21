//! Optional Redis-backed endpoint caching and rate limiting for furnace-rs.
//!
//! Register a [`RedisExtra`] service with the application builder, then mark
//! controller endpoints with `#[cache(ttl = N)]` and
//! `#[rate_limit(limit = N, per = S)]`.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod backend;
mod identity;
mod redis_backend;
mod service;

pub use backend::{Backend, BackendError, BackendFuture, MemoryBackend, WindowHit};
pub use service::{RedisExtra, RedisExtraError};

/// Exposes the framework-neutral core boundary to extensions.
pub use furnace_rs_core as core;
