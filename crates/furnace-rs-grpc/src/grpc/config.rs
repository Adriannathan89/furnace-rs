//! Source-aware pure gRPC configuration parsing; files are read only at bootstrap.
use super::FURNACE151;
use furnace_rs_core::{Config, Diagnostic, Error, Result};
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::{Duration, Instant},
};

/// When a named remote client establishes its initial connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrpcConnectionMode {
    /// Establish the connection during infrastructure readiness.
    Eager,
    /// Establish the connection on first call.
    Lazy,
}

/// Source-relative paths for a server's certificate and private key.
#[derive(Clone)]
pub struct GrpcServerTlsConfig {
    certificate: PathBuf,
    private_key: PathBuf,
}

impl GrpcServerTlsConfig {
    /// Returns the resolved certificate path.
    pub fn certificate(&self) -> &std::path::Path {
        &self.certificate
    }

    /// Returns the resolved private-key path.
    pub fn private_key(&self) -> &std::path::Path {
        &self.private_key
    }
}

/// Verified client TLS options; custom roots can supplement or replace public roots.
#[derive(Clone)]
pub struct GrpcClientTlsConfig {
    custom_ca_files: Vec<PathBuf>,
    custom_ca_only: bool,
    domain_name: Option<String>,
}

impl GrpcClientTlsConfig {
    /// Returns resolved custom CA file paths.
    pub fn custom_ca_files(&self) -> &[PathBuf] {
        &self.custom_ca_files
    }

    /// Whether configured CAs are the only trusted roots.
    pub const fn custom_ca_only(&self) -> bool {
        self.custom_ca_only
    }

    /// Returns the explicit verified server name, if configured.
    pub fn domain_name(&self) -> Option<&str> {
        self.domain_name.as_deref()
    }
}

/// Parsed conventional gRPC listener settings.
#[derive(Clone)]
pub struct GrpcServerConfig {
    address: SocketAddr,
    max_encoding_message_bytes: usize,
    max_decoding_message_bytes: usize,
    tls_handshake_timeout: Duration,
    shutdown_timeout: Duration,
    tls: Option<GrpcServerTlsConfig>,
    reflection_enabled: bool,
}

impl GrpcServerConfig {
    /// Parses and validates listener settings without opening sockets or TLS files.
    #[allow(clippy::result_large_err)]
    pub fn parse(config: &Config) -> Result<Self> {
        for key in ["grpc", "grpc.server", "grpc.reflection"] {
            reject_scalar_or_array(config, key)?;
        }

        let host = scalar(config, "grpc.server.host", "127.0.0.1")?
            .parse::<IpAddr>()
            .map_err(|_| invalid(config, "grpc.server.host", "must be a literal IP address"))?;
        let port = scalar(config, "grpc.server.port", "50051")?
            .parse::<u16>()
            .ok()
            .filter(|p| *p != 0)
            .ok_or_else(|| invalid(config, "grpc.server.port", "must be in 1..=65535"))?;
        let tls = if present(config, "grpc.server.tls") {
            require_tls_feature(config, "grpc.server.tls")?;
            reject_scalar_or_array(config, "grpc.server.tls")?;

            Some(GrpcServerTlsConfig {
                certificate: path(config, "grpc.server.tls.certificate")?,
                private_key: path(config, "grpc.server.tls.private_key")?,
            })
        } else {
            None
        };

        Ok(Self {
            address: SocketAddr::new(host, port),
            max_encoding_message_bytes: size(config, "grpc.server.max_encoding_message_bytes")?,
            max_decoding_message_bytes: size(config, "grpc.server.max_decoding_message_bytes")?,
            tls_handshake_timeout: duration(
                config,
                "grpc.server.tls_handshake_timeout_ms",
                10_000,
            )?,
            shutdown_timeout: duration(config, "grpc.server.shutdown_timeout_ms", 30_000)?,
            tls,
            reflection_enabled: boolean(config, "grpc.reflection.enabled", false)?,
        })
    }

    /// Overrides the listener address for explicit runtime composition, allowing port zero.
    pub fn with_address(mut self, address: SocketAddr) -> Self {
        self.address = address;

        self
    }

    /// Returns the configured bind address.
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    /// Returns the maximum encoded response message size in bytes.
    pub const fn max_encoding_message_bytes(&self) -> usize {
        self.max_encoding_message_bytes
    }

    /// Returns the maximum decoded request message size in bytes.
    pub const fn max_decoding_message_bytes(&self) -> usize {
        self.max_decoding_message_bytes
    }

    /// Returns the bounded TLS handshake time.
    pub const fn tls_handshake_timeout(&self) -> Duration {
        self.tls_handshake_timeout
    }

    /// Returns the maximum graceful call-drain time.
    pub const fn shutdown_timeout(&self) -> Duration {
        self.shutdown_timeout
    }

    /// Returns the optional TLS identity paths.
    pub const fn tls(&self) -> Option<&GrpcServerTlsConfig> {
        self.tls.as_ref()
    }

