//! Native TLS uses verified server identity and preserves WebSocket upgrades.
#![cfg(feature = "tls")]
mod support;
use furnace_rs_core::{Injector, cauldron};
use furnace_rs_socket::{SocketContext, SocketResult, socket_gateway};
use futures_util::StreamExt;
struct Hello;
impl Injector for Hello {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[socket_gateway(path = "/hello", mode = "events")]
impl Hello {
    #[on_connect]
    async fn hello(&self, c: SocketContext) -> SocketResult<()> {
        c.emit("hello", serde_json::json!(true)).await
    }
}
#[cauldron(root,socket_gateways=[Hello])]
struct App;
#[tokio::test]
async fn native_tls_requires_trusted_identity_and_supports_upgrade() {
    let identity = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("cert.pem");
    let key = dir.path().join("key.pem");
    std::fs::write(&cert, identity.cert.pem()).unwrap();
    std::fs::write(&key, identity.signing_key.serialize_pem()).unwrap();
    let server = support::Server::configured::<App>(&[
        ("socket.tls.certificate", cert.to_str().unwrap()),
        ("socket.tls.private_key", key.to_str().unwrap()),
    ])
    .await;
    let url = format!("wss://localhost:{}/hello", server.address.port());
    assert!(tokio_tungstenite::connect_async(&url).await.is_err());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(identity.cert.der().clone()).unwrap();
    let config = rustls::ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let (mut client, _) = tokio_tungstenite::connect_async_tls_with_config(
        &url,
        None,
        false,
        Some(tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(
            config,
        ))),
    )
    .await
    .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&client.next().await.unwrap().unwrap().into_text().unwrap()).unwrap();
    assert_eq!(v["event"], "hello");
    client.close(None).await.unwrap();
    server.stop().await;
}
