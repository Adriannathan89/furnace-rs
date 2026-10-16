//! Shared startup and explicit composition for a separate socket listener.
mod adapter;
mod server;
use crate::SocketServerConfig;
pub use adapter::{RuntimeSocketExt, build_socket_routes};
#[doc(hidden)]
pub use adapter::{gateway_route, owned_route};
use furnace_rs_common::runtime::RuntimeTasks;
pub use furnace_rs_common::runtime::{
    ApplicationRuntimeError, FurnaceBurnExt, RuntimeBindings, RuntimeRoutes, RuntimeSelection,
    serve_transports,
};
use tokio::sync::watch;
#[derive(Clone)]
pub(crate) struct SocketRuntime {
    pub hub: crate::SocketHub,
    pub config: SocketServerConfig,
    pub shutdown: watch::Receiver<bool>,
    pub tasks: RuntimeTasks,
}
