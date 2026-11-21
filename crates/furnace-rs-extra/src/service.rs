//! The application-facing service holding the shared storage backend.

use crate::{Backend, BackendError, BackendFuture, WindowHit, redis_backend::RedisBackend};
use std::{fmt, sync::Arc, time::Duration};

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

/// Storage of a `RedisExtra` nobody provided: every call explains the fix.
struct Unconfigured;

impl Backend for Unconfigured {
    fn get<'a>(&'a self, _: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        Box::pin(async { Err(not_provided()) })
    }
    fn set_ex<'a>(&'a self, _: &'a str, _: Vec<u8>, _: Duration) -> BackendFuture<'a, ()> {
        Box::pin(async { Err(not_provided()) })
    }
    fn incr_window<'a>(&'a self, _: &'a str, _: Duration) -> BackendFuture<'a, WindowHit> {
        Box::pin(async { Err(not_provided()) })
    }
}

fn not_provided() -> BackendError {
    BackendError::new(
        "RedisExtra was not provided: call `FurnaceBuilder::provide(RedisExtra::new(url)?)` before `build()`",
    )
}

impl RedisExtra {
    /// The placeholder the cauldron registers; requests fail open and log the fix.
    fn unconfigured() -> Self {
        Self::with_backend(Unconfigured)
    }
}

impl crate::core::Injector for RedisExtra {
    type Dependencies = ();

    /// Never fails, so applications that merely link this crate still build;
    /// `provide` replaces it with a real instance.
    async fn inject((): ()) -> crate::core::Result<Self> {
        Ok(Self::unconfigured())
    }

    fn descriptor() -> &'static crate::core::ProviderDescriptor {
        &REDIS_EXTRA_DESCRIPTOR
    }
}

const REDIS_EXTRA_DESCRIPTOR: crate::core::ProviderDescriptor =
    crate::core::__private::InjectorMetadata::<RedisExtra, RedisExtra>::DESCRIPTOR
        .with_authored_type_name("RedisExtra")
        .with_visibility(crate::core::ProviderVisibility::Public)
        .with_namespace(module_path!())
        .with_location(crate::core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));

crate::core::__private::inventory::submit! { REDIS_EXTRA_DESCRIPTOR }

/// Global module declaring the application's [`RedisExtra`].
///
/// Import it into the root cauldron, then supply the instance with
/// `FurnaceBuilder::provide(RedisExtra::new(url)?)`; a rooted application
/// rejects `provide` for a type that no imported cauldron registers.
/// Without the instance, requests fail open and each logs how to fix it.
#[crate::core::cauldron(providers = [RedisExtra], exports = [RedisExtra], global)]
pub struct ExtraCauldron;

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

    #[tokio::test]
    async fn unconfigured_service_explains_how_to_fix_itself() {
        let error = RedisExtra::unconfigured()
            .backend()
            .get("k")
            .await
            .expect_err("an unconfigured service has no storage");
        assert!(
            error.to_string().contains("FurnaceBuilder::provide"),
            "{error}"
        );
    }

    #[test]
    fn with_backend_constructs() {
        let _ = RedisExtra::with_backend(MemoryBackend::default());
    }
}
