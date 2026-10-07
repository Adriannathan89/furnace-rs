//! Validated HTTP server startup and lifecycle coordination.
//!
//! [`serve`] validates route metadata and builds a raw generated router, while
//! [`serve_router`] accepts a complete raw router from a caller. Both finalize
//! router configuration before they start application lifecycle hooks or ask
//! Tokio to bind a listener.

use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use furnace_rs_core::{Cauldron, Diagnostic, DiagnosticCode, Error, FURNACE020, Furnace};
use tokio::net::TcpListener;

use crate::cors::CORS_AUTO_CONFIGURATION_ID;
use crate::http_scope::HttpApplicationScope;
use crate::route::RouteDescriptor;
use crate::server_config::{
    HttpRuntimeMode, SERVER_AUTO_CONFIGURATION_ID, ServerBinding, load_standard_config_from,
};
use crate::{build_router, configure_router};

mod body;

const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(10);
const BODY_READ_IDLE_TIMEOUT: Duration = Duration::from_secs(10);

/// A standard application run had no reachable managed HTTP route.
pub const FURNACE031: DiagnosticCode = DiagnosticCode::new("FURNACE031");

/// An error produced while preparing, running, or stopping the HTTP runtime.
#[derive(Debug)]
#[non_exhaustive]
pub enum HttpRuntimeError {
    /// Route validation or final router configuration failed before lifecycle startup.
    Bootstrap(furnace_rs_core::Error),
    /// Application lifecycle startup or shutdown failed.
    Lifecycle(furnace_rs_core::Error),
    /// The HTTP listener could not bind to the requested address.
    Bind(std::io::Error),
    /// Axum stopped because of an HTTP serving failure.
    Serve(std::io::Error),
    /// An operational failure occurred and the subsequent shutdown also failed.
    OperationAndShutdown {
        /// The bind or serving error that initiated shutdown.
        operation: Box<HttpRuntimeError>,
        /// The lifecycle error returned while attempting shutdown.
        shutdown: furnace_rs_core::Error,
    },
}

impl fmt::Display for HttpRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bootstrap(error) => write!(formatter, "HTTP bootstrap failed: {error}"),
            Self::Lifecycle(error) => write!(formatter, "application lifecycle failed: {error}"),
            Self::Bind(error) => write!(formatter, "HTTP listener bind failed: {error}"),
            Self::Serve(error) => write!(formatter, "HTTP serving failed: {error}"),
            Self::OperationAndShutdown {
                operation,
                shutdown,
            } => write!(
                formatter,
                "{operation}; application shutdown also failed: {shutdown}"
            ),
        }
    }
}

impl StdError for HttpRuntimeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Bootstrap(error) | Self::Lifecycle(error) => Some(error),
            Self::Bind(error) | Self::Serve(error) => Some(error),
            Self::OperationAndShutdown { operation, .. } => Some(operation.as_ref()),
        }
    }
}

/// Runs a rooted application module with conventional HTTP configuration.
///
/// Import this trait to call [`Furnace::burn`]. The standard path loads optional
/// `.env` and `furnace.toml` files from the current working directory, applies
/// `FURNACE_*` environment overrides, and owns automatic HTTP binding. Use the
/// low-level [`Furnace::builder`] and [`serve`] APIs when the application needs
/// explicit configuration, providers, lifecycle hooks, or a listener address.
pub trait FurnaceBurnExt {
    /// Builds and runs the selected rooted application module.
    fn burn<M>() -> impl Future<Output = Result<(), HttpRuntimeError>> + Send
    where
        M: Cauldron;
}

impl FurnaceBurnExt for Furnace {
    #[allow(clippy::manual_async_fn)] // The public trait keeps an explicit `Send` future.
    fn burn<M>() -> impl Future<Output = Result<(), HttpRuntimeError>> + Send
    where
        M: Cauldron,
    {
        async move {
            let root = std::env::current_dir().map_err(config_directory_error)?;
            if let Some(result) = crate::inspection::try_run_inspection::<M>(&root) {
                return result;
            }
            let prepared = prepare_standard_run::<M>(&root).await?;
            println!("{}", prepared.startup_summary());
            serve_prepared(prepared, TcpListener::bind, shutdown_signal()).await
        }
    }
}

struct PreparedStandardRun {
    application: Furnace,
    router: axum::Router,
    binding: std::sync::Arc<ServerBinding>,
    route_count: usize,
    routes: Vec<RouteDescriptor>,
}

struct StartupSummary {
    host: String,
    port: u16,
    route_count: usize,
    routes: Vec<RouteDescriptor>,
}

impl StartupSummary {
    const fn new(
        host: String,
        port: u16,
        route_count: usize,
        routes: Vec<RouteDescriptor>,
    ) -> Self {
        Self {
            host,
            port,
            route_count,
            routes,
        }
    }
}

impl fmt::Display for StartupSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "\n  FURNACE · Application ready\n  ────────────────────────────\n  Server  http://{}:{}\n  Routes  {}",
            self.host, self.port, self.route_count
        )?;
        if !self.routes.is_empty() {
            write!(formatter, "\n\n  METHOD  PATH\n  ──────  ────")?;
        }
        for route in &self.routes {
            write!(
                formatter,
                "\n  {:<6}  {}",
                route.method().as_str(),
                route.full_path()
            )?;
        }
        Ok(())
    }
}

impl PreparedStandardRun {
    fn startup_summary(&self) -> StartupSummary {
        StartupSummary::new(
            self.binding.host().to_owned(),
            self.binding.port(),
            self.route_count,
            self.routes.clone(),
        )
    }
}

async fn prepare_standard_run<M: Cauldron>(
    root: &Path,
) -> Result<PreparedStandardRun, HttpRuntimeError> {
    let config = load_standard_config_from(root).map_err(HttpRuntimeError::Bootstrap)?;
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<M>().map_err(HttpRuntimeError::Bootstrap)?;
    let server_input_registered = builder
        .__auto_configuration_input(SERVER_AUTO_CONFIGURATION_ID, HttpRuntimeMode::Automatic);
    debug_assert!(server_input_registered);
    let cors_input_registered =
        builder.__auto_configuration_input(CORS_AUTO_CONFIGURATION_ID, HttpRuntimeMode::Automatic);
    debug_assert!(cors_input_registered);
    let application = builder.build().await.map_err(HttpRuntimeError::Bootstrap)?;

    prepare_standard_application(application)
}

