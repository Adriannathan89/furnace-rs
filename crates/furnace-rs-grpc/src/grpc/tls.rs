//! Native TLS preparation; static configuration parsing never calls these helpers.
use super::{GrpcClientConfig, GrpcServerConfig};
use furnace_rs_core::Result;
#[cfg(feature = "tls")]
use furnace_rs_core::{Diagnostic, Error};
/// Prepares native client TLS roots and server-name validation without connecting.
#[allow(clippy::result_large_err)]
pub async fn client_endpoint(config: &GrpcClientConfig) -> Result<tonic::transport::Endpoint> {
    let endpoint = config.endpoint().connect_timeout(config.connect_timeout());
    #[cfg(feature = "tls")]
    if let Some(options) = config.tls() {
        let mut tls =
            tonic::transport::ClientTlsConfig::new().timeout(config.tls_handshake_timeout());
        if !options.custom_ca_only() {
            tls = tls.with_webpki_roots();
        }
        if let Some(domain) = options.domain_name() {
            tls = tls.domain_name(domain);
        }
        for path in options.custom_ca_files() {
            let pem = tokio::fs::read(path).await.map_err(|_| tls_error())?;
            tls = tls.ca_certificate(tonic::transport::Certificate::from_pem(pem));
        }
        return endpoint.tls_config(tls).map_err(|_| tls_error());
    }
    Ok(endpoint)
}
/// Loads a server identity and installs native verified TLS with a handshake timeout.
#[allow(clippy::result_large_err)]
pub async fn configure_grpc_server(
    server: tonic::transport::Server,
    config: &GrpcServerConfig,
) -> Result<tonic::transport::Server> {
    #[cfg(feature = "tls")]
    if let Some(options) = config.tls() {
        let certificate = tokio::fs::read(options.certificate())
            .await
            .map_err(|_| tls_error())?;
        let key = tokio::fs::read(options.private_key())
            .await
            .map_err(|_| tls_error())?;
        let identity = tonic::transport::Identity::from_pem(certificate, key);
        return server
            .tls_config(
                tonic::transport::ServerTlsConfig::new()
                    .identity(identity)
                    .timeout(config.tls_handshake_timeout()),
            )
            .map_err(|_| tls_error());
    }
    #[cfg(not(feature = "tls"))]
    let _ = config;
    Ok(server)
}
#[cfg(feature = "tls")]
fn tls_error() -> Error {
    Error::new(Diagnostic::new(
        super::FURNACE151,
        "gRPC TLS preparation failed",
        "configured certificate material could not establish verified TLS",
    ))
}
