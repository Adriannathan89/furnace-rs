//! Native TLS verifies both server name and trust chain with local certificates.
#![cfg(feature = "tls")]
#[path = "support/tls.rs"]
mod certificates;
use certificates::Certificates;
use furnace_grpc_test_support::pb;
use furnace_rs_core::{Config, ConfigBuilder, MapSource};
use furnace_rs_grpc::{
    GrpcClientConfig, GrpcServerConfig, client_endpoint, configure_grpc_server, tonic,
};
use std::time::Duration;
#[derive(Clone)]
struct Service;
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Service {
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
}
fn config(values: impl IntoIterator<Item = (String, String)>) -> Config {
    ConfigBuilder::new()
        .source(MapSource::new("tls-test", values))
        .build()
        .unwrap()
}
fn server_config(certs: &Certificates) -> Config {
    config([
        (
            "grpc.server.tls.certificate".into(),
            certs.certificate.to_str().unwrap().into(),
        ),
        (
            "grpc.server.tls.private_key".into(),
            certs.key.to_str().unwrap().into(),
        ),
    ])
}
#[derive(Clone, Copy, PartialEq)]
enum Trust {
    None,
    ServerCa,
    UnrelatedCa,
}
async fn trusted_call(custom_only: bool, name: &str, trust: Trust) -> bool {
    let certs = Certificates::new();
    let server_options = GrpcServerConfig::parse(&server_config(&certs)).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut native = configure_grpc_server(tonic::transport::Server::builder(), &server_options)
        .await
        .unwrap();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        native
            .add_service(pb::hidden_server::HiddenServer::new(Service))
            .serve_with_incoming_shutdown(
                tonic::codegen::tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = signal.await;
                },
            )
            .await
            .unwrap();
    });
    let mut builder = ConfigBuilder::new().source(MapSource::new(
        "tls-test",
        [
            ("grpc.clients.remote.endpoint", format!("https://{address}")),
            ("grpc.clients.remote.tls.domain_name", name.into()),
            ("grpc.clients.remote.connect_timeout_ms", "1000".into()),
        ],
    ));
    let unrelated = Certificates::new();
    let ca = if trust == Trust::UnrelatedCa {
        &unrelated.ca
    } else {
        &certs.ca
    };
    if trust != Trust::None {
        builder = builder.source(
            MapSource::new(
                "ca",
                [(
                    "grpc.clients.remote.tls.custom_ca_only",
                    custom_only.to_string(),
                )],
            )
            .with_string_array(
                "grpc.clients.remote.tls.custom_ca_files",
                [ca.to_str().unwrap()],
            ),
        );
    }
    let client_options = GrpcClientConfig::parse(&builder.build().unwrap(), "remote").unwrap();
    let result = client_endpoint(&client_options)
        .await
        .unwrap()
        .connect()
        .await;
    let success = if let Ok(channel) = result {
        let mut client = pb::hidden_client::HiddenClient::new(channel);
        client
            .secret(pb::Item { value: 7 })
            .await
            .map(|response| response.into_inner().value == 7)
            .unwrap_or(false)
    } else {
        false
    };
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    success
}
#[tokio::test]
async fn trusted_custom_ca_and_additive_webpki_roots() {
    assert!(trusted_call(false, "localhost", Trust::ServerCa).await);
}
#[tokio::test]
async fn custom_ca_only_roots_are_isolated() {
    assert!(trusted_call(true, "localhost", Trust::ServerCa).await);
}
#[tokio::test]
async fn wrong_hostname_is_rejected() {
    assert!(!trusted_call(true, "wrong.invalid", Trust::ServerCa).await);
}
#[tokio::test]
async fn untrusted_ca_is_rejected() {
    assert!(!trusted_call(false, "localhost", Trust::None).await);
}
#[tokio::test]
async fn mismatched_server_key_is_rejected_and_redacted() {
    let certs = Certificates::new();
    let other = Certificates::new();
    let options = GrpcServerConfig::parse(&config([
        (
            "grpc.server.tls.certificate".into(),
            certs.certificate.to_str().unwrap().into(),
        ),
        (
            "grpc.server.tls.private_key".into(),
            other.key.to_str().unwrap().into(),
        ),
    ]))
    .unwrap();
    let error = configure_grpc_server(tonic::transport::Server::builder(), &options)
        .await
        .unwrap_err();
    assert!(!format!("{error:?} {error}").contains("PRIVATE KEY"));
}

#[tokio::test]
async fn tls_handshake_is_bounded_when_peer_never_replies() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("https://{}", listener.local_addr().unwrap());
    let (accepted, receive) = tokio::sync::oneshot::channel();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        use tokio::io::AsyncReadExt;
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut record = [0; 3];
        tokio::time::timeout(Duration::from_secs(1), socket.read_exact(&mut record))
            .await
            .unwrap()
            .expect("HTTPS must send a TLS ClientHello without an explicit TLS table");
        accepted.send(record).unwrap();
        let _ = signal.await;
        drop(socket);
    });
    let options = GrpcClientConfig::parse(
        &config([
            ("grpc.clients.remote.endpoint".into(), endpoint),
            (
                "grpc.clients.remote.tls_handshake_timeout_ms".into(),
                "50".into(),
            ),
            (
                "grpc.clients.remote.connect_timeout_ms".into(),
                "2000".into(),
            ),
        ]),
        "remote",
    )
    .unwrap();
    let endpoint = client_endpoint(&options).await.unwrap();
    let started = std::time::Instant::now();
    let connect = tokio::spawn(async move { endpoint.connect().await });
    let record = receive.await.unwrap();
    assert_eq!(&record[..2], &[22, 3], "expected a TLS handshake record");
    assert!(
        tokio::time::timeout(Duration::from_secs(1), connect)
            .await
            .expect("TLS handshake outlived configured timeout")
            .unwrap()
            .is_err()
    );
    assert!(started.elapsed() >= Duration::from_millis(50));
    stop.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn custom_ca_only_rejects_a_different_private_ca() {
    assert!(!trusted_call(true, "localhost", Trust::UnrelatedCa).await);
}
#[tokio::test]
async fn plaintext_helpers_preserve_native_transport() {
    let server_options = GrpcServerConfig::parse(&Config::default()).unwrap();
    let mut native = configure_grpc_server(tonic::transport::Server::builder(), &server_options)
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        native
            .add_service(pb::hidden_server::HiddenServer::new(Service))
            .serve_with_incoming_shutdown(
                tonic::codegen::tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = signal.await;
                },
            )
            .await
            .unwrap();
    });
    let options = GrpcClientConfig::parse(
        &config([("grpc.clients.remote.endpoint".into(), endpoint)]),
        "remote",
    )
    .unwrap();
    let channel = client_endpoint(&options)
        .await
        .unwrap()
        .connect()
        .await
        .unwrap();
    let mut client = pb::hidden_client::HiddenClient::new(channel);
    assert_eq!(
        client
            .secret(pb::Item { value: 7 })
            .await
            .unwrap()
            .into_inner()
            .value,
        7
    );
    drop(client);
    stop.send(()).unwrap();
    task.await.unwrap();
}
