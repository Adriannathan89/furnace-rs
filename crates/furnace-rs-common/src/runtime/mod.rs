//! Shared transport-neutral application runtime coordination.
mod tasks;
#[doc(hidden)]
pub use tasks::RuntimeTasks;
mod error;
pub use error::ApplicationRuntimeError;
mod config;
pub use config::RuntimeSelection;
mod coordinator;
#[doc(hidden)]
pub use coordinator::{
    PreparedTransport, TransportServe, TransportTransition, serve_prepared_transports,
};
mod routes;
#[doc(hidden)]
pub use routes::RuntimeTransportDescriptor;
pub use routes::{RuntimeBindings, RuntimeRoutes, serve_transports};
mod burn;
pub use burn::FurnaceBurnExt;
#[doc(hidden)]
pub use burn::{load_standard_config, prepare_application};
#[doc(hidden)]
pub use tasks::RuntimeExecutor;
#[cfg(feature = "http")]
mod http;

#[doc(hidden)]
pub use coordinator::serve_prepared_transports_with;
