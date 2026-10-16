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
    /// Source of this declaration.
    pub location: furnace_rs_core::SourceLocation,
    /// Managed output identity.
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
    /// Typed route constructor; absent metadata cannot be served.
    pub route: Option<
        fn(
            furnace_rs_core::ApplicationContext,
        ) -> furnace_rs_core::Result<axum::routing::MethodRouter>,
    >,
    /// Static handler declarations.
    /// Optional handshake guard.
    pub guard: Option<SocketGuard>,
    /// Authentication cookie name.
    pub cookie: &'static str,
    /// Authentication query parameter name.
    pub query: &'static str,
    /// Static gateway operation metadata.
    pub handlers: &'static [SocketHandlerDescriptor],
}
impl SocketEndpointDescriptor {
    /// Records the authored registration location.
    pub const fn with_location(mut self, location: furnace_rs_core::SourceLocation) -> Self {
        self.location = location;
        self
    }

    /// Selects cookie and query credential names.
    pub const fn with_credentials(mut self, cookie: &'static str, query: &'static str) -> Self {
        self.cookie = cookie;
        self.query = query;
        self
    }
    /// Applies a handshake policy.
    pub const fn with_guard(mut self, guard: SocketGuard) -> Self {
        self.guard = Some(guard);
        self
    }

    /// Attaches a native route factory.
    pub const fn with_route(
        mut self,
        route: fn(
            furnace_rs_core::ApplicationContext,
        ) -> furnace_rs_core::Result<axum::routing::MethodRouter>,
    ) -> Self {
        self.route = Some(route);
        self
    }
    /// Constructs side-effect-free registration metadata.
    pub const fn new(
        provider: fn() -> TypeId,
        implementation: &'static str,
        path: &'static str,
        mode: EndpointMode,
        processing: ProcessingMode,
        handlers: &'static [SocketHandlerDescriptor],
    ) -> Self {
        Self {
            location: furnace_rs_core::SourceLocation::new("<unknown>", 0, 0),
            provider,
            implementation,
            path,
            mode,
            processing,
            handlers,
            route: None,
            guard: None,
            cookie: "access_token",
            query: "access_token",
        }
    }
}
/// Link-time descriptor entry.
#[doc(hidden)]
pub struct SocketEndpointRegistration {
    /// Selected endpoint metadata.
    pub descriptor: &'static SocketEndpointDescriptor,
}
furnace_rs_core::__private::inventory::collect!(SocketEndpointRegistration);
/// Optional typed Passport policy bound at handshake.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct SocketGuard {
    #[cfg(feature = "jwt")]
    pub(crate) descriptor: fn() -> &'static furnace_rs_common::GuardDescriptor,
}
#[cfg(feature = "jwt")]
impl SocketGuard {
    /// Captures a typed guard without resolving its strategy.
    pub const fn of<G: furnace_rs_common::GuardPolicy>() -> Self {
        Self {
            descriptor: G::descriptor,
        }
    }
}
