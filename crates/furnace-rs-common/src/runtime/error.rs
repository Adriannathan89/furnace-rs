//! Shared lifecycle and transport failures.
use std::{error::Error as StdError, fmt};
/// An error produced while preparing, running, or stopping the HTTP runtime.
#[derive(Debug)]
#[non_exhaustive]
pub enum ApplicationRuntimeError {
    /// Route validation or final router configuration failed before lifecycle startup.
    Bootstrap(furnace_rs_core::Error),
    /// Application lifecycle startup or shutdown failed.
    Lifecycle(furnace_rs_core::Error),
    /// The HTTP listener could not bind to the requested address.
    Bind(std::io::Error),
    /// Axum stopped because of an HTTP serving failure.
    Serve(std::io::Error),
    /// A selected transport listener could not bind.
    TransportBind {
        /// Protocol owning the listener.
        transport: &'static str,
        /// Original socket error.
        source: std::io::Error,
    },
    /// A selected transport stopped serving unexpectedly.
    TransportServe {
        /// Protocol owning the server.
        transport: &'static str,
        /// Original transport failure.
        source: Box<dyn StdError + Send + Sync>,
    },
    /// Runtime-owned calls exceeded the configured graceful drain deadline.
    DrainTimeout,
    /// An operational failure occurred and the subsequent shutdown also failed.
    OperationAndShutdown {
        /// The bind or serving error that initiated shutdown.
        operation: Box<ApplicationRuntimeError>,
        /// The lifecycle error returned while attempting shutdown.
        shutdown: furnace_rs_core::Error,
    },
}

impl fmt::Display for ApplicationRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bootstrap(error) => write!(formatter, "HTTP bootstrap failed: {error}"),
            Self::Lifecycle(error) => write!(formatter, "application lifecycle failed: {error}"),
            Self::Bind(error) => write!(formatter, "HTTP listener bind failed: {error}"),
            Self::Serve(error) => write!(formatter, "HTTP serving failed: {error}"),
            Self::TransportBind { transport, source } => {
                write!(formatter, "{transport} listener bind failed: {source}")
            }
            Self::TransportServe { transport, source } => {
                write!(formatter, "{transport} serving failed: {source}")
            }
            Self::DrainTimeout => formatter.write_str("application transport drain timed out"),
            Self::OperationAndShutdown {
                operation,
                shutdown,
            } => write!(
                formatter,
                "{operation}; application shutdown also failed: {shutdown}"
            ),
        }
    }
}

impl StdError for ApplicationRuntimeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Bootstrap(error) | Self::Lifecycle(error) => Some(error),
            Self::Bind(error) | Self::Serve(error) => Some(error),
            Self::OperationAndShutdown { operation, .. } => Some(operation.as_ref()),
            Self::TransportBind { source, .. } => Some(source),
            Self::TransportServe { source, .. } => Some(source.as_ref()),
            Self::DrainTimeout => None,
        }
    }
}
