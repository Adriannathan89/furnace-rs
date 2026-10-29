//! Server identity validation before listener readiness.
#[cfg(feature = "tls")]
pub(super) type Tls = Option<tokio_rustls::TlsAcceptor>;

#[cfg(not(feature = "tls"))]
pub(super) type Tls = ();

#[allow(clippy::result_large_err)]
pub(super) async fn prepare(config: &crate::SocketServerConfig) -> furnace_rs_core::Result<Tls> {
    #[cfg(feature = "tls")]
    {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
        let Some(cert) = &config.certificate else {
            return Ok(None);
        };
        let fail = || crate::config::invalid("socket.tls");
        let bytes = tokio::fs::read(cert).await.map_err(|_| fail())?;
        let key = tokio::fs::read(config.private_key.as_ref().ok_or_else(fail)?)
            .await
            .map_err(|_| fail())?;
        let chain = CertificateDer::pem_slice_iter(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| fail())?;
        let key = PrivateKeyDer::from_pem_slice(&key).map_err(|_| fail())?;
        let mut server = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| fail())?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|_| fail())?;
        server.alpn_protocols = vec![b"http/1.1".to_vec()];

        Ok(Some(tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(
            server,
        ))))
    }

    #[cfg(not(feature = "tls"))]
    {
        let _ = (&config.certificate, &config.private_key);

        Ok(())
    }
}
