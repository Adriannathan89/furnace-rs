//! Side-effect-free conventional transport selection.
use furnace_rs_core::{Config, Diagnostic, DiagnosticCode, Error, Result};

/// The listeners selected for a conventional application run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeSelection {
    /// Serve the existing HTTP application.
    pub http: bool,
    /// Serve protobuf-defined gRPC services.
    pub grpc: bool,
}

impl RuntimeSelection {
    /// Parses the configured set and rejects transports unavailable in this build.
    #[allow(clippy::result_large_err)]
    pub fn parse(config: &Config, compiled_http: bool, compiled_grpc: bool) -> Result<Self> {
        let invalid = || {
            Error::new(
                Diagnostic::new(
                    DiagnosticCode::new("FURNACE151"),
                    "invalid transport selection",
                    "furnace.transports must select unique compiled http/grpc transports",
                )
                .with_subject("furnace.transports"),
            )
        };
        let mut selected = Self {
            http: false,
            grpc: false,
        };
        if let Some(values) = config.get_string_array("furnace.transports") {
            if values.is_empty() {
                return Err(invalid());
            }
            for value in values {
                let enabled = match value.as_str() {
                    "http" if compiled_http => &mut selected.http,
                    "grpc" if compiled_grpc => &mut selected.grpc,
                    _ => return Err(invalid()),
                };
                if std::mem::replace(enabled, true) {
                    return Err(invalid());
                }
            }
        } else {
            if furnace_rs_core::__private::configuration::present(config, "furnace.transports")
                || !compiled_http
            {
                return Err(invalid());
            }
            selected.http = true;
        }
        Ok(selected)
    }
}