fn prepare_standard_application(
    application: Furnace,
) -> Result<PreparedStandardRun, HttpRuntimeError> {
    let scope =
        HttpApplicationScope::for_application(&application).map_err(HttpRuntimeError::Bootstrap)?;
    if !scope.has_routes() {
        return Err(HttpRuntimeError::Bootstrap(no_runnable_route_error()));
    }
    let route_count = scope.route_records().count();

    let router = build_router(&application).map_err(HttpRuntimeError::Bootstrap)?;
    let router = configure_router(&application, router).map_err(HttpRuntimeError::Bootstrap)?;
    let binding = application
        .context()
        .resolve::<ServerBinding>()
        .map_err(HttpRuntimeError::Bootstrap)?;
    let routes = scope.route_records().map(|(_, route)| *route).collect();

    Ok(PreparedStandardRun {
        application,
        router,
        binding,
        route_count,
        routes,
    })
}

async fn serve_prepared<B, BindFuture, Shutdown>(
    prepared: PreparedStandardRun,
    binder: B,
    shutdown: Shutdown,
) -> Result<(), HttpRuntimeError>
where
    B: FnOnce((String, u16)) -> BindFuture,
    BindFuture: Future<Output = std::io::Result<TcpListener>>,
    Shutdown: Future<Output = ()> + Send + 'static,
{
    let PreparedStandardRun {
        application,
        router,
        binding,
        route_count: _,
        routes: _,
    } = prepared;
    let address = (binding.host().to_owned(), binding.port());
    serve_configured_router_with(application, router, address, binder, shutdown).await
}

fn no_runnable_route_error() -> Error {
    Error::new(
        Diagnostic::new(
            FURNACE031,
            "no runnable HTTP route",
            "the selected application has no reachable managed HTTP route",
        )
        .with_subject("HTTP runtime")
        .with_suggestion("declare a managed controller route reachable from the root module"),
    )
}

fn config_directory_error(error: std::io::Error) -> HttpRuntimeError {
    HttpRuntimeError::Bootstrap(Error::with_source(
        Diagnostic::new(
            FURNACE020,
            "configuration directory could not be determined",
            "could not determine the process current working directory",
        )
        .with_subject("current working directory")
        .with_suggestion("run the application from an accessible directory"),
        error,
    ))
}

/// Builds, configures, starts, serves, and shuts down an application on `address`.
///
/// Generated-route validation and final router configuration complete before
/// lifecycle hooks start or the listener is bound. Once lifecycle startup
/// succeeds, every exit path attempts shutdown.
/// Incomplete initial requests and HTTP/1 request headers expire after ten
/// seconds. This deadline does not limit handler execution or response streaming.
/// A request-body read that waits ten seconds without a frame times out. Before
/// response headers are sent, this produces HTTP 408; a timeout while streaming
/// a response terminates that stream instead.
/// A bind or serving failure is retained if shutdown succeeds; if shutdown
/// also fails, both failures are returned in [`HttpRuntimeError::OperationAndShutdown`].
///
/// # Errors
///
/// Returns [`HttpRuntimeError::Bootstrap`] for route validation, controller
/// resolution, or registrar failures; [`HttpRuntimeError::Lifecycle`] for
/// lifecycle start or clean-shutdown failures; [`HttpRuntimeError::Bind`] when
/// the address cannot be bound; [`HttpRuntimeError::Serve`] for an Axum serving
/// failure; or [`HttpRuntimeError::OperationAndShutdown`] when an operational
/// failure and its cleanup failure occur together.
///
/// # Examples
///
/// ```no_run
/// use furnace_rs_common::{core::Furnace, serve};
///
/// #[tokio::main]
/// async fn main() -> Result<(), furnace_rs_common::HttpRuntimeError> {
///     let application = Furnace::builder().build().await.map_err(
///         furnace_rs_common::HttpRuntimeError::Bootstrap,
///     )?;
///     serve(application, "127.0.0.1:3000").await
/// }
/// ```
#[allow(clippy::result_large_err)]
pub async fn serve(
    application: Furnace,
    address: impl tokio::net::ToSocketAddrs,
) -> Result<(), HttpRuntimeError> {
    let router = build_router(&application).map_err(HttpRuntimeError::Bootstrap)?;
    serve_router(application, router, address).await
}

/// Configures, starts, serves, and shuts down an application with a complete raw router.
///
/// Pass the raw router after merging any generated and native routes. This
/// function applies final application-wide configuration, including CORS, once
/// before lifecycle startup. Call [`crate::configure_router`] only when using a
/// router directly; passing an already configured router here would apply that
/// configuration twice.
///
/// The explicit `address` is the complete listener override. It is resolved
/// and bound after lifecycle startup, and it may use port zero regardless of
/// any automatic `server.host` or `server.port` configuration.
/// Incomplete initial requests and HTTP/1 request headers expire after ten
/// seconds without imposing a handler execution deadline.
/// Request-body reads also have a ten-second idle deadline, renewed by progress.
///
/// # Errors
///
/// Returns [`HttpRuntimeError::Bootstrap`] when final router configuration
/// fails before lifecycle startup. Lifecycle, bind, serving, and combined
/// operational/shutdown errors follow the same contract as [`serve`].
#[allow(clippy::result_large_err)]
pub async fn serve_router(
    application: Furnace,
    router: axum::Router,
    address: impl tokio::net::ToSocketAddrs,
) -> Result<(), HttpRuntimeError> {
    serve_router_with(
        application,
        router,
        address,
        TcpListener::bind,
        shutdown_signal(),
    )
    .await
}

#[cfg(test)]
async fn serve_with<Address, B, BindFuture, Shutdown>(
    application: Furnace,
    address: Address,
    binder: B,
    shutdown: Shutdown,
) -> Result<(), HttpRuntimeError>
where
    B: FnOnce(Address) -> BindFuture,
    BindFuture: Future<Output = std::io::Result<TcpListener>>,
    Shutdown: Future<Output = ()> + Send + 'static,
{
    let router = build_router(&application).map_err(HttpRuntimeError::Bootstrap)?;
    serve_router_with(application, router, address, binder, shutdown).await
}

