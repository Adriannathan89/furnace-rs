//! The application-facing service holding the shared storage backend.

use crate::Backend;
use std::sync::Arc;

/// Redis-backed service used by `#[cache]` and `#[rate_limit]` endpoints.
///
/// Supply it with `FurnaceBuilder::provide`.
#[derive(Clone)]
pub struct RedisExtra {
    backend: Arc<dyn Backend>,
}

impl RedisExtra {
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
    fn with_backend_constructs() {
        let _ = RedisExtra::with_backend(MemoryBackend::default());
    }
}
