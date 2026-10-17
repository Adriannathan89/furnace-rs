//! Resolves managed service handles once and builds native Tonic routes.
use super::{FURNACE150, GrpcApplicationScope, GrpcServerConfig};
use furnace_rs_core::{Diagnostic, Error, Furnace, Result};
use tonic::service::Routes;

/// Bootstrap inputs retained only while typed service registrars run.
#[doc(hidden)]
pub struct GrpcRegistrationContext<'a> {
    /// Application owning each resolved implementation.
    pub application: &'a Furnace,
    /// Parsed message limits and server settings.
    pub config: &'a GrpcServerConfig,
    /// Validated, root-selected protobuf metadata.
    pub scope: &'a GrpcApplicationScope,
}
/// Typed registrar function generated for one concrete service implementation.
#[doc(hidden)]
pub type GrpcServiceRegistrar = fn(&GrpcRegistrationContext<'_>, Routes) -> Result<Routes>;

/// Builds raw protobuf routes; callers may compose additional native Tonic services.
#[allow(clippy::result_large_err)]
pub fn build_grpc_routes(application: &Furnace) -> Result<Routes> {
    let scope = GrpcApplicationScope::for_application(application)?;
    let config = GrpcServerConfig::parse(application.context().config())?;
    let context = GrpcRegistrationContext {
        application,
        config: &config,
        scope: &scope,
    };
    let mut routes = Routes::default();
    for service in scope.services() {
        let registrar = service.descriptor().registrar().ok_or_else(|| {
            Error::new(
                Diagnostic::new(
                    FURNACE150,
                    "missing gRPC service registrar",
                    "selected protobuf metadata has no typed server adapter",
                )
                .with_subject(service.name())
                .with_location(service.descriptor().location()),
            )
        })?;
        routes = registrar(&context, routes)?;
    }
    Ok(routes)
}

impl GrpcRegistrationContext<'_> {
    /// Installs a generated server with its already validated method policies.
    #[doc(hidden)]
    #[allow(clippy::result_large_err)]
    pub fn add_service<T: super::GrpcService, S>(&self, server: S, routes: Routes) -> Result<Routes>
    where
        S: tower::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<tonic::body::Body>,
                Error = std::convert::Infallible,
            > + tonic::server::NamedService
            + Clone
            + Send
            + Sync
            + 'static,
        S::Future: Send + 'static,
    {
        #[cfg(feature = "jwt")]
        {
            use crate::common::passport::__private::GuardOccurrence;
            let guards = self.scope.guards();
            let preflight = crate::PassportStrategyCatalog::preflight_scoped(
                self.application.cauldron_graph(),
                &guards,
            )?;
            let selected = self
                .scope
                .services()
                .iter()
                .find(|service| {
                    service.descriptor().provider_type_id() == std::any::TypeId::of::<T>()
                })
                .expect("registrar is selected from scope");
            let mut bindings = std::collections::BTreeMap::new();
            for method in selected.methods() {
                let binding = if method.guard.is_some() {
                    let occurrence = GuardOccurrence {
                        provider: std::any::TypeId::of::<T>(),
                        operation: method.path.clone(),
                    };
                    let binding = preflight
                        .binding_for_occurrence(&occurrence, selected.owner())
                        .expect("selected guarded method has a preflight binding");
                    Some(binding.clone())
                } else {
                    None
                };
                bindings.insert(method.path.clone(), binding);
            }
            Ok(routes.add_service(super::GrpcPassportService::new(
                server,
                self.application.context().clone(),
                bindings,
            )))
        }
        #[cfg(not(feature = "jwt"))]
        Ok(routes.add_service(server))
    }
}
