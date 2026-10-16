//! Transport registry integration and native upgrade routing.
use super::SocketRuntime;
use crate::{
    SocketApplicationScope, SocketEndpointDescriptor, SocketHandshake, SocketResult,
    SocketServerConfig, WebSocket,
};
use axum::{
    Extension,
    extract::{Request, ws::WebSocketUpgrade},
    routing::{MethodRouter, get},
};
use furnace_rs_common::runtime::*;
use furnace_rs_core::{ApplicationContext, CauldronGraph, Config, Furnace, FurnaceBuilder};
use futures_util::future::BoxFuture;
use std::{net::SocketAddr, sync::Arc};
/// Native composition for the separate listener.
pub trait RuntimeSocketExt: Sized {
    /// Builder's socket route/config type.
    type Socket;
    /// Adds socket routes or bindings.
    fn with_socket(self, socket: Self::Socket) -> Self;
}
impl RuntimeSocketExt for RuntimeRoutes {
    type Socket = axum::Router;
    fn with_socket(mut self, socket: Self::Socket) -> Self {
        self.insert("socket", socket);
        self
    }
}
impl RuntimeSocketExt for RuntimeBindings {
    type Socket = SocketServerConfig;
    fn with_socket(mut self, socket: Self::Socket) -> Self {
        self.insert("socket", socket);
        self
    }
}
/// Builds selected upgrade routes without binding a listener.
#[allow(clippy::result_large_err)]
pub fn build_socket_routes(app: &Furnace) -> furnace_rs_core::Result<axum::Router> {
    let mut router = axum::Router::new();
    for endpoint in SocketApplicationScope::for_application(app)?.endpoints() {
        let build = endpoint
            .route
            .ok_or_else(|| crate::config::invalid("socket.registration"))?;
        router = router.route(endpoint.path, build(app.context().clone())?);
    }
    Ok(router)
}
/// Typed adapter used by owned-endpoint registration.
#[doc(hidden)]
#[allow(clippy::result_large_err)]
pub fn owned_route<T: Send + Sync + 'static>(
    app: ApplicationContext,
    endpoint: &'static SocketEndpointDescriptor,
    handler: fn(Arc<T>, SocketHandshake, WebSocket) -> BoxFuture<'static, SocketResult<()>>,
) -> furnace_rs_core::Result<MethodRouter> {
    let provider = app.resolve::<T>()?;
    Ok(get(
        move |ws: WebSocketUpgrade,
              Extension(runtime): Extension<SocketRuntime>,
              request: Request| {
            let provider = provider.clone();
            let app = app.clone();
            async move {
                if *runtime.shutdown.borrow() {
                    return http::StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                let origin = request
                    .headers()
                    .get_all(http::header::ORIGIN)
                    .iter()
                    .collect::<Vec<_>>();
                if origin.len() > 1
                    || origin.first().is_some_and(|v| {
                        v.to_str()
                            .ok()
                            .is_none_or(|v| !runtime.config.origins.iter().any(|o| o == v))
                    })
                {
                    return http::StatusCode::FORBIDDEN.into_response();
                }
                let _ = endpoint;
                let handshake = SocketHandshake {
                    application: app,
                    headers: request.headers().clone(),
                    method: request.method().clone(),
                    uri: request.uri().clone(),
                    peer: request.extensions().get::<SocketAddr>().copied(),
                    extensions: Arc::new(http::Extensions::new()),
                };
                ws.max_message_size(runtime.config.max_message_bytes)
                    .max_frame_size(runtime.config.max_message_bytes)
                    .on_upgrade(move |socket| async move {
                        runtime.tasks.spawn(async move {
                            if let Err(error) = handler(provider, handshake, socket).await {
                                tracing::debug!(code = error.code(), "owned socket closed");
                            }
                        });
                    })
                    .into_response()
            }
        },
    ))
}
use axum::response::IntoResponse;
#[allow(clippy::result_large_err)]
fn validate(graph: &CauldronGraph, config: &Config) -> furnace_rs_core::Result<()> {
    SocketServerConfig::parse(config)?;
    if SocketApplicationScope::for_cauldron_graph(Some(graph))?
        .endpoints()
        .is_empty()
    {
        return Err(crate::config::invalid("socket.registration"));
    }
    Ok(())
}
fn configure(_: &mut FurnaceBuilder) {}
fn prepare<'a>(
    app: &'a Furnace,
    routes: Option<&'a mut RuntimeRoutes>,
    bindings: Option<&'a mut RuntimeBindings>,
) -> BoxFuture<'a, Result<PreparedTransport, ApplicationRuntimeError>> {
    Box::pin(async move {
        let config = match bindings {
            Some(b) => b.take::<SocketServerConfig>("socket").ok_or_else(|| {
                ApplicationRuntimeError::Bootstrap(crate::config::invalid("socket.binding"))
            })?,
            None => SocketServerConfig::parse(app.context().config())
                .map_err(ApplicationRuntimeError::Bootstrap)?,
        };
        let router = match routes {
            Some(r) => r.take::<axum::Router>("socket").ok_or_else(|| {
                ApplicationRuntimeError::Bootstrap(crate::config::invalid("socket.routes"))
            })?,
            None => build_socket_routes(app).map_err(ApplicationRuntimeError::Bootstrap)?,
        };
        Ok(PreparedTransport {
            name: "socket",
            address: config.address.to_string(),
            summary: "WebSocket endpoints".into(),
            drain_timeout: Some(config.shutdown_timeout),
            transition: None,
            serve: Box::new(move |listener, shutdown, tasks| {
                Box::pin(super::server::serve(
                    listener,
                    router,
                    SocketRuntime {
                        config,
                        shutdown,
                        tasks,
                    },
                ))
            }),
        })
    })
}
furnace_rs_core::__private::inventory::submit! {RuntimeTransportDescriptor{name:"socket",validate,configure,prepare}}
