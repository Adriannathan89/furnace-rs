//! Shared startup and explicit composition for a separate socket listener.
mod adapter;
mod server;
use crate::SocketServerConfig;
#[doc(hidden)]
pub use adapter::owned_route;
pub use adapter::{RuntimeSocketExt, build_socket_routes};
use furnace_rs_common::runtime::RuntimeTasks;
pub use furnace_rs_common::runtime::{
    ApplicationRuntimeError, FurnaceBurnExt, RuntimeBindings, RuntimeRoutes, RuntimeSelection,
    serve_transports,
};
use tokio::sync::watch;
#[derive(Clone)]
pub(crate) struct SocketRuntime {
    pub config: SocketServerConfig,
    pub shutdown: watch::Receiver<bool>,
    pub tasks: RuntimeTasks,
}
