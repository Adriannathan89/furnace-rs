//! Redis implementation of [`Backend`]; connects lazily on first use.

use crate::{Backend, BackendError, BackendFuture, WindowHit};
use redis::{Script, aio::ConnectionManager};
use std::{sync::OnceLock, time::Duration};
use tokio::sync::OnceCell;

/// Counts one hit and (re)arms the window expiry in a single atomic step.
const WINDOW_SCRIPT: &str = r"
local c = redis.call('INCR', KEYS[1])
if c == 1 then redis.call('PEXPIRE', KEYS[1], ARGV[1]) end
local ttl = redis.call('PTTL', KEYS[1])
if ttl < 0 then redis.call('PEXPIRE', KEYS[1], ARGV[1]); ttl = tonumber(ARGV[1]) end
return {c, ttl}
";

pub(crate) struct RedisBackend {
    client: redis::Client,
    connection: OnceCell<ConnectionManager>,
    script: OnceLock<Script>,
}

impl RedisBackend {
    pub(crate) fn new(client: redis::Client) -> Self {
        Self {
            client,
            connection: OnceCell::new(),
            script: OnceLock::new(),
        }
    }

    async fn connection(&self) -> Result<ConnectionManager, BackendError> {
        self.connection
            .get_or_try_init(|| ConnectionManager::new(self.client.clone()))
            .await
            .cloned()
            .map_err(|e| BackendError::new(e.to_string()))
    }
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

impl Backend for RedisBackend {
    fn get<'a>(&'a self, key: &'a str) -> BackendFuture<'a, Option<Vec<u8>>> {
        Box::pin(async move {
            let mut conn = self.connection().await?;
            redis::cmd("GET")
                .arg(key)
                .query_async(&mut conn)
                .await
                .map_err(|e| BackendError::new(e.to_string()))
        })
    }

    fn set_ex<'a>(&'a self, key: &'a str, value: Vec<u8>, ttl: Duration) -> BackendFuture<'a, ()> {
        Box::pin(async move {
            let mut conn = self.connection().await?;
            redis::cmd("SET")
                .arg(key)
                .arg(value)
                .arg("PX")
                .arg(millis(ttl))
                .query_async(&mut conn)
                .await
                .map_err(|e| BackendError::new(e.to_string()))
        })
    }

    fn incr_window<'a>(&'a self, key: &'a str, window: Duration) -> BackendFuture<'a, WindowHit> {
        Box::pin(async move {
            let mut conn = self.connection().await?;
            let script = self.script.get_or_init(|| Script::new(WINDOW_SCRIPT));
            let (count, ttl_ms): (u64, i64) = script
                .key(key)
                .arg(millis(window))
                .invoke_async(&mut conn)
                .await
                .map_err(|e| BackendError::new(e.to_string()))?;
            Ok(WindowHit {
                count,
                reset_secs: u64::try_from(ttl_ms).unwrap_or(0).div_ceil(1000).max(1),
            })
        })
    }
}
