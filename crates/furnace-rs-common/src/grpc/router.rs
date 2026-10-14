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