async fn serve_router_with<Address, B, BindFuture, Shutdown>(
    application: Furnace,
    router: axum::Router,
    address: Address,
    binder: B,
    shutdown: Shutdown,
) -> Result<(), HttpRuntimeError>
where
    B: FnOnce(Address) -> BindFuture,
    BindFuture: Future<Output = std::io::Result<TcpListener>>,
    Shutdown: Future<Output = ()> + Send + 'static,
{
    let router = configure_router(&application, router).map_err(HttpRuntimeError::Bootstrap)?;
    serve_configured_router_with(application, router, address, binder, shutdown).await
}

async fn serve_configured_router_with<Address, B, BindFuture, Shutdown>(
    mut application: Furnace,
    router: axum::Router,
    address: Address,
    binder: B,
    shutdown: Shutdown,
) -> Result<(), HttpRuntimeError>
where
    B: FnOnce(Address) -> BindFuture,
    BindFuture: Future<Output = std::io::Result<TcpListener>>,
    Shutdown: Future<Output = ()> + Send + 'static,
{
    let debug = application.context().config().get("furnace.mode") == Some("debug");
    application
        .start()
        .await
        .map_err(HttpRuntimeError::Lifecycle)?;
    let listener = match binder(address).await {
        Ok(listener) => listener,
        Err(error) => {
            return finish_after_error(application, HttpRuntimeError::Bind(error)).await;
        }
    };
    let result = serve_http(debug, listener, router, shutdown)
        .await
        .map_err(HttpRuntimeError::Serve);
    finish(application, result).await
}

// Axum's convenience server does not install a Hyper timer. Without one,
// incomplete request headers retain their socket and task indefinitely.
async fn serve_http(
    debug: bool,
    mut listener: TcpListener,
    router: axum::Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    use axum::response::IntoResponse;
    use axum::serve::Listener;
    use hyper_util::{
        rt::{TokioExecutor, TokioIo, TokioTimer},
        server::conn::auto::Builder,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tower::ServiceExt;

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
    let mut connections = tokio::task::JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            (stream, _) = Listener::accept(&mut listener) => {
                let first_request = std::sync::Arc::new(tokio::sync::Notify::new());
                let request_received = std::sync::Arc::clone(&first_request);
                let connection_router = router.clone().with_state(());
                let service = hyper::service::service_fn(move |request: axum::http::Request<hyper::body::Incoming>| {
                    request_received.notify_one();
                    let router = connection_router.clone();
                    async move {
                        let path = debug.then(|| request.uri().path().to_owned());
                        let method = debug.then(|| request.method().clone());
                        let version = request.version();
                        let expired = Arc::new(AtomicBool::new(false));
                        let request = request.map(|body| axum::body::Body::new(body::IdleTimeoutBody::new(body, BODY_READ_IDLE_TIMEOUT, Arc::clone(&expired))));
                        let response = router.oneshot(request).await?;
                        let response = if expired.load(Ordering::Relaxed) {
                            let mut response = (
                                axum::http::StatusCode::REQUEST_TIMEOUT,
                                axum::Json(serde_json::json!({"error": {"code": "request_timeout", "message": "request body read timed out"}})),
                            ).into_response();
                            if matches!(version, axum::http::Version::HTTP_10 | axum::http::Version::HTTP_11) {
                                response.headers_mut().insert(axum::http::header::CONNECTION, axum::http::HeaderValue::from_static("close"));
                            }
                            response
                        } else {
                            response
                        };
                        if let (Some(path), Some(method)) = (path, method) {
                            use std::io::Write;
                            let _ = writeln!(
                                std::io::stdout().lock(),
                                "[debug] {} | {:3} | {:<7} | {}",
                                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                                response.status().as_u16(),
                                method.as_str(),
                                path,
                            );
                        }
                        Ok::<_, std::convert::Infallible>(response)
                    }
                });
                let mut shutdown_rx = shutdown_rx.clone();
                connections.spawn(async move {
                    let mut builder = Builder::new(TokioExecutor::new());
                    builder.http1().timer(TokioTimer::new())
                        .header_read_timeout(HEADER_READ_TIMEOUT);
                    builder.http2().enable_connect_protocol();
                    let connection = builder
                        .serve_connection_with_upgrades(TokioIo::new(stream), service);
                    tokio::pin!(connection);
                    // Automatic protocol detection precedes Hyper's header timer.
                    // Bound that phase too, including clients sending no bytes or
                    // only a partial HTTP/2 preface. Stop this timer once the first
                    // request arrives so long-running handlers are unaffected.
                    let initial_headers = async {
                        if tokio::time::timeout(HEADER_READ_TIMEOUT, first_request.notified()).await.is_ok() {
                            std::future::pending::<()>().await;
                        }
                    };
                    tokio::select! {
                        _ = &mut connection => {},
                        _ = initial_headers => {},
                        _ = shutdown_rx.changed() => {
                            connection.as_mut().graceful_shutdown();
                            let _ = connection.await;
                        }
                    }
                });
            }
        }
    }
    // Stop accepting before draining active requests and running lifecycle hooks.
    drop(listener);
    drop(shutdown_tx);
    while connections.join_next().await.is_some() {}
    Ok(())
}

async fn finish(
    mut application: Furnace,
    operation: Result<(), HttpRuntimeError>,
) -> Result<(), HttpRuntimeError> {
    match (operation, application.shutdown().await) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(shutdown)) => Err(HttpRuntimeError::Lifecycle(shutdown)),
        (Err(operation), Ok(())) => Err(operation),
        (Err(operation), Err(shutdown)) => Err(HttpRuntimeError::OperationAndShutdown {
            operation: Box::new(operation),
            shutdown,
        }),
    }
}

async fn finish_after_error(
    application: Furnace,
    operation: HttpRuntimeError,
) -> Result<(), HttpRuntimeError> {
    finish(application, Err(operation)).await
}

async fn shutdown_signal() {
    let Some(path) = std::env::var_os(crate::__private::DEV_SHUTDOWN_ENV).map(PathBuf::from) else {
        os_shutdown_signal().await;
        return;
    };

    tokio::select! {
        _ = os_shutdown_signal() => {}
        _ = wait_for_dev_shutdown(path) => {}
    }
}

async fn os_shutdown_signal() {
    #[cfg(unix)]
    if let Ok(mut terminate) =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
        return;
    }

    let _ = tokio::signal::ctrl_c().await;
}

