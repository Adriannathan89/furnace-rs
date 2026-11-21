//! The application-facing service holding the shared storage backend.

use crate::{Backend, redis_backend::RedisBackend};
use std::{fmt, sync::Arc};

/// The Redis URL given to [`RedisExtra::new`] was not usable.
#[derive(Debug)]
pub struct RedisExtraError(String);

impl fmt::Display for RedisExtraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RedisExtraError {}

/// Redis-backed service used by `#[cache]` and `#[rate_limit]` endpoints.
///
/// Supply it with `FurnaceBuilder::provide`.
#[derive(Clone)]
pub struct RedisExtra {
    backend: Arc<dyn Backend>,
}

impl RedisExtra {
    /// Creates the service for a `redis://` or `rediss://` URL.
    ///
    /// The connection is opened lazily, so an unreachable Redis does not
    /// block startup; request-time failures are logged and fail open.
    ///
    /// # Errors
    ///
    /// Returns an error for any other scheme, a URL containing `#`, or a URL
    /// the Redis client cannot parse.
    pub fn new(url: &str) -> Result<Self, RedisExtraError> {
        let invalid =
            || RedisExtraError("invalid Redis URL: expected redis:// or rediss://".into());
        if !(url.starts_with("redis://") || url.starts_with("rediss://")) || url.contains('#') {
            return Err(invalid());
        }
        if url.starts_with("rediss://") && rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let client = redis::Client::open(url).map_err(|_| invalid())?;
        Ok(Self::with_backend(RedisBackend::new(client)))
    }

    /// Wraps any storage backend, such as [`crate::MemoryBackend`] in tests.
    pub fn with_backend(backend: impl Backend) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Returns the shared backend.
    pub(crate) fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryBackend;

    #[test]
    fn new_rejects_invalid_urls() {
        for url in ["", "http://x", "redis://h:6379/#frag", "localhost:6379"] {
            assert!(RedisExtra::new(url).is_err(), "{url:?} must be rejected");
        }
    }

    #[test]
    fn new_accepts_redis_urls_without_connecting() {
        // Port 1 is closed: construction must not touch the network.
        assert!(RedisExtra::new("redis://127.0.0.1:1").is_ok());
        assert!(RedisExtra::new("rediss://u:p@127.0.0.1:1").is_ok());
    }

    #[test]
    fn with_backend_constructs() {
        let _ = RedisExtra::with_backend(MemoryBackend::default());
    }
}