    /// Whether guarded server reflection is explicitly enabled.
    pub const fn reflection_enabled(&self) -> bool {
        self.reflection_enabled
    }
}

/// Parsed settings for one named generated client.
#[derive(Clone)]
pub struct GrpcClientConfig {
    endpoint: tonic::transport::Endpoint,
    connection_mode: GrpcConnectionMode,
    connect_timeout: Duration,
    tls_handshake_timeout: Duration,
    request_timeout: Option<Duration>,
    max_encoding_message_bytes: usize,
    max_decoding_message_bytes: usize,
    tls: Option<GrpcClientTlsConfig>,
}

impl GrpcClientConfig {
    /// Parses one required endpoint and client options without DNS, connection, or file I/O.
    #[allow(clippy::result_large_err)]
    pub fn parse(config: &Config, name: &str) -> Result<Self> {
        reject_scalar_or_array(config, "grpc")?;
        reject_scalar_or_array(config, "grpc.clients")?;

        if !valid_client_name(name) {
            return Err(invalid(config, "grpc.clients", "invalid client name"));
        }

        let base = format!("grpc.clients.{name}");
        let key = |suffix: &str| format!("{base}.{suffix}");
        reject_scalar_or_array(config, &base)?;
        let endpoint_key = key("endpoint");
        let raw = scalar(config, &endpoint_key, "")?;
        let uri = raw
            .parse::<http::Uri>()
            .map_err(|_| invalid(config, &endpoint_key, "invalid endpoint"))?;

        if !matches!(uri.scheme_str(), Some("http" | "https"))
            || uri.authority().is_none_or(|a| a.as_str().contains('@'))
            || uri.host().is_none_or(str::is_empty)
        {
            return Err(invalid(
                config,
                &endpoint_key,
                "must be an HTTP or HTTPS service endpoint without user credentials",
            ));
        }

        let secure = uri.scheme_str() == Some("https");

        if secure {
            require_tls_feature(config, &endpoint_key)?;
        }

        let tls_key = key("tls");
        let tls = if present(config, &tls_key) {
            if !secure {
                return Err(invalid(config, &tls_key, "TLS requires an HTTPS endpoint"));
            }

            require_tls_feature(config, &tls_key)?;
            reject_scalar_or_array(config, &tls_key)?;
            let files_key = key("tls.custom_ca_files");
            let mut files = Vec::new();

            if present(config, &files_key) {
                let values = config
                    .get_string_array(&files_key)
                    .ok_or_else(|| invalid(config, &files_key, "must be a string array"))?;

                // Reuse core source-relative resolution for each authored array element.
                for (index, value) in values.iter().enumerate() {
                    if value.trim().is_empty() {
                        return Err(invalid(config, &files_key, "CA paths must not be empty"));
                    }

                    files.push(
                        config
                            .resolve_string_array_path(&files_key, index)
                            .ok_or_else(|| {
                                invalid(config, &files_key, "cannot resolve source-relative path")
                            })?,
                    );
                }
            }

            let only = boolean(config, &key("tls.custom_ca_only"), false)?;

            if only && files.is_empty() {
                return Err(invalid(
                    config,
                    &tls_key,
                    "custom-only trust requires configured CAs",
                ));
            }

            let domain_key = key("tls.domain_name");
            let domain_name = if present(config, &domain_key) {
                let value = scalar(config, &domain_key, "")?;

                if value.is_empty() || value.chars().any(char::is_control) {
                    return Err(invalid(config, &domain_key, "invalid server name"));
                }

                #[cfg(feature = "tls")]
                rustls::pki_types::ServerName::try_from(value.to_owned())
                    .map_err(|_| invalid(config, &domain_key, "invalid server name"))?;

                Some(value.to_owned())
            } else {
                None
            };

            Some(GrpcClientTlsConfig {
                custom_ca_files: files,
                custom_ca_only: only,
                domain_name,
            })
        } else {
            None
        };
        let endpoint = tonic::transport::Endpoint::from(uri);
        let connection_mode = match scalar(config, &key("connect"), "lazy")? {
            "lazy" => GrpcConnectionMode::Lazy,
            "eager" => GrpcConnectionMode::Eager,
            _ => return Err(invalid(config, &key("connect"), "must be lazy or eager")),
        };

        Ok(Self {
            endpoint,
            connection_mode,
            connect_timeout: duration(config, &key("connect_timeout_ms"), 10_000)?,
            tls_handshake_timeout: duration(config, &key("tls_handshake_timeout_ms"), 10_000)?,
            request_timeout: if present(config, &key("request_timeout_ms")) {
                Some(duration(config, &key("request_timeout_ms"), 0)?)
            } else {
                None
            },
            max_encoding_message_bytes: size(config, &key("max_encoding_message_bytes"))?,
            max_decoding_message_bytes: size(config, &key("max_decoding_message_bytes"))?,
            tls,
        })
    }

    /// Returns a clone of the parsed native endpoint; it is not connected.
    pub fn endpoint(&self) -> tonic::transport::Endpoint {
        self.endpoint.clone()
    }

