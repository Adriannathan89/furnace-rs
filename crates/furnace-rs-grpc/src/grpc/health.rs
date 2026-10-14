//! Public native health and optional guarded reflection installation.
use super::{GrpcApplicationScope, reflection};
use furnace_rs_core::{Furnace, Result};
/// Shared health state transitioned by the runtime before admission and shutdown.
#[derive(Clone, Debug)]
pub struct GrpcHealth {
    reporter: tonic_health::server::HealthReporter,
    services: Vec<String>,
}
impl GrpcHealth {
    /// Publishes aggregate and selected service readiness to Check and Watch callers.
    pub async fn set_serving(&mut self, serving: bool) {
        let status = if serving {
            tonic_health::ServingStatus::Serving
        } else {
            tonic_health::ServingStatus::NotServing
        };
        self.reporter.set_service_status("", status).await;
        for service in &self.services {
            self.reporter.set_service_status(service, status).await;
        }
    }
}
/// Installs native infrastructure; the runtime initializes health before opening admission.
#[allow(clippy::result_large_err)]
pub fn install_grpc_infrastructure(
    app: &Furnace,
    scope: &GrpcApplicationScope,
    mut routes: tonic::service::Routes,
) -> Result<(tonic::service::Routes, GrpcHealth)> {
    let (reporter, server) = tonic_health::server::health_reporter();
    routes = routes.add_service(server);
    let mut services: Vec<_> = scope
        .services()
        .iter()
        .map(|service| service.name().to_owned())
        .collect();
    services.push(reflection::HEALTH.to_owned());
    if reflection::selected_policy(scope, app.context().config())?.is_some() {
        #[cfg(feature = "jwt")]
        {
            let fds = reflection::reflection_descriptors(scope)?;
            let first = tonic_reflection::server::Builder::configure()
                .include_reflection_service(false)
                .register_file_descriptor_set(fds.clone())
                .build_v1()
                .map_err(|_| reflection_build_error())?;
            let second = tonic_reflection::server::Builder::configure()
                .include_reflection_service(false)
                .register_file_descriptor_set(fds)
                .build_v1alpha()
                .map_err(|_| reflection_build_error())?;
            routes = routes.add_service(super::GrpcPassportService::new(
                first,
                app.context().clone(),
                reflection::bindings(scope, app, reflection::NAMES[0])?,
            ));
            routes = routes.add_service(super::GrpcPassportService::new(
                second,
                app.context().clone(),
                reflection::bindings(scope, app, reflection::NAMES[1])?,
            ));
            services.extend(reflection::NAMES.into_iter().map(str::to_owned));
        }
    }
    Ok((routes, GrpcHealth { reporter, services }))
}
#[cfg(feature = "jwt")]
fn reflection_build_error() -> furnace_rs_core::Error {
    furnace_rs_core::Error::new(furnace_rs_core::Diagnostic::new(
        super::FURNACE150,
        "invalid reflection descriptors",
        "selected protobuf metadata could not build native reflection services",
    ))
}
