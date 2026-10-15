//! Conventional configuration loading for the HTTP runtime.

use std::any::TypeId;
use std::fmt;
use std::net::IpAddr;
use std::path::Path;

use furnace_rs_core::__private::{
    AutoConfigurationApplyContext, AutoConfigurationContext, AutoConfigurationContribution,
    AutoConfigurationDescriptor, AutoConfigurationEvaluation,
};
use furnace_rs_core::{
    AutoConfigurationConfigEvidence, AutoConfigurationReasonCode, Config, ConfigBuilder,
    Diagnostic, DotenvSource, EnvSource, Error, FURNACE020, FurnaceBuilder, Result, SourceLocation,
    TomlSource,
};

use crate::http_scope::HttpApplicationScope;

pub(crate) const SERVER_AUTO_CONFIGURATION_ID: &str = "furnace.common.http.server";

const SERVER_BINDING_TYPE_NAME: &str = "furnace_rs_common::server_config::ServerBinding";

#[derive(Clone, Copy)]
pub(crate) enum HttpRuntimeMode {
    Automatic,
}

#[derive(Clone)]
pub(crate) struct ServerBinding {
    host: String,
    port: u16,
}

impl ServerBinding {
    /// Reads and validates the automatic HTTP bind host and port from configuration.
    pub(crate) fn from_config(config: &Config) -> Result<Self> {
        let host = config.get("server.host").unwrap_or("127.0.0.1");
        let host = parse_host(host, config.source_of("server.host"))?;
        let port = config.get("server.port").unwrap_or("3000");
        let port = parse_port(port, config.source_of("server.port"))?;

        Ok(Self { host, port })
    }

    /// Returns the validated host name or IP address used for binding.
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    /// Returns the validated nonzero TCP port used for binding.
    pub(crate) const fn port(&self) -> u16 {
        self.port
    }

    /// Returns the host and port as a bind-address pair.
    pub(crate) fn address(&self) -> (&str, u16) {
        (self.host(), self.port())
    }
}

impl fmt::Debug for ServerBinding {
    /// Formats the binding without revealing configured network details.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

/// Enables the conventional automatic HTTP listener during application building.
pub(crate) fn enable_automatic_server(builder: &mut FurnaceBuilder) -> bool {
    builder.__auto_configuration_input(SERVER_AUTO_CONFIGURATION_ID, HttpRuntimeMode::Automatic)
}

#[allow(dead_code)]
/// Loads `.env`, `furnace.toml`, and the conventional `FURNACE_` environment overrides.
pub(crate) fn load_standard_config_from(root: &Path) -> Result<Config> {
    load_standard_config_from_with_environment(root, EnvSource::new("FURNACE_"))
}

/// Loads conventional configuration sources with an explicitly supplied environment source.
pub(crate) fn load_standard_config_from_with_environment(
    root: &Path,
    environment: EnvSource,
) -> Result<Config> {
    ConfigBuilder::new()
        .dotenv(DotenvSource::optional(root.join(".env")))
        .source(TomlSource::optional(root.join("furnace.toml")))
        .source(environment)
        .build()
}

/// Supplies the type identity used to register automatic HTTP binding configuration.
fn server_binding_type_id() -> TypeId {
    TypeId::of::<ServerBinding>()
}

/// Decides whether automatic HTTP binding applies and reports its configuration evidence.
fn evaluate(context: &AutoConfigurationContext<'_>) -> AutoConfigurationEvaluation {
    if context.input::<HttpRuntimeMode>().is_none() {
        return AutoConfigurationEvaluation::overridden(
            AutoConfigurationReasonCode::new("explicit_listener"),
            "the low-level runtime owns its listener address",
            Vec::new(),
            server_evidence(context),
        );
    }

    let Some(cauldron_graph) = context.cauldron_graph() else {
        return AutoConfigurationEvaluation::skipped(
            AutoConfigurationReasonCode::new("no_managed_routes"),
            "the selected application has no managed HTTP routes",
            Vec::new(),
            server_evidence(context),
        );
    };

    let http = match HttpApplicationScope::for_cauldron_graph(Some(cauldron_graph)) {
        Ok(http) => http,
        Err(error) => {
            return AutoConfigurationEvaluation::failed(
                AutoConfigurationReasonCode::new("http_scope_failed"),
                "the selected application HTTP scope could not be analyzed",
                Vec::new(),
                server_evidence(context),
                error,
            );
        }
    };
    if !http.has_routes() {
        return AutoConfigurationEvaluation::skipped(
            AutoConfigurationReasonCode::new("no_managed_routes"),
            "the selected application has no managed HTTP routes",
            Vec::new(),
            server_evidence(context),
        );
    }

    match ServerBinding::from_config(context.config()) {
        Ok(_) => AutoConfigurationEvaluation::active(
            AutoConfigurationReasonCode::new("conditions_matched"),
            "automatic HTTP server conditions matched",
            Vec::new(),
            server_evidence(context),
        ),
        Err(error) => AutoConfigurationEvaluation::failed(
            AutoConfigurationReasonCode::new("invalid_configuration"),
            "automatic HTTP server configuration is invalid",
            Vec::new(),
            server_evidence(context),
            error,
        ),
    }
}

/// Contributes the validated HTTP binding after automatic-configuration evaluation succeeds.
fn apply(context: &AutoConfigurationApplyContext<'_>) -> Result<AutoConfigurationContribution> {
    Ok(AutoConfigurationContribution::new(
        ServerBinding::from_config(context.config())?,
    ))
}

/// Validates and normalizes an IPv4 address, IPv6 address, or DNS host name.
fn parse_host(host: &str, source: Option<&str>) -> Result<String> {
    if host.trim().is_empty() {
        return Err(invalid_server_configuration(
            "server.host",
            source,
            "must not be empty or whitespace-only",
        ));
    }
    if host.chars().any(|character| character.is_ascii_control()) {
        return Err(invalid_server_configuration(
            "server.host",
            source,
            "must not contain ASCII control characters",
        ));
    }

    if let Ok(address) = host.parse::<IpAddr>() {
        return Ok(address.to_string());
    }
    if !is_valid_hostname(host) {
        return Err(invalid_server_configuration(
            "server.host",
            source,
            "must be an IPv4 address, IPv6 address, or valid hostname",
        ));
    }

    Ok(host.to_owned())
}

/// Checks the DNS label rules accepted for an automatic server host.
pub(crate) fn is_valid_hostname(host: &str) -> bool {
    let hostname = host.strip_suffix('.').unwrap_or(host);
    !hostname.is_empty()
        && hostname.len() <= 253
        && hostname.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|character| character.is_ascii_alphanumeric() || character == b'-')
        })
}

