//! Protocol-neutral manual composition and linked transport adapters.
use super::{ApplicationRuntimeError, PreparedTransport};
use furnace_rs_core::{CauldronGraph, Config, Furnace, FurnaceBuilder};
use futures_util::future::BoxFuture;
use std::{any::Any, collections::BTreeMap};
/// Raw native routes to be finalized before lifecycle startup.
#[derive(Default)]
pub struct RuntimeRoutes(BTreeMap<&'static str, Box<dyn Any + Send>>);
impl RuntimeRoutes {
    /// Starts an empty listener composition.
    pub fn new() -> Self {
        Self::default()
    }
    /// Installs an unconfigured native HTTP router.
    #[cfg(feature = "http")]
    pub fn with_http(mut self, router: axum::Router) -> Self {
        self.insert("http", router);
        self
    }
    /// Installs an integration-owned native route set.
    #[doc(hidden)]
    pub fn insert<T: Any + Send>(&mut self, name: &'static str, value: T) {
        self.0.insert(name, Box::new(value));
    }
    /// Removes a typed native route set for its owning adapter.
    #[doc(hidden)]
    pub fn take<T: Any>(&mut self, name: &str) -> Option<T> {
        self.0.remove(name)?.downcast().ok().map(|v| *v)
    }
}
/// Explicit listener settings for manually composed transports.
#[derive(Default)]
pub struct RuntimeBindings(BTreeMap<&'static str, Box<dyn Any + Send>>);
impl RuntimeBindings {
    /// Starts an empty listener configuration.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the explicit HTTP address, including port zero.
    #[cfg(feature = "http")]
    pub fn with_http(mut self, address: std::net::SocketAddr) -> Self {
        self.insert("http", address);
        self
    }
    /// Installs an integration-owned listener configuration.
    #[doc(hidden)]
    pub fn insert<T: Any + Send>(&mut self, name: &'static str, value: T) {
        self.0.insert(name, Box::new(value));
    }
    /// Removes a typed configuration for its owning adapter.
    #[doc(hidden)]
    pub fn take<T: Any>(&mut self, name: &str) -> Option<T> {
        self.0.remove(name)?.downcast().ok().map(|v| *v)
    }
}
/// Linked transport adapter without native protocol types in the coordinator.
#[doc(hidden)]
pub struct RuntimeTransportDescriptor {
    /// Protocol key accepted in furnace.transports.
    pub name: &'static str,
    /// Side-effect-free listener and selected managed-operation validation.
    pub validate: fn(&CauldronGraph, &Config) -> furnace_rs_core::Result<()>,
    /// Registers selected automatic infrastructure before construction.
    pub configure: fn(&mut FurnaceBuilder),
    /// Finalizes raw routes and prepares native server state before startup.
    pub prepare: RuntimePrepare,
}
furnace_rs_core::__private::inventory::collect!(RuntimeTransportDescriptor);
/// Runs explicit native route composition with one lifecycle owner.
pub async fn serve_transports(
    application: Furnace,
    mut routes: RuntimeRoutes,
    mut bindings: RuntimeBindings,
    shutdown: impl std::future::Future<Output = ()> + Send,
) -> Result<(), ApplicationRuntimeError> {
    let mut prepared = Vec::new();
    for adapter in furnace_rs_core::__private::inventory::iter::<RuntimeTransportDescriptor> {
        if routes.0.contains_key(adapter.name) {
            if !bindings.0.contains_key(adapter.name) {
                return Err(invalid_composition());
            }
            prepared.push(
                (adapter.prepare)(&application, Some(&mut routes), Some(&mut bindings)).await?,
            );
        }
    }
    if !routes.0.is_empty() || !bindings.0.is_empty() {
        return Err(invalid_composition());
    }
    super::serve_prepared_transports(application, prepared, shutdown).await
}
fn invalid_composition() -> ApplicationRuntimeError {
    ApplicationRuntimeError::Bootstrap(furnace_rs_core::Error::new(
        furnace_rs_core::Diagnostic::new(
            furnace_rs_core::DiagnosticCode::new("FURNACE151"),
            "invalid transport composition",
            "each raw route set requires one binding and a linked transport adapter",
        ),
    ))
}

/// Native server preparation callback without transport type dependencies.
#[doc(hidden)]
pub type RuntimePrepare =
    for<'a> fn(
        &'a Furnace,
        Option<&'a mut RuntimeRoutes>,
        Option<&'a mut RuntimeBindings>,
    ) -> BoxFuture<'a, Result<PreparedTransport, ApplicationRuntimeError>>;
