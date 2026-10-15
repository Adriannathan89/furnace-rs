//! Standalone transport's neutral runtime registration.
use crate::{GrpcApplicationScope, GrpcServerConfig};
use furnace_rs_common::runtime::*;
use furnace_rs_core::{CauldronGraph, Config, Furnace, FurnaceBuilder};
use futures_util::future::BoxFuture;
/// Adds native gRPC values to shared runtime composition builders.
pub trait RuntimeGrpcExt: Sized {
    /// Native route set or listener configuration accepted by this builder.
    type Grpc;
    /// Selects this builder's gRPC routes or listener configuration.
    fn with_grpc(self, grpc: Self::Grpc) -> Self;
}
impl RuntimeGrpcExt for RuntimeRoutes {
    type Grpc = tonic::service::Routes;
    /// Adds native gRPC routes or listener settings to the runtime builder.
    fn with_grpc(mut self, grpc: Self::Grpc) -> Self {
        self.insert("grpc", grpc);
        self
    }
}
impl RuntimeGrpcExt for RuntimeBindings {
    type Grpc = GrpcServerConfig;
    /// Adds native gRPC routes or listener settings to the runtime builder.
    fn with_grpc(mut self, grpc: Self::Grpc) -> Self {
        self.insert("grpc", grpc);
        self
    }
}
/// Validates listener configuration and requires a selected service.
fn validate(graph: &CauldronGraph, config: &Config) -> furnace_rs_core::Result<()> {
    GrpcServerConfig::parse(config)?;
    if GrpcApplicationScope::for_cauldron_graph(Some(graph))?
        .services()
        .is_empty()
    {
        return Err(furnace_rs_core::Error::new(
            furnace_rs_core::Diagnostic::new(
                crate::FURNACE151,
                "no runnable gRPC service",
                "the selected application has no reachable managed protobuf service",
            ),
        ));
    }
    Ok(())
}
/// Registers gRPC runtime integration with the application builder.
fn configure(_: &mut FurnaceBuilder) {}
/// Builds the selected routes and server bindings before runtime startup.
fn prepare<'a>(
    app: &'a Furnace,
    routes: Option<&'a mut RuntimeRoutes>,
    bindings: Option<&'a mut RuntimeBindings>,
) -> BoxFuture<'a, Result<PreparedTransport, ApplicationRuntimeError>> {
    Box::pin(async move {
        let config = if let Some(bindings) = bindings {
            bindings
                .take::<GrpcServerConfig>("grpc")
                .expect("selected gRPC binding")
        } else {
            GrpcServerConfig::parse(app.context().config())
                .map_err(ApplicationRuntimeError::Bootstrap)?
        };
        let scope = GrpcApplicationScope::for_application(app)
            .map_err(ApplicationRuntimeError::Bootstrap)?;
        let routes = if let Some(routes) = routes {
            routes
                .take::<tonic::service::Routes>("grpc")
                .expect("selected gRPC routes")
        } else {
            crate::build_grpc_routes(app).map_err(ApplicationRuntimeError::Bootstrap)?
        };
        let (routes, mut health) = crate::install_grpc_infrastructure(app, &scope, routes)
            .map_err(ApplicationRuntimeError::Bootstrap)?;
        health.set_serving(false).await;
        let tls = super::server::prepare_tls(&config)
            .await
            .map_err(ApplicationRuntimeError::Bootstrap)?;
        let handshake = config.tls_handshake_timeout();
        let methods = scope
            .services()
            .iter()
            .flat_map(|service| {
                service
                    .methods()
                    .iter()
                    .map(move |method| (service.name(), method.name.as_str()))
            })
            .collect::<Vec<_>>();
        let service_width = methods
            .iter()
            .map(|(service, _)| service.len())
            .max()
            .unwrap_or(0)
            .max("SERVICE".len());
        let mut summary = format!("Methods  {}", methods.len());
        if !methods.is_empty() {
            let rows = methods
                .iter()
                .map(|(service, method)| format!("{service:service_width$}  {method}"))
                .collect::<Vec<_>>()
                .join("\n");
            let service_header = format!("{:<service_width$}", "SERVICE");
            let service_rule = "─".repeat(service_width);
            summary.push_str(&format!(
                "\n\n{service_header}  METHOD\n{service_rule}  ──────\n{rows}"
            ));
        }
        Ok(PreparedTransport {
            name: "grpc",
            address: config.address().to_string(),
            summary,
            drain_timeout: Some(config.shutdown_timeout()),
            transition: Some(Box::new(move |serving| {
                let mut health = health.clone();
                Box::pin(async move {
                    health.set_serving(serving).await;
                })
            })),
            serve: Box::new(move |listener, shutdown, tasks| {
                Box::pin(super::server::serve(
                    listener,
                    routes.prepare(),
                    shutdown,
                    tasks,
                    tls,
                    handshake,
                ))
            }),
        })
    })
}
furnace_rs_core::__private::inventory::submit! { RuntimeTransportDescriptor { name: "grpc", validate, configure, prepare } }
