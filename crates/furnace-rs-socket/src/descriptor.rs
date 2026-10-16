//! Immutable link-time WebSocket metadata.
use crate::{EndpointMode, ProcessingMode};
use std::any::TypeId;
/// One declared gateway operation.
#[derive(Clone, Copy, Debug)]
pub struct SocketHandlerDescriptor {
    /// Wire event or Rust lifecycle method name.
    pub name: &'static str,
    /// One of event, text, binary, connect, disconnect, join, owned.
    pub kind: &'static str,
}
/// Static registration of one gateway or owned endpoint.
pub struct SocketEndpointDescriptor {
    /// DI output identity.
    pub provider: fn() -> TypeId,
    /// Implementation name.
    pub implementation: &'static str,
    /// WebSocket upgrade path.
    pub path: &'static str,
    /// Endpoint protocol.
    pub mode: EndpointMode,
    /// Per-connection scheduling.
    pub processing: ProcessingMode,
    /// Static operation metadata.
    pub handlers: &'static [SocketHandlerDescriptor],
}
impl SocketEndpointDescriptor {
    /// Constructs side-effect-free registration metadata.
    pub const fn new(provider: fn() -> TypeId, implementation: &'static str, path: &'static str, mode: EndpointMode, processing: ProcessingMode, handlers: &'static [SocketHandlerDescriptor]) -> Self {
        Self { provider, implementation, path, mode, processing, handlers }
    }
}
/// Link-time descriptor entry.
#[doc(hidden)]
pub struct SocketEndpointRegistration {
    /// Selected endpoint metadata.
    pub descriptor: &'static SocketEndpointDescriptor,
}
furnace_rs_core::__private::inventory::collect!(SocketEndpointRegistration);