async fn wait_for_dev_shutdown(path: PathBuf) {
    loop {
        if tokio::fs::metadata(&path)
            .await
            .is_ok_and(|metadata| metadata.is_file())
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::io::{self, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use furnace_rs_core::{
        ApplicationContext, AutoConfigurationStatus, Cauldron, ConfigBuilder, Diagnostic, Error,
        FURNACE011, FURNACE020, Furnace, LifecycleFuture, LifecycleHook, MapSource, SourceLocation,
    };
    use tokio::net::TcpListener;

    use super::{
        FURNACE031, HttpRuntimeError, StartupSummary, prepare_standard_application,
        prepare_standard_run, serve_prepared, serve_router_with, serve_with, wait_for_dev_shutdown,
    };
    use crate::cors::CORS_AUTO_CONFIGURATION_ID;
    use crate::server_config::{HttpRuntimeMode, SERVER_AUTO_CONFIGURATION_ID, ServerBinding};
    use crate::{ControllerEndpointDescriptor, HttpMethod, RouteDescriptor};

    static STARTS: AtomicUsize = AtomicUsize::new(0);
    static BINDS: AtomicUsize = AtomicUsize::new(0);
    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn body_deadline_preserves_valid_json_and_body_size_rejections() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = axum::Router::new()
            .route(
                "/upload",
                axum::routing::post(|_: axum::Json<serde_json::Value>| async { "accepted" }),
            )
            .layer(axum::extract::DefaultBodyLimit::max(8));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(super::serve_http(false, listener, router, async move {
            let _ = shutdown_rx.await;
        }));
        for (length, body, status) in [
            (2, "{}", "200"),
            (1, "{", "400"),
            (16, "abcdefghijklmnop", "413"),
        ] {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            let request = format!(
                "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
            );
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut response = Vec::new();
            tokio::time::timeout(Duration::from_secs(2), stream.read_to_end(&mut response))
                .await
                .unwrap()
                .unwrap();
            assert!(
                response.starts_with(format!("HTTP/1.1 {status}").as_bytes()),
                "unexpected response: {response:?}"
            );
        }
        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn stalled_http2_body_does_not_stop_other_requests() {
        use hyper_util::rt::{TokioExecutor, TokioIo};

        struct StalledBody;
        impl hyper::body::Body for StalledBody {
            type Data = hyper::body::Bytes;
            type Error = std::convert::Infallible;
            fn poll_frame(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>>
            {
                std::task::Poll::Pending
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = axum::Router::new()
            .route(
                "/upload",
                axum::routing::post(|_: axum::Json<serde_json::Value>| async { "accepted" }),
            )
            .route("/health", axum::routing::get(|| async { "healthy" }));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(super::serve_http(false, listener, router, async move {
            let _ = shutdown_rx.await;
        }));
        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        let (mut sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream))
                .await
                .unwrap();
        let client = tokio::spawn(connection);
        let upload = axum::http::Request::builder()
            .method("POST")
            .uri("http://localhost/upload")
            .header("content-type", "application/json")
            .body(axum::body::Body::new(StalledBody))
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(12), sender.send_request(upload))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::REQUEST_TIMEOUT);
        assert!(
            !response
                .headers()
                .contains_key(axum::http::header::CONNECTION)
        );
        let body = axum::body::to_bytes(axum::body::Body::new(response.into_body()), 1024)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"error": {"code": "request_timeout", "message": "request body read timed out"}})
        );
        let health = axum::http::Request::builder()
            .uri("http://localhost/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(2), sender.send_request(health))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(axum::body::Body::new(response.into_body()), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"healthy");
        drop(sender);
        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        client.abort();
    }

    #[tokio::test]
    async fn stalled_request_body_expires_and_server_remains_available() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = axum::Router::new()
            .route(
                "/upload",
                axum::routing::post(|_: axum::Json<serde_json::Value>| async { "accepted" }),
            )
            .route("/health", axum::routing::get(|| async { "healthy" }));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(super::serve_http(false, listener, router, async move {
            let _ = shutdown_rx.await;
        }));
        let mut stalled = tokio::net::TcpStream::connect(address).await.unwrap();
        stalled.write_all(b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 10\r\n\r\n{").await.unwrap();
        let mut response = Vec::new();
        let expired =
            tokio::time::timeout(Duration::from_secs(12), stalled.read_to_end(&mut response)).await;
        drop(stalled);

        let mut healthy = tokio::net::TcpStream::connect(address).await.unwrap();
        healthy
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut health_response = Vec::new();
        tokio::time::timeout(
            Duration::from_secs(2),
            healthy.read_to_end(&mut health_response),
        )
        .await
        .unwrap()
        .unwrap();
        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(health_response.starts_with(b"HTTP/1.1 200"));
        assert!(health_response.ends_with(b"healthy"));
        assert!(
            matches!(expired, Ok(Ok(_))),
            "stalled upload did not expire: {expired:?}"
        );
        assert!(
            response.starts_with(b"HTTP/1.1 408"),
            "expected safe timeout response: {response:?}"
        );
    }

    #[tokio::test]
    async fn header_deadline_does_not_cancel_handlers_or_graceful_drain() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let handler_started = Arc::clone(&started);
        let handler_release = Arc::clone(&release);
        let router = axum::Router::new().route(
            "/slow",
            axum::routing::get(move || {
                let started = Arc::clone(&handler_started);
                let release = Arc::clone(&handler_release);
                async move {
                    started.notify_one();
                    release.notified().await;
                    "completed"
                }
            }),
        );
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(super::serve_http(false, listener, router, async move {
            let _ = shutdown_rx.await;
        }));
        let client = tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            stream
                .write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            let mut response = Vec::new();
            tokio::time::timeout(Duration::from_secs(15), stream.read_to_end(&mut response))
                .await
                .unwrap()
                .unwrap();
            response
        });
        tokio::time::timeout(Duration::from_secs(2), started.notified())
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(11)).await;
        let _ = shutdown_tx.send(());
        tokio::task::yield_now().await;
        assert!(
            !server.is_finished(),
            "shutdown must drain the active handler"
        );
        release.notify_one();
        let response = client.await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(response.ends_with(b"completed"));
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn idle_connections_and_partial_protocol_prefaces_expire() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(super::serve_http(
            false,
            listener,
            axum::Router::new(),
            async move {
                let _ = shutdown_rx.await;
            },
        ));
        let mut checks = tokio::task::JoinSet::new();
        for prefix in [b"".as_slice(), b"PRI", b"G"] {
            let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
            stream.write_all(prefix).await.unwrap();
            checks.spawn(async move {
                let mut byte = [0];
                tokio::time::timeout(Duration::from_secs(12), stream.read(&mut byte)).await
            });
        }
        let mut results = Vec::new();
        while let Some(result) = checks.join_next().await {
            results.push(result.unwrap());
        }
        let _ = shutdown_tx.send(());
        server.await.unwrap().unwrap();
        for result in results {
            assert!(
                matches!(result, Ok(Ok(0)) | Ok(Err(_))),
                "connection did not expire: {result:?}"
            );
        }
    }

    #[tokio::test]
    async fn header_deadline_preserves_http2_requests() {
        use hyper_util::rt::{TokioExecutor, TokioIo};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router =
            axum::Router::new().route("/health", axum::routing::get(|| async { "healthy" }));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(super::serve_http(false, listener, router, async move {
            let _ = shutdown_rx.await;
        }));
        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        let (mut sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream))
                .await
                .unwrap();
        let client = tokio::spawn(connection);
        let request = axum::http::Request::builder()
            .uri("http://localhost/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(2), sender.send_request(request))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(axum::body::Body::new(response.into_body()), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"healthy");
        drop(sender);
        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        client.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn incomplete_headers_expire_without_stopping_the_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut builder = Furnace::builder();
        builder.root::<raw_router::App>().unwrap();
        let application = builder.build().await.unwrap();
        let router =
            axum::Router::new().route("/health", axum::routing::get(|| async { "healthy" }));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(serve_router_with(
            application,
            router,
            (),
            move |_| async move { Ok(listener) },
            async move {
                let _ = shutdown_rx.await;
            },
        ));

        // One incomplete request is enough to check the resource-retention bug;
        // this test never floods the listener or exhausts machine resources.
        let mut stalled = tokio::net::TcpStream::connect(address).await.unwrap();
        stalled
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nX-Stalled: ")
            .await
            .unwrap();
        let mut byte = [0];
        let expired = tokio::time::timeout(Duration::from_secs(12), stalled.read(&mut byte)).await;

        let mut healthy = tokio::net::TcpStream::connect(address).await.unwrap();
        healthy
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(2), healthy.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200"));
        assert!(response.ends_with(b"healthy"));

        // Clean up even when the unpatched server keeps the request open.
        drop(stalled);
        let _ = shutdown_tx.send(());
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(
            matches!(expired, Ok(Ok(0)) | Ok(Err(_))),
            "incomplete headers retained a connection past the deadline: {expired:?}"
        );
    }

    #[tokio::test]
    async fn private_shutdown_file_completes_the_standard_shutdown_signal() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shutdown");
        let mut waiting = tokio::spawn(wait_for_dev_shutdown(path.clone()));

        assert!(
            tokio::time::timeout(Duration::from_millis(110), &mut waiting)
                .await
                .is_err()
        );

        tokio::fs::write(&path, b"stop").await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .expect("shutdown file should be observed")
            .unwrap();
    }

    #[furnace_rs_core::cauldron]
    struct ServerTestApp;

    impl furnace_rs_core::Cauldron for ServerTestApp {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.controller::<PreflightController>()
                .provide::<RouterPreflightEvents>()
                .provide::<PreflightPermit>()
        }
    }

    mod raw_router {
        #[furnace_rs_core::cauldron]
        pub(super) struct App;

        impl furnace_rs_core::Cauldron for App {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                furnace_rs_core::CauldronRegistration::new(self)
            }
        }
    }

    mod standard_run {
        pub(super) mod routed {

            #[furnace_rs_common_macros::controller]
            pub(super) struct RoutedController;

            impl crate::Sealable for RoutedController {
                fn seals() -> crate::SealRegistration<Self> {
                    crate::SealRegistration::new()
                }
            }

            #[furnace_rs_common_macros::controller]
            impl RoutedController {
                #[furnace_rs_common_macros::get("/standard-run-health")]
                async fn health(&self) -> &'static str {
                    "healthy"
                }
            }

            #[furnace_rs_core::cauldron]
            pub struct RoutedApp;

            impl furnace_rs_core::Cauldron for RoutedApp {
                fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                    self.controller::<RoutedController>()
                }
            }
        }

        pub(super) mod empty {
            #[furnace_rs_core::cauldron]
            pub struct EmptyApp;

            impl furnace_rs_core::Cauldron for EmptyApp {
                fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                    furnace_rs_core::CauldronRegistration::new(self)
                }
            }
        }
    }

    #[cfg(feature = "jwt")]
    mod unreachable_jwt {
        use crate::{ClaimsPrincipal, PassportPrincipal};

        #[derive(serde::Deserialize)]
        pub(super) struct UnreachableClaims;

        impl PassportPrincipal for UnreachableClaims {
            fn has_role(&self, _: &str) -> bool {
                false
            }

            fn has_permission(&self, _: &str) -> bool {
                false
            }
        }

        #[furnace_rs_common_macros::controller]
        pub(super) struct UnreachableController;

        #[furnace_rs_common_macros::guard(principal = ClaimsPrincipal < UnreachableClaims >, strategy = "jwt")]
        struct UnreachableControllerGuard;

        impl crate::Sealable for UnreachableController {
            fn seals() -> crate::SealRegistration<Self> {
                Self::seal::<UnreachableControllerGuard>()
            }
        }

        #[furnace_rs_common_macros::controller]
        impl UnreachableController {
            #[furnace_rs_common_macros::get("/unreachable")]
            async fn unreachable(&self) -> &'static str {
                "unreachable"
            }
        }

        #[furnace_rs_core::cauldron]
        pub(super) struct UnreachableJwtCauldron;

        impl furnace_rs_core::Cauldron for UnreachableJwtCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.controller::<UnreachableController>()
            }
        }
    }

    struct PreflightController;
    struct PreflightPermit(bool);

    #[derive(Clone)]
    struct RouterPreflightEvents(Arc<Mutex<Vec<&'static str>>>);

    impl furnace_rs_core::Injector for RouterPreflightEvents {
        type Dependencies = ();
        async fn inject((): ()) -> furnace_rs_core::Result<Self> {
            Ok(Self(Arc::new(Mutex::new(Vec::new()))))
        }
    }
    impl furnace_rs_core::Injector for PreflightPermit {
        type Dependencies = ();
        async fn inject((): ()) -> furnace_rs_core::Result<Self> {
            Ok(Self(false))
        }
    }

    fn preflight_controller_type_id() -> TypeId {
        TypeId::of::<PreflightController>()
    }

    fn preflight_registrar(
        router: axum::Router,
        context: &crate::__private::RouterBuildContext<'_>,
        routes: &mut crate::__private::ValidatedRouteIter<'_>,
    ) -> furnace_rs_core::Result<axum::Router> {
        if !context.application().resolve::<PreflightPermit>()?.0 {
            return Err(Error::new(Diagnostic::new(
                furnace_rs_core::FURNACE003,
                "test preflight rejected",
                "the registrar has not been permitted",
            )));
        }
        context
            .application()
            .resolve::<RouterPreflightEvents>()?
            .0
            .lock()
            .unwrap()
            .push("router_preflight");
        let Some(path) = routes.next(HttpMethod::Get, "health")? else {
            routes.finish()?;
            return Ok(router);
        };
        routes.finish()?;
        Ok(router.route(path, axum::routing::get(|| async { "ok" })))
    }

    furnace_rs_core::__private::inventory::submit! {
        crate::ControllerDescriptor::new("test::PreflightController", preflight_controller_type_id, SourceLocation::new(file!(), line!(), column!()), crate::SealDefinition::default)
    }

    furnace_rs_core::__private::inventory::submit! {
        ControllerEndpointDescriptor::new("server_tests::PreflightController", preflight_controller_type_id, SourceLocation::new(file!(), line!(), column!()),
            &[RouteDescriptor::new(
                    HttpMethod::Get,
                    "",
                    "/health",
                    "/health",
                    "health",
                    SourceLocation::new(file!(), line!(), column!()),
                )],
            preflight_registrar,
        )
        .with_namespace(module_path!())
    }

    struct RecordingHook {
        events: Arc<Mutex<Vec<&'static str>>>,
        fail_shutdown: bool,
    }

    impl LifecycleHook for RecordingHook {
        fn name(&self) -> &str {
            "server-test"
        }

        fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
            Box::pin(async move {
                STARTS.fetch_add(1, Ordering::SeqCst);
                self.events.lock().unwrap().push("lifecycle_start");
                Ok(())
            })
        }

        fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
            Box::pin(async move {
                self.events.lock().unwrap().push("lifecycle_stop");
                if self.fail_shutdown {
                    Err(test_core_error("shutdown failed"))
                } else {
                    Ok(())
                }
            })
        }
    }

    fn test_core_error(message: &str) -> Error {
        Error::new(Diagnostic::new(FURNACE020, "server test failure", message))
    }

    async fn application(
        events: Arc<Mutex<Vec<&'static str>>>,
        preflight_permitted: bool,
        fail_shutdown: bool,
    ) -> Furnace {
        let mut builder = Furnace::builder();
        builder.root::<ServerTestApp>().unwrap();
        let router_preflight_events = Arc::clone(&events);
        builder.lifecycle_hook(RecordingHook {
            events,
            fail_shutdown,
        });
        builder
            .provide(RouterPreflightEvents(router_preflight_events))
            .unwrap();
        builder.provide(PreflightController).unwrap();
        builder
            .provide(PreflightPermit(preflight_permitted))
            .unwrap();
        builder.build().await.unwrap()
    }

    fn address() -> SocketAddr {
        SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
    }

    fn automatic_standard_builder<M: Cauldron>(host: &str) -> furnace_rs_core::FurnaceBuilder {
        let config = ConfigBuilder::new()
            .source(MapSource::new(
                "test",
                [("server.host", host), ("server.port", "3000")],
            ))
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config);
        builder.root::<M>().unwrap();
        assert!(
            builder.__auto_configuration_input(
                SERVER_AUTO_CONFIGURATION_ID,
                HttpRuntimeMode::Automatic,
            )
        );
        assert!(builder.__auto_configuration_input(
            CORS_AUTO_CONFIGURATION_ID,
            HttpRuntimeMode::Automatic,
        ));
        builder
    }

    fn automatic_report<'a>(
        application: &'a Furnace,
        identifier: &str,
    ) -> &'a furnace_rs_core::AutoConfigurationReport {
        application
            .auto_configurations()
            .iter()
            .find(|report| report.identifier() == identifier)
            .expect("the automatic HTTP configuration must be registered")
    }

    #[tokio::test]
    async fn standard_run_preparation_roots_and_configures_a_routed_application() {
        let directory = tempfile::tempdir().unwrap();

        let prepared = prepare_standard_run::<standard_run::routed::RoutedApp>(directory.path())
            .await
            .unwrap();

        assert_eq!(
            prepared
                .application
                .cauldron_graph()
                .unwrap()
                .root()
                .type_name(),
            std::any::type_name::<standard_run::routed::RoutedApp>(),
        );
        assert_eq!(prepared.binding.host(), "127.0.0.1");
        assert_eq!(prepared.binding.port(), 3000);
        assert_eq!(
            automatic_report(&prepared.application, SERVER_AUTO_CONFIGURATION_ID).status(),
            AutoConfigurationStatus::Active,
        );
        assert_eq!(prepared.route_count, 1);
        assert_eq!(
            prepared.startup_summary().to_string(),
            "\n  FURNACE · Application ready\n  ────────────────────────────\n  Server  http://127.0.0.1:3000\n  Routes  1\n\n  METHOD  PATH\n  ──────  ────\n  GET     /standard-run-health"
        );
    }

    #[test]
    fn startup_summary_formats_owned_binding_and_validated_route_count() {
        let summary = StartupSummary::new("api.internal".into(), 4321, 7, vec![]);

        assert_eq!(
            summary.to_string(),
            "\n  FURNACE · Application ready\n  ────────────────────────────\n  Server  http://api.internal:4321\n  Routes  7"
        );
    }

    #[test]
    fn startup_summary_lists_every_method_and_full_path() {
        let routes = [
            (HttpMethod::Get, "/", "/users"),
            (HttpMethod::Post, "/", "/users"),
            (HttpMethod::Put, "/:id", "/users/:id"),
            (HttpMethod::Patch, "/:id", "/users/:id"),
            (HttpMethod::Delete, "/:id", "/users/:id"),
        ]
        .into_iter()
        .map(|(method, path, full_path)| {
            RouteDescriptor::new(
                method,
                "/users",
                path,
                full_path,
                "handler",
                SourceLocation::new("routes.rs", 1, 1),
            )
        })
        .collect();
        let summary = StartupSummary::new("api.internal".into(), 4321, 5, routes);

        assert_eq!(
            summary.to_string(),
            "\n  FURNACE · Application ready\n  ────────────────────────────\n  Server  http://api.internal:4321\n  Routes  5\n\n  METHOD  PATH\n  ──────  ────\n  GET     /users\n  POST    /users\n  PUT     /users/:id\n  PATCH   /users/:id\n  DELETE  /users/:id"
        );
    }

    #[tokio::test]
    async fn standard_run_preparation_rejects_roots_without_reachable_routes() {
        let directory = tempfile::tempdir().unwrap();

        let error =
            match prepare_standard_run::<standard_run::empty::EmptyApp>(directory.path()).await {
                Ok(_) => panic!("a root without routes must not be runnable"),
                Err(error) => error,
            };

        match error {
            HttpRuntimeError::Bootstrap(error) => {
                assert_eq!(error.code(), FURNACE031);
                assert!(error.to_string().contains("no runnable HTTP route"));
            }
            other => panic!("expected bootstrap error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn standard_run_preparation_redacts_malformed_conventional_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let sentinel = "postgres://secret.example/standard-run";
        std::fs::write(
            directory.path().join("furnace.toml"),
            format!("[server\nport = \"{sentinel}\"\n"),
        )
        .unwrap();

        let error =
            match prepare_standard_run::<standard_run::routed::RoutedApp>(directory.path()).await {
                Ok(_) => panic!("malformed conventional configuration must fail preparation"),
                Err(error) => error,
            };

        match error {
            HttpRuntimeError::Bootstrap(error) => {
                assert_eq!(error.code(), FURNACE020);
                assert!(!error.to_string().contains(sentinel));
            }
            other => panic!("expected bootstrap error, got {other:?}"),
        }
    }

    #[cfg(feature = "jwt")]
    #[tokio::test]
    async fn standard_run_preparation_ignores_unreachable_jwt_requirements() {
        let directory = tempfile::tempdir().unwrap();

        let prepared = prepare_standard_run::<standard_run::routed::RoutedApp>(directory.path())
            .await
            .unwrap();

        assert_eq!(
            automatic_report(&prepared.application, "furnace.common.passport.jwt").status(),
            AutoConfigurationStatus::Skipped,
        );
    }

    #[tokio::test]
    async fn standard_run_bind_failure_starts_then_stops_lifecycle_once() {
        let _guard = TEST_LOCK.lock().await;
        STARTS.store(0, Ordering::SeqCst);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut builder =
            automatic_standard_builder::<standard_run::routed::RoutedApp>("api.internal");
        builder.lifecycle_hook(RecordingHook {
            events: Arc::clone(&events),
            fail_shutdown: false,
        });
        let prepared = prepare_standard_application(builder.build().await.unwrap()).unwrap();
        let binder_events = Arc::clone(&events);
        let binder = move |(host, port): (String, u16)| async move {
            assert_eq!((host.as_str(), port), ("api.internal", 3000));
            binder_events.lock().unwrap().push("bind");
            Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "unavailable",
            ))
        };

        let error = serve_prepared(prepared, binder, async {})
            .await
            .unwrap_err();

        assert!(matches!(error, HttpRuntimeError::Bind(_)));
        assert_eq!(STARTS.load(Ordering::SeqCst), 1);
        assert_eq!(
            *events.lock().unwrap(),
            ["lifecycle_start", "bind", "lifecycle_stop"]
        );
    }

    #[tokio::test]
    async fn standard_run_bind_and_shutdown_failures_are_both_retained() {
        let _guard = TEST_LOCK.lock().await;
        STARTS.store(0, Ordering::SeqCst);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut builder =
            automatic_standard_builder::<standard_run::routed::RoutedApp>("api.internal");
        builder.lifecycle_hook(RecordingHook {
            events: Arc::clone(&events),
            fail_shutdown: true,
        });
        let prepared = prepare_standard_application(builder.build().await.unwrap()).unwrap();
        let binder_events = Arc::clone(&events);
        let binder = move |(host, port): (String, u16)| async move {
            assert_eq!((host.as_str(), port), ("api.internal", 3000));
            binder_events.lock().unwrap().push("bind");
            Err(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "unavailable",
            ))
        };

        let error = serve_prepared(prepared, binder, async {})
            .await
            .unwrap_err();

        match error {
            HttpRuntimeError::OperationAndShutdown {
                operation,
                shutdown,
            } => {
                assert!(matches!(*operation, HttpRuntimeError::Bind(_)));
                assert_eq!(shutdown.code(), FURNACE011);
            }
            other => panic!("expected combined failure, got {other:?}"),
        }
        assert_eq!(STARTS.load(Ordering::SeqCst), 1);
        assert_eq!(
            *events.lock().unwrap(),
            ["lifecycle_start", "bind", "lifecycle_stop"]
        );
    }

    #[test]
    fn ipv6_automatic_server_binding_uses_a_host_port_tuple() {
        let config = ConfigBuilder::new()
            .source(MapSource::new(
                "test",
                [("server.host", "::1"), ("server.port", "3000")],
            ))
            .build()
            .unwrap();
        let binding = ServerBinding::from_config(&config).unwrap();
        let address = binding.address();

        accepts_tokio_socket_address(address);
        assert_eq!(address, ("::1", 3000));
        assert_eq!(format!("{binding:?}"), "[REDACTED]");
    }

    fn accepts_tokio_socket_address(_: impl tokio::net::ToSocketAddrs) {}

    #[tokio::test]
    async fn preflight_failure_prevents_lifecycle_start_and_bind() {
        let _guard = TEST_LOCK.lock().await;
        STARTS.store(0, Ordering::SeqCst);
        BINDS.store(0, Ordering::SeqCst);
        let events = Arc::new(Mutex::new(Vec::new()));
        let application = application(Arc::clone(&events), false, false).await;
        let binder = |address| async move {
            BINDS.fetch_add(1, Ordering::SeqCst);
            tokio::net::TcpListener::bind(address).await
        };

        let error = serve_with(application, address(), binder, async {})
            .await
            .unwrap_err();

        assert!(matches!(error, HttpRuntimeError::Bootstrap(_)));
        assert_eq!(STARTS.load(Ordering::SeqCst), 0);
        assert_eq!(BINDS.load(Ordering::SeqCst), 0);
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn raw_router_preflight_precedes_lifecycle_binding_and_graceful_shutdown() {
        let _guard = TEST_LOCK.lock().await;
        STARTS.store(0, Ordering::SeqCst);
        let events = Arc::new(Mutex::new(Vec::new()));
        let application = application(Arc::clone(&events), true, false).await;
        let router = crate::build_router(&application).unwrap();
        let binder_events = Arc::clone(&events);
        let binder = move |address| async move {
            binder_events.lock().unwrap().push("bind");
            tokio::net::TcpListener::bind(address).await
        };
        let shutdown_events = Arc::clone(&events);
        let shutdown = async move {
            shutdown_events.lock().unwrap().push("serve");
        };

        serve_router_with(application, router, address(), binder, shutdown)
            .await
            .unwrap();

        assert_eq!(
            *events.lock().unwrap(),
            [
                "router_preflight",
                "lifecycle_start",
                "bind",
                "serve",
                "lifecycle_stop",
            ]
        );
    }

    #[tokio::test]
    async fn bind_failure_still_attempts_shutdown() {
        let _guard = TEST_LOCK.lock().await;
        let events = Arc::new(Mutex::new(Vec::new()));
        let application = application(Arc::clone(&events), true, false).await;
        let binder_events = Arc::clone(&events);
        let binder = move |_| async move {
            binder_events.lock().unwrap().push("bind");
            Err(io::Error::new(io::ErrorKind::AddrInUse, "occupied"))
        };

        let error = serve_with(application, address(), binder, async {})
            .await
            .unwrap_err();

        assert!(matches!(error, HttpRuntimeError::Bind(_)));
        assert_eq!(
            *events.lock().unwrap(),
            [
                "router_preflight",
                "lifecycle_start",
                "bind",
                "lifecycle_stop"
            ]
        );
    }

    #[tokio::test]
    async fn operational_and_shutdown_failures_are_both_retained() {
        let _guard = TEST_LOCK.lock().await;
        let events = Arc::new(Mutex::new(Vec::new()));
        let application = application(Arc::clone(&events), true, true).await;
        let binder = |_| async {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "bind denied",
            ))
        };

        let error = serve_with(application, address(), binder, async {})
            .await
            .unwrap_err();

        match error {
            HttpRuntimeError::OperationAndShutdown {
                operation,
                shutdown,
            } => {
                assert!(matches!(*operation, HttpRuntimeError::Bind(_)));
                assert_eq!(shutdown.code(), FURNACE011);
                let source = std::error::Error::source(&shutdown)
                    .unwrap()
                    .downcast_ref::<Error>()
                    .unwrap();
                assert_eq!(source.code(), FURNACE020);
            }
            other => panic!("expected combined failure, got {other:?}"),
        }
        assert_eq!(
            *events.lock().unwrap(),
            ["router_preflight", "lifecycle_start", "lifecycle_stop"]
        );
    }

    #[tokio::test]
    async fn invalid_cors_configuration_prevents_lifecycle_start() {
        let _guard = TEST_LOCK.lock().await;
        STARTS.store(0, Ordering::SeqCst);
        let events = Arc::new(Mutex::new(Vec::new()));
        let config = ConfigBuilder::new()
            .source(
                MapSource::new("test", std::iter::empty::<(&str, &str)>())
                    .with_string_array("server.cors.origins", ["https://app.example.com"]),
            )
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config);
        builder.root::<ServerTestApp>().unwrap();
        builder.lifecycle_hook(RecordingHook {
            events: Arc::clone(&events),
            fail_shutdown: false,
        });

        assert!(builder.build().await.is_err());
        assert_eq!(STARTS.load(Ordering::SeqCst), 0);
        assert!(events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn explicit_raw_router_ignores_automatic_binding_keys_and_applies_cors() {
        let _guard = TEST_LOCK.lock().await;
        let events = Arc::new(Mutex::new(Vec::new()));
        let config = ConfigBuilder::new()
            .source(
                MapSource::new(
                    "test",
                    [
                        ("server.host", "private\ninvalid-host"),
                        ("server.port", "0"),
                    ],
                )
                .with_string_array("server.cors.origins", ["https://app.example.com"])
                .with_string_array("server.cors.methods", ["GET"]),
            )
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config);
        builder.root::<raw_router::App>().unwrap();
        builder.lifecycle_hook(RecordingHook {
            events: Arc::clone(&events),
            fail_shutdown: false,
        });
        let application = builder.build().await.unwrap();
        let router = axum::Router::new().route("/native", axum::routing::get(|| async { "ok" }));
        let (bound_address_sender, bound_address_receiver) = tokio::sync::oneshot::channel();
        let binder_events = Arc::clone(&events);
        let binder = move |address: SocketAddr| async move {
            assert_eq!(address.port(), 0);
            binder_events.lock().unwrap().push("bind");
            let listener = TcpListener::bind(address).await?;
            bound_address_sender
                .send(listener.local_addr().unwrap())
                .unwrap();
            Ok(listener)
        };
        let shutdown_events = Arc::clone(&events);
        let shutdown = async move {
            let bound_address = bound_address_receiver.await.unwrap();
            let response = tokio::task::spawn_blocking(move || {
                let mut stream = std::net::TcpStream::connect(bound_address)?;
                stream.write_all(
                    b"GET /native HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: https://app.example.com\r\nConnection: close\r\n\r\n",
                )?;
                let mut response = String::new();
                stream.read_to_string(&mut response)?;
                Ok::<_, io::Error>(response)
            })
            .await
            .unwrap()
            .unwrap();
            assert!(response.starts_with("HTTP/1.1 200"));
            assert!(response.contains("access-control-allow-origin: https://app.example.com"));
            shutdown_events.lock().unwrap().push("serve");
        };

        serve_router_with(application, router, address(), binder, shutdown)
            .await
            .unwrap();

        assert_eq!(
            *events.lock().unwrap(),
            ["lifecycle_start", "bind", "serve", "lifecycle_stop"]
        );
    }
}