/// Parses a nonzero TCP port and attaches source context to invalid values.
fn parse_port(port: &str, source: Option<&str>) -> Result<u16> {
    let Ok(port) = port.parse::<u16>() else {
        return Err(invalid_server_configuration(
            "server.port",
            source,
            "must be an integer in the range 1..=65535",
        ));
    };
    if port == 0 {
        return Err(invalid_server_configuration(
            "server.port",
            source,
            "must be an integer in the range 1..=65535",
        ));
    }

    Ok(port)
}

/// Builds a configuration diagnostic while limiting the source label to safe values.
fn invalid_server_configuration(key: &'static str, source: Option<&str>, rule: &str) -> Error {
    Error::new(
        Diagnostic::new(
            FURNACE020,
            "automatic HTTP server configuration is invalid",
            format!("{key} {rule}; source: {}", safe_source_label(source)),
        )
        .with_subject(key)
        .with_suggestion("configure a valid automatic HTTP server address"),
    )
}

/// Maps arbitrary configuration source paths to non-sensitive diagnostic labels.
fn safe_source_label(source: Option<&str>) -> &'static str {
    match source {
        None => "defaults",
        Some("defaults") => "defaults",
        Some("environment") => "environment",
        Some("furnace.toml") => "furnace.toml",
        Some("test") => "test",
        Some(source)
            if Path::new(source)
                .file_name()
                .is_some_and(|name| name == "furnace.toml") =>
        {
            "furnace.toml"
        }
        Some(_) => "[REDACTED]",
    }
}

/// Collects source metadata for the automatic HTTP host and port settings.
fn server_evidence(context: &AutoConfigurationContext<'_>) -> Vec<AutoConfigurationConfigEvidence> {
    ["server.host", "server.port"]
        .into_iter()
        .map(|key| AutoConfigurationConfigEvidence::new(key, context.config().source_of(key)))
        .collect()
}

furnace_rs_core::__private::inventory::submit! {
    AutoConfigurationDescriptor::new(
        SERVER_AUTO_CONFIGURATION_ID,
        SERVER_BINDING_TYPE_NAME,
        server_binding_type_id,
        SourceLocation::new(file!(), line!(), column!()),
        evaluate,
        apply,
    )
}
