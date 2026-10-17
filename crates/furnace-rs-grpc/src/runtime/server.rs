//! Supervised native protobuf dispatch on HTTP/2 connections.
use furnace_rs_common::runtime::{ApplicationRuntimeError, RuntimeTasks};
use std::future::Future;
use tokio::{net::TcpListener, sync::watch};
use tonic::service::Routes;
use tower::ServiceExt;
#[derive(Clone)]
struct Executor(RuntimeTasks);
impl<F> hyper::rt::Executor<F> for Executor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    /// Spawns a connection task on the runtime-owned Tokio handle.
    fn execute(&self, future: F) {
        self.0.spawn(async move {
            let _ = future.await;
        });
    }
}
#[cfg(feature = "tls")]
pub(super) type Tls = Option<tokio_rustls::TlsAcceptor>;
#[cfg(not(feature = "tls"))]
pub(super) type Tls = ();
/// Loads and validates server TLS identity before accepting connections.
pub(super) async fn prepare_tls(config: &crate::GrpcServerConfig) -> furnace_rs_core::Result<Tls> {
    #[cfg(feature = "tls")]
    {
        use rustls::{
            self,
            pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
        };
        let Some(options) = config.tls() else {
            return Ok(None);
        };
        let certificate = tokio::fs::read(options.certificate())
            .await
            .map_err(|_| tls_error())?;
        let key = tokio::fs::read(options.private_key())
            .await
            .map_err(|_| tls_error())?;
        let certificates = CertificateDer::pem_slice_iter(&certificate)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| tls_error())?;
        let key = PrivateKeyDer::from_pem_slice(&key).map_err(|_| tls_error())?;
        let mut tls = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| tls_error())?
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(|_| tls_error())?;
        tls.alpn_protocols = vec![b"h2".to_vec()];
        Ok(Some(tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(
            tls,
        ))))
    }
    #[cfg(not(feature = "tls"))]
    {
        let _ = config;
        Ok(())
    }
}
#[cfg(feature = "tls")]
/// Creates a fixed diagnostic for invalid server TLS material.
fn tls_error() -> furnace_rs_core::Error {
    furnace_rs_core::Error::new(furnace_rs_core::Diagnostic::new(
        crate::FURNACE151,
        "gRPC TLS preparation failed",
        "configured certificate material could not establish verified TLS",
    ))
}
/// Accepts connections, updates health, and drains active calls on shutdown.
pub(super) async fn serve(
    listener: TcpListener,
    routes: Routes,
    mut shutdown: watch::Receiver<bool>,
    tasks: RuntimeTasks,
    tls: Tls,
    handshake_timeout: std::time::Duration,
) -> Result<(), ApplicationRuntimeError> {
    loop {
        if *shutdown.borrow_and_update() {
            break;
        }
        tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            accepted = listener.accept() => {
                let (stream, peer) = accepted.map_err(|source| ApplicationRuntimeError::TransportServe { transport: "grpc", source: Box::new(source) })?;
                let info = tonic::transport::server::TcpConnectInfo { local_addr: stream.local_addr().ok(), remote_addr: Some(peer) };
                let routes = routes.clone();
                let stopped = shutdown.clone();
                let executor = Executor(tasks.clone());
                #[cfg(feature = "tls")]
                let tls = tls.clone();
                tasks.spawn(async move {
                    #[cfg(feature = "tls")]
                    if let Some(tls) = tls {
                        let mut cancelled = stopped.clone();
                        let stream = tokio::select! {
                            biased;
                            _ = cancelled.changed() => return,
                            result = tokio::time::timeout(handshake_timeout, tls.accept(stream)) => match result { Ok(Ok(stream)) => stream, _ => return },
                        };
                        connection(stream, routes, info, stopped, executor).await;
                        return;
                    }
                    #[cfg(not(feature = "tls"))]
                    let _ = (tls, handshake_timeout);
                    connection(stream, routes, info, stopped, executor).await;
                });
            }
        }
    }
    Ok(())
}
/// Serves one accepted connection with its peer information and routes.
async fn connection<IO>(
    stream: IO,
    routes: Routes,
    info: tonic::transport::server::TcpConnectInfo,
    mut shutdown: watch::Receiver<bool>,
    executor: Executor,
) where
    IO: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let service =
        hyper::service::service_fn(move |mut request: http::Request<hyper::body::Incoming>| {
            request.extensions_mut().insert(info.clone());
            // Match native Tonic establishment deadlines; never wrap the response body.
            let timeout = request
                .headers()
                .get("grpc-timeout")
                .and_then(|value| value.to_str().ok())
                .and_then(crate::grpc::parse_timeout);
            let dispatch = routes.clone().oneshot(request);
            async move {
                match timeout {
                    Some(timeout) => match tokio::time::timeout(timeout, dispatch).await {
                        Ok(response) => response,
                        Err(_) => {
                            Ok(tonic::Status::deadline_exceeded("deadline exceeded").into_http())
                        }
                    },
                    None => dispatch.await,
                }
            }
        });
    let mut builder = hyper::server::conn::http2::Builder::new(executor);
    builder.timer(hyper_util::rt::TokioTimer::new());
    let connection = builder.serve_connection(hyper_util::rt::TokioIo::new(stream), service);
    tokio::pin!(connection);
    if *shutdown.borrow_and_update() {
        connection.as_mut().graceful_shutdown();
        let _ = connection.await;
        return;
    }
    tokio::select! {
        _ = &mut connection => {},
        _ = shutdown.changed() => { connection.as_mut().graceful_shutdown(); let _ = connection.await; }
    }
}
