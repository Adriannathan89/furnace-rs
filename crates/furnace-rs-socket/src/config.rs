//! Pure configuration validation; file and network access occur only in preparation.
use furnace_rs_core::{Config, Diagnostic, DiagnosticCode, Error, Result};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
/// Validated separate-listener configuration.
#[derive(Clone)]
pub struct SocketServerConfig {
    /// Explicit bind address; port zero is useful for native composition.
    pub address: SocketAddr,
    /// Maximum assembled incoming message size.
    pub max_message_bytes: usize,
    /// Maximum queued messages per managed connection.
    pub queue_messages: usize,
    /// Maximum queued payload bytes per managed connection.
    pub queue_bytes: usize,
    /// Interval between heartbeat pings.
    pub heartbeat_interval: Duration,
    /// Maximum response time for a heartbeat pong.
    pub pong_timeout: Duration,
    /// Maximum transport drain time.
    pub shutdown_timeout: Duration,
    /// Lease renewal interval.
    pub lease_refresh: Duration,
    /// Instance lease lifetime.
    pub lease_expiry: Duration,
    /// Allowed exact browser origins.
    pub origins: Vec<String>,
    /// Application isolation namespace.
    pub namespace: String,
    pub(crate) redis_url: Option<String>,
    pub(crate) certificate: Option<PathBuf>,
    pub(crate) private_key: Option<PathBuf>,
}
impl std::fmt::Debug for SocketServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocketServerConfig")
            .field("address", &self.address)
            .field("secrets", &"[REDACTED]")
            .finish()
    }
}
impl SocketServerConfig {
    /// Validates listener settings without reading files or contacting Redis.
    #[allow(clippy::result_large_err)]
    pub fn parse(c: &Config) -> Result<Self> {
        let address = c
            .get("socket.address")
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| invalid("socket.address"))?;
        for table in ["socket", "socket.tls", "socket.redis"] {
            if c.get(table).is_some() || c.get_string_array(table).is_some() {
                return Err(invalid(table));
            }
        }
        let number = |key: &str, default: usize| -> Result<usize> {
            if c.get_string_array(key).is_some() || c.contains_table(key) {
                return Err(invalid(key));
            }
            match c.get(key) {
                Some(v) => v
                    .parse()
                    .ok()
                    .filter(|v| *v > 0)
                    .ok_or_else(|| invalid(key)),
                None => Ok(default),
            }
        };
        let duration = |key, default| number(key, default).map(|v| Duration::from_millis(v as u64));
        let origins = if let Some(v) = c.get_string_array("socket.origins") {
            v.to_vec()
        } else if c.get("socket.origins").is_some() || c.contains_table("socket.origins") {
            return Err(invalid("socket.origins"));
        } else {
            Vec::new()
        };
        for origin in &origins {
            let uri = origin
                .parse::<http::Uri>()
                .map_err(|_| invalid("socket.origins"))?;
            if !matches!(uri.scheme_str(), Some("http" | "https"))
                || uri.authority().is_none()
                || uri.authority().is_some_and(|a| a.as_str().contains('@'))
                || uri.path_and_query().is_some_and(|p| p.as_str() != "/")
            {
                return Err(invalid("socket.origins"));
            }
        }
        let redis_url = c.get("socket.redis.url").map(str::to_owned);
        let namespace = c.get("socket.namespace").unwrap_or("default").to_owned();
        if namespace.is_empty()
            || (redis_url.is_some()
                && (c.get("socket.namespace").is_none() || !cfg!(feature = "redis")))
        {
            return Err(invalid("socket.redis"));
        }
        if let Some(url) = &redis_url {
            if !(url.starts_with("redis://") || url.starts_with("rediss://")) {
                return Err(invalid("socket.redis.url"));
            }
        }
        let certificate = c.resolve_path("socket.tls.certificate");
        let private_key = c.resolve_path("socket.tls.private_key");
        if certificate.is_some() != private_key.is_some()
            || ((certificate.is_some() || c.contains_table("socket.tls")) && !cfg!(feature = "tls"))
        {
            return Err(invalid("socket.tls"));
        }
        if furnace_rs_core::__private::configuration::present(c, "socket.tls")
            && certificate.is_none()
        {
            return Err(invalid("socket.tls"));
        }
        let lease_refresh = duration("socket.lease_refresh_ms", 5_000)?;
        let lease_expiry = duration("socket.lease_expiry_ms", 15_000)?;
        if lease_refresh >= lease_expiry {
            return Err(invalid("socket.lease_refresh_ms"));
        }
        Ok(Self {
            address,
            max_message_bytes: number("socket.max_message_bytes", 1_048_576)?,
            queue_messages: number("socket.queue_messages", 256)?,
            queue_bytes: number("socket.queue_bytes", 4_194_304)?,
            heartbeat_interval: duration("socket.heartbeat_interval_ms", 30_000)?,
            pong_timeout: duration("socket.pong_timeout_ms", 10_000)?,
            shutdown_timeout: duration("socket.shutdown_timeout_ms", 10_000)?,
            lease_refresh,
            lease_expiry,
            origins,
            namespace,
            redis_url,
            certificate,
            private_key,
        })
    }
}
/// Creates a fixed diagnostic without including configuration values.
pub(crate) fn invalid(key: &str) -> Error {
    Error::new(
        Diagnostic::new(
            DiagnosticCode::new("FURNACE160"),
            "invalid socket configuration",
            "socket settings must have valid types, limits, and enabled optional features",
        )
        .with_subject(key),
    )
}
