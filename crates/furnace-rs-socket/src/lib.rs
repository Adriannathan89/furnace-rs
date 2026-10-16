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
