//! Storage contract shared by the cache and rate-limit layers.

use std::{
    collections::HashMap,
    fmt,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::Instant;

/// Boxed future returned by [`Backend`] methods.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'a>>;

/// A storage failure. Layers log it and fail open.
#[derive(Debug)]
pub struct BackendError(String);

impl BackendError {
    /// Creates an error from any message.
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BackendError {}

/// Result of counting one request in a fixed window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowHit {
    /// Requests counted in the current window, including this one.
    pub count: u64,
    /// Whole seconds until the window resets (at least 1).
    pub reset_secs: u64,
}

/// Key-value and counter storage used by [`crate::RedisExtra`].
pub trait Backend: Send + Sync + 'static {
    /// Reads a value, or `None` when absent or expired.
    fn get<'a>(&'a self, key: &'a str) -> BackendFuture<'a, Option<Vec<u8>>>;
    /// Stores a value that expires after `ttl`.
    fn set_ex<'a>(&'a self, key: &'a str, value: Vec<u8>, ttl: Duration) -> BackendFuture<'a, ()>;
    /// Atomically counts one hit; the window starts on the first hit.
    fn incr_window<'a>(&'a self, key: &'a str, window: Duration) -> BackendFuture<'a, WindowHit>;
}

/// In-process backend for tests and single-instance development.
///
/// Expiry uses [`tokio::time::Instant`], so paused test time controls it.
#[derive(Clone, Default)]
pub struct MemoryBackend {
    state: Arc<Mutex<MemoryState>>,
}

#[derive(Default)]
struct MemoryState {
    values: HashMap<String, (Vec<u8>, Instant)>,
    counters: HashMap<String, (u64, Instant)>,
}

impl Backend for MemoryBackend {
    fn get<'a>(&'a self, key: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        Box::pin(async move {
            let state = self.state.lock().expect("memory backend lock");
            Ok(state
                .values
                .get(key)
                .filter(|(_, expires)| *expires > Instant::now())
                .map(|(value, _)| value.clone()))
        })
    }

    fn set_ex<'a>(&'a self, key: &'a str, value: Vec<u8>, ttl: Duration) -> BackendFuture<'a, ()> {
        Box::pin(async move {
            let mut state = self.state.lock().expect("memory backend lock");
            state
                .values
                .insert(key.to_owned(), (value, Instant::now() + ttl));
            Ok(())
        })
    }

    fn incr_window<'a>(&'a self, key: &'a str, window: Duration) -> BackendFuture<'a, WindowHit> {
        Box::pin(async move {
            let now = Instant::now();
            let mut state = self.state.lock().expect("memory backend lock");
            let entry = state
                .counters
                .entry(key.to_owned())
                .and_modify(|(count, expires)| {
                    if *expires <= now {
                        *count = 0;
                        *expires = now + window;
                    }
                })
                .or_insert((0, now + window));
            entry.0 += 1;
            Ok(WindowHit {
                count: entry.0,
                reset_secs: (entry.1 - now).as_secs_f64().ceil().max(1.0) as u64,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn incr_window_counts_within_window() {
        let backend = MemoryBackend::default();
        for expected in 1..=3 {
            let hit = backend
                .incr_window("k", Duration::from_secs(60))
                .await
                .unwrap();
            assert_eq!(hit.count, expected);
            assert!((1..=60).contains(&hit.reset_secs));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn incr_window_resets_after_window() {
        let backend = MemoryBackend::default();
        backend
            .incr_window("k", Duration::from_secs(60))
            .await
            .unwrap();
        backend
            .incr_window("k", Duration::from_secs(60))
            .await
            .unwrap();
        tokio::time::advance(Duration::from_secs(61)).await;
        let hit = backend
            .incr_window("k", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(hit.count, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn set_ex_expires() {
        let backend = MemoryBackend::default();
        backend
            .set_ex("k", vec![1], Duration::from_secs(10))
            .await
            .unwrap();
        assert_eq!(backend.get("k").await.unwrap(), Some(vec![1]));
        tokio::time::advance(Duration::from_secs(11)).await;
        assert_eq!(backend.get("k").await.unwrap(), None);
    }
}
