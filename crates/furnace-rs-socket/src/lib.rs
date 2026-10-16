//! Standard WebSocket server endpoints, managed gateways, and distributed rooms.
#![forbid(unsafe_code)]
/// Native WebSocket messages and owned socket.
pub use axum::extract::ws::{Message, WebSocket};
/// Neutral runtime and authentication contracts.
#[doc(hidden)]
pub use furnace_rs_common as common;
/// Shared dependency injection contracts.
pub use furnace_rs_core as core;
mod config;
mod error;
mod protocol;
pub use config::*;
pub use error::*;
pub use protocol::*;
mod descriptor;
mod scope;
pub use descriptor::*;
pub use scope::*;

mod context;
pub use context::*;
/// Separate-listener runtime integration.
pub mod runtime;
pub use runtime::build_socket_routes;
/// Macro expansion implementation contracts.
#[doc(hidden)]
pub mod __private {
    pub use crate::descriptor::SocketGuard;
    pub use crate::dispatch::{SocketGateway, SocketInput};
    pub use crate::runtime::{gateway_route, owned_route};
    pub use axum::routing::MethodRouter;
    pub use futures_util::future::BoxFuture;
}
/// Gateway and owned-socket registration attributes.
pub use furnace_rs_socket_macros::{socket_endpoint, socket_gateway};

mod hub;
pub use hub::{SocketGatewayHub, SocketHub};
mod dispatch;