    /// Returns whether connection readiness is eager or lazy.
    pub const fn connection_mode(&self) -> GrpcConnectionMode {
        self.connection_mode
    }

    /// Returns the connection deadline.
    pub const fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    /// Returns the TLS handshake deadline.
    pub const fn tls_handshake_timeout(&self) -> Duration {
        self.tls_handshake_timeout
    }

    /// Returns the optional default call-establishment deadline.
    pub const fn request_timeout(&self) -> Option<Duration> {
        self.request_timeout
    }

    /// Returns the maximum encoded request size in bytes.
    pub const fn max_encoding_message_bytes(&self) -> usize {
        self.max_encoding_message_bytes
    }

    /// Returns the maximum decoded response size in bytes.
    pub const fn max_decoding_message_bytes(&self) -> usize {
        self.max_decoding_message_bytes
    }

    /// Returns the optional verified TLS settings.
    pub const fn tls(&self) -> Option<&GrpcClientTlsConfig> {
        self.tls.as_ref()
    }
}

/// Checks the lowercase configuration grammar for a named client.
pub(crate) fn valid_client_name(name: &str) -> bool {
    name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

/// Checks whether a configuration path exists, including table paths.
fn present(config: &Config, key: &str) -> bool {
    furnace_rs_core::__private::configuration::present(config, key)
}

#[allow(clippy::result_large_err)]
/// Reads a scalar value or its default and rejects non-scalar values.
fn scalar<'a>(config: &'a Config, key: &str, default: &'a str) -> Result<&'a str> {
    if let Some(value) = config.get(key) {
        Ok(value)
    } else if present(config, key) {
        Err(invalid(config, key, "must be a scalar"))
    } else {
        Ok(default)
    }
}

#[allow(clippy::result_large_err)]
/// Requires the selected configuration path to be a table.
fn reject_scalar_or_array(config: &Config, key: &str) -> Result<()> {
    if config.get(key).is_some() || config.get_string_array(key).is_some() {
        Err(invalid(config, key, "must be a table"))
    } else {
        Ok(())
    }
}

#[allow(clippy::result_large_err)]
/// Resolves a required path relative to its configuration source.
fn path(config: &Config, key: &str) -> Result<PathBuf> {
    let value = scalar(config, key, "")?;

    if value.trim().is_empty() {
        return Err(invalid(config, key, "a path is required"));
    }

    config
        .resolve_path(key)
        .ok_or_else(|| invalid(config, key, "invalid path"))
}

#[allow(clippy::result_large_err)]
/// Parses a positive millisecond duration that fits a deadline.
fn duration(config: &Config, key: &str, default: u64) -> Result<Duration> {
    let value = if present(config, key) {
        scalar(config, key, "")?.parse::<u64>().ok()
    } else {
        Some(default)
    };
    let value = value
        .filter(|ms| *ms > 0)
        .map(Duration::from_millis)
        .filter(|d| Instant::now().checked_add(*d).is_some());

    value.ok_or_else(|| {
        invalid(
            config,
            key,
            "must be a positive duration fitting a deadline",
        )
    })
}

#[allow(clippy::result_large_err)]
/// Parses a positive message-size limit supported by the transport.
fn size(config: &Config, key: &str) -> Result<usize> {
    scalar(config, key, "4194304")?
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0 && *n <= u32::MAX as usize)
        .ok_or_else(|| invalid(config, key, "must be a positive supported message size"))
}

#[allow(clippy::result_large_err)]
/// Parses a boolean setting or returns its configured default.
fn boolean(config: &Config, key: &str, default: bool) -> Result<bool> {
    scalar(config, key, if default { "true" } else { "false" })?
        .parse()
        .map_err(|_| invalid(config, key, "must be a boolean"))
}

#[allow(clippy::result_large_err)]
/// Rejects TLS settings when the crate TLS feature is disabled.
fn require_tls_feature(config: &Config, key: &str) -> Result<()> {
    if cfg!(feature = "tls") {
        Ok(())
    } else {
        Err(invalid(
            config,
            key,
            "requires the tls feature (facade grpc-tls)",
        ))
    }
}

/// Builds a source-aware diagnostic for an invalid gRPC setting.
fn invalid(config: &Config, key: &str, rule: &str) -> Error {
    let source = config
        .source_of(key)
        .or_else(|| config.source_of_string_array(key))
        .or_else(|| config.table_source(key));
    let source = match source {
        None => "defaults",
        Some("test") => "test",
        Some("environment") => "environment",
        Some(_) => "configuration",
    };

    Error::new(
        Diagnostic::new(
            FURNACE151,
            "invalid gRPC configuration",
            format!("{rule}; source: {source}"),
        )
        .with_subject(key),
    )
}

macro_rules! redacted_debug { ($($ty:ty),+) => { $(impl fmt::Debug for $ty { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("[REDACTED]") } })+ }; }

redacted_debug!(
    GrpcServerConfig,
    GrpcClientConfig,
    GrpcServerTlsConfig,
    GrpcClientTlsConfig
);
