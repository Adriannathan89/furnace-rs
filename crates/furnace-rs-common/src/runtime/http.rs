//! Adapter retaining the existing HTTP connection safeguards.
use super::{
    ApplicationRuntimeError as RuntimeError, PreparedTransport, RuntimeBindings, RuntimeRoutes,
    RuntimeTransportDescriptor,
};
use furnace_rs_core::{CauldronGraph, Config, Furnace, FurnaceBuilder};
use futures_util::future::BoxFuture;
/// Checks selected HTTP routes and validates their automatic bind configuration.
fn validate(graph: &CauldronGraph, config: &Config) -> furnace_rs_core::Result<()> {
    if !crate::http_scope::HttpApplicationScope::for_cauldron_graph(Some(graph))?.has_routes() {
        return Err(crate::server::no_runnable_route_error());
    }
    crate::server_config::ServerBinding::from_config(config)?;
    Ok(())
}
/// Enables the automatic HTTP and CORS configuration inputs for a Furnace builder.
fn configure(builder: &mut FurnaceBuilder) {
    crate::server_config::enable_automatic_server(builder);
    builder.__auto_configuration_input(
        crate::cors::CORS_AUTO_CONFIGURATION_ID,
        crate::server_config::HttpRuntimeMode::Automatic,
    );
}
/// Builds an HTTP router and packages its address, summary, and supervised serving operation.
fn prepare<'a>(
    app: &'a Furnace,
    routes: Option<&'a mut RuntimeRoutes>,
    bindings: Option<&'a mut RuntimeBindings>,
) -> BoxFuture<'a, Result<PreparedTransport, RuntimeError>> {
    Box::pin(async move {
        let router = if let Some(routes) = routes {
            routes
                .take::<axum::Router>("http")
                .expect("selected HTTP routes")
        } else {
            crate::build_router(app).map_err(RuntimeError::Bootstrap)?
        };
        let router = crate::configure_router(app, router).map_err(RuntimeError::Bootstrap)?;
        let address = if let Some(bindings) = bindings {
            bindings
                .take::<std::net::SocketAddr>("http")
                .expect("selected HTTP binding")
                .to_string()
        } else {
            let binding = app
                .context()
                .resolve::<crate::server_config::ServerBinding>()
                .map_err(RuntimeError::Bootstrap)?;
            let host = binding.host();
            if host.contains(':') {
                format!("[{host}]:{}", binding.port())
            } else {
                format!("{host}:{}", binding.port())
            }
        };
        let scope = crate::http_scope::HttpApplicationScope::for_application(app)
            .map_err(RuntimeError::Bootstrap)?;
        let routes = scope
            .route_records()
            .map(|(_, route)| route)
            .collect::<Vec<_>>();
        let mut summary = format!("Routes  {}", routes.len());
        if !routes.is_empty() {
            let rows = routes
                .iter()
                .map(|route| format!("{:<6}  {}", route.method().as_str(), route.full_path()))
                .collect::<Vec<_>>()
                .join("\n");
            summary.push_str(&format!("\n\nMETHOD  PATH\n──────  ────\n{rows}"));
        }
        let debug = app.context().config().get("furnace.mode") == Some("debug");
        Ok(PreparedTransport {
            name: "http",
            address,
            summary,
            drain_timeout: None,
            transition: None,
            serve: Box::new(move |listener, mut shutdown, tasks| {
                Box::pin(async move {
                    crate::server::serve_http_supervised(
                        debug,
                        listener,
                        router,
                        async move {
                            if !*shutdown.borrow_and_update() {
                                let _ = shutdown.changed().await;
                            }
                        },
                        Some(tasks),
                    )
                    .await
                    .map_err(RuntimeError::Serve)
                })
            }),
        })
    })
}
furnace_rs_core::__private::inventory::submit! { RuntimeTransportDescriptor { name: "http", validate, configure, prepare } }
