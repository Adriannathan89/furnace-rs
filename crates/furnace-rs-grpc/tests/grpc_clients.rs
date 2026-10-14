//! Named outbound clients retain native protobuf APIs and prepare credentials locally.
use furnace_grpc_test_support::pb;
use furnace_rs_core::{ConfigBuilder, Furnace, Injector, MapSource, Secret, cauldron};
use furnace_rs_grpc::{
    GrpcClientError, GrpcClientResult, GrpcTokenProvider, grpc_client, set_grpc_bearer_token, tonic,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
#[derive(Clone, Default)]
struct Tokens {
    calls: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
    stall: Arc<AtomicBool>,
    entered: Arc<tokio::sync::Notify>,
}
impl Injector for Tokens {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self::default())
    }
}
impl GrpcTokenProvider for Tokens {
    async fn token(&self) -> GrpcClientResult<Secret<String>> {
        let count = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if self.stall.load(Ordering::SeqCst) {
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        if self.fail.load(Ordering::SeqCst) {
            return Err(GrpcClientError::token_provider(tonic::Status::internal(
                "private-token-sentinel",
            )));
        }
        Ok(Secret::new(format!("provider-{count}")))
    }
}
#[grpc_client(name = "alpha", client = pb::hidden_client::HiddenClient, token_provider = Tokens)]
struct Alpha;
#[grpc_client(name = "beta", client = pb::hidden_client::HiddenClient)]
struct Beta;
#[cauldron(providers = [Tokens, Alpha, Beta])]
struct App;
type RecordedHeaders = Vec<(Option<String>, Option<String>)>;
#[derive(Clone, Default)]
struct Remote {
    calls: Arc<AtomicUsize>,
    headers: Arc<Mutex<RecordedHeaders>>,
    value: u64,
    connections: Arc<AtomicUsize>,
    accepted: Arc<tokio::sync::Notify>,
    barrier: Option<Arc<tokio::sync::Barrier>>,
    hold: Option<Arc<tokio::sync::Notify>>,
}
#[tonic::async_trait]
impl pb::hidden_server::Hidden for Remote {
    async fn secret(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.headers.lock().unwrap().push((
            request
                .metadata()
                .get("authorization")
                .map(|v| v.to_str().unwrap().into()),
            request
                .metadata()
                .get("grpc-timeout")
                .map(|v| v.to_str().unwrap().into()),
        ));
        if let Some(hold) = &self.hold {
            hold.notified().await;
        }
        if let Some(barrier) = &self.barrier {
            barrier.wait().await;
        }
        Ok(tonic::Response::new(pb::Item { value: self.value }))
    }
}
struct Running {
    remote: Remote,
    endpoint: String,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}
impl Running {
    async fn start(value: u64) -> Self {
        Self::start_remote(Remote {
            value,
            ..Remote::default()
        })
        .await
    }
    async fn start_remote(remote: Remote) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let service = pb::hidden_server::HiddenServer::new(remote.clone());
        let (stop, signal) = tokio::sync::oneshot::channel();
        let connections = remote.connections.clone();
        let accepted = remote.accepted.clone();
        let incoming = tonic::codegen::tokio_stream::StreamExt::map(
            tonic::codegen::tokio_stream::wrappers::TcpListenerStream::new(listener),
            move |connection| {
                if connection.is_ok() {
                    connections.fetch_add(1, Ordering::SeqCst);
                    accepted.notify_one();
                }
                connection
            },
        );
        let task = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(service)
                .serve_with_incoming_shutdown(incoming, async {
                    let _ = signal.await;
                })
                .await
                .unwrap();
        });
        Self {
            remote,
            endpoint,
            stop,
            task,
        }
    }
    async fn stop(self) {
        self.stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap();
    }
}
async fn app(alpha: &str, beta: &str, extra: &[(&str, &str)]) -> Furnace {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "clients",
            [
                ("grpc.clients.alpha.endpoint", alpha),
                ("grpc.clients.beta.endpoint", beta),
            ]
            .into_iter()
            .chain(extra.iter().copied()),
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder.build().await.unwrap()
}
#[tokio::test]
async fn two_typed_clients_target_distinct_servers() {
    let first = Running::start(7).await;
    let second = Running::start(13).await;
    let mut app = app(&first.endpoint, &second.endpoint, &[]).await;
    app.start().await.unwrap();
    assert_eq!(
        app.context()
            .resolve::<Alpha>()
            .unwrap()
            .client()
            .secret(pb::Item { value: 0 })
            .await
            .unwrap()
            .into_inner()
            .value,
        7
    );
    assert_eq!(
        app.context()
            .resolve::<Beta>()
            .unwrap()
            .client()
            .secret(pb::Item { value: 0 })
            .await
            .unwrap()
            .into_inner()
            .value,
        13
    );
    app.shutdown().await.unwrap();
    first.stop().await;
    second.stop().await;
}
#[tokio::test]
async fn per_call_token_suppresses_provider() {
    let server = Running::start(7).await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    let tokens = app.context().resolve::<Tokens>().unwrap();
    let mut request = tonic::Request::new(pb::Item { value: 0 });
    set_grpc_bearer_token(&mut request, "explicit-token").unwrap();
    app.context()
        .resolve::<Alpha>()
        .unwrap()
        .client()
        .secret(request)
        .await
        .unwrap();
    assert_eq!(tokens.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        server.remote.headers.lock().unwrap()[0].0.as_deref(),
        Some("Bearer explicit-token")
    );
    server.stop().await;
}
#[tokio::test]
async fn provider_token_refreshed_per_call() {
    let server = Running::start(7).await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    let alpha = app.context().resolve::<Alpha>().unwrap();
    alpha.client().secret(pb::Item { value: 0 }).await.unwrap();
    alpha.client().secret(pb::Item { value: 0 }).await.unwrap();
    let headers = server.remote.headers.lock().unwrap().clone();
    assert_eq!(headers[0].0.as_deref(), Some("Bearer provider-1"));
    assert_eq!(headers[1].0.as_deref(), Some("Bearer provider-2"));
    server.stop().await;
}
#[tokio::test]
async fn duplicate_or_malformed_metadata_fails_without_provider_or_network() {
    let server = Running::start(7).await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    let alpha = app.context().resolve::<Alpha>().unwrap();
    for credentials in [
        vec!["Basic private-token-sentinel"],
        vec!["Bearer private:token"],
        vec!["Bearer one", "Bearer two"],
    ] {
        let mut request = tonic::Request::new(pb::Item { value: 0 });
        for credential in credentials {
            request
                .metadata_mut()
                .append("authorization", credential.parse().unwrap());
        }
        let error = alpha.client().secret(request).await.unwrap_err();
        assert!(!error.to_string().contains("private-token-sentinel"));
    }
    assert_eq!(
        app.context()
            .resolve::<Tokens>()
            .unwrap()
            .calls
            .load(Ordering::SeqCst),
        0
    );
    assert_eq!(server.remote.calls.load(Ordering::SeqCst), 0);
    server.stop().await;
}
#[tokio::test]
async fn provider_failure_never_calls_remote_handler() {
    let server = Running::start(7).await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    app.context()
        .resolve::<Tokens>()
        .unwrap()
        .fail
        .store(true, Ordering::SeqCst);
    let error = app
        .context()
        .resolve::<Alpha>()
        .unwrap()
        .client()
        .secret(pb::Item { value: 0 })
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("private-token-sentinel"));
    assert_eq!(server.remote.calls.load(Ordering::SeqCst), 0);
    server.stop().await;
}
#[tokio::test]
async fn anonymous_client_has_no_authorization() {
    let server = Running::start(7).await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    app.context()
        .resolve::<Beta>()
        .unwrap()
        .client()
        .secret(pb::Item { value: 0 })
        .await
        .unwrap();
    assert!(server.remote.headers.lock().unwrap()[0].0.is_none());
    server.stop().await;
}
#[tokio::test]
async fn per_call_deadline_overrides_client_default() {
    let server = Running::start(7).await;
    let app = app(
        &server.endpoint,
        &server.endpoint,
        &[("grpc.clients.beta.request_timeout_ms", "1000")],
    )
    .await;
    let mut request = tonic::Request::new(pb::Item { value: 0 });
    request.set_timeout(Duration::from_secs(3));
    let expected = request
        .metadata()
        .get("grpc-timeout")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    app.context()
        .resolve::<Beta>()
        .unwrap()
        .client()
        .secret(request)
        .await
        .unwrap();
    assert_eq!(
        server.remote.headers.lock().unwrap()[0].1.as_deref(),
        Some(expected.as_str())
    );
    server.stop().await;
}
#[tokio::test]
async fn lazy_unavailable_endpoint_does_not_block_startup() {
    let mut app = app("http://127.0.0.1:1", "http://127.0.0.1:1", &[]).await;
    app.start().await.unwrap();
    app.shutdown().await.unwrap();
}
#[tokio::test]
async fn eager_unavailable_endpoint_rolls_back_startup() {
    let mut app = app(
        "http://127.0.0.1:1",
        "http://127.0.0.1:1",
        &[
            ("grpc.clients.alpha.connect", "eager"),
            ("grpc.clients.alpha.connect_timeout_ms", "50"),
        ],
    )
    .await;
    assert!(app.start().await.is_err());
}

struct Observer {
    starts: Arc<AtomicUsize>,
    stops: Arc<AtomicUsize>,
    connections: Option<(Arc<AtomicUsize>, Arc<tokio::sync::Notify>)>,
}
impl furnace_rs_core::LifecycleHook for Observer {
    fn name(&self) -> &str {
        "readiness-observer"
    }
    fn start<'a>(
        &'a self,
        _: &'a furnace_rs_core::ApplicationContext,
    ) -> furnace_rs_core::LifecycleFuture<'a> {
        Box::pin(async move {
            if let Some((connections, accepted)) = &self.connections {
                if connections.load(Ordering::SeqCst) == 0 {
                    tokio::time::timeout(Duration::from_secs(3), accepted.notified())
                        .await
                        .expect("eager connection was never initiated before application hook");
                }
                assert!(
                    connections.load(Ordering::SeqCst) > 0,
                    "application hook ran before eager readiness"
                );
            }
            self.starts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn stop<'a>(
        &'a self,
        _: &'a furnace_rs_core::ApplicationContext,
    ) -> furnace_rs_core::LifecycleFuture<'a> {
        Box::pin(async move {
            self.stops.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}
#[tokio::test]
async fn eager_readiness_precedes_application_hooks() {
    let server = Running::start(7).await;
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [
                ("grpc.clients.alpha.endpoint", server.endpoint.as_str()),
                ("grpc.clients.beta.endpoint", server.endpoint.as_str()),
                ("grpc.clients.alpha.connect", "eager"),
            ],
        ))
        .build()
        .unwrap();
    let starts = Arc::new(AtomicUsize::new(0));
    let stops = Arc::new(AtomicUsize::new(0));
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder.lifecycle_hook(Observer {
        starts: starts.clone(),
        stops: stops.clone(),
        connections: Some((
            server.remote.connections.clone(),
            server.remote.accepted.clone(),
        )),
    });
    let mut app = builder.build().await.unwrap();
    app.start().await.unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(
        app.context()
            .resolve::<Tokens>()
            .unwrap()
            .calls
            .load(Ordering::SeqCst),
        0
    );
    app.shutdown().await.unwrap();
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    server.stop().await;
}
#[tokio::test]
async fn failed_eager_readiness_unwinds_started_infrastructure() {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [
                ("grpc.clients.alpha.endpoint", "http://127.0.0.1:1"),
                ("grpc.clients.beta.endpoint", "http://127.0.0.1:1"),
                ("grpc.clients.alpha.connect", "eager"),
            ],
        ))
        .build()
        .unwrap();
    let starts = Arc::new(AtomicUsize::new(0));
    let stops = Arc::new(AtomicUsize::new(0));
    let application_starts = Arc::new(AtomicUsize::new(0));
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    builder.__infrastructure_lifecycle_hook(
        "a.fixture",
        Observer {
            starts: starts.clone(),
            stops: stops.clone(),
            connections: None,
        },
    );
    builder.lifecycle_hook(Observer {
        starts: application_starts.clone(),
        stops: Arc::default(),
        connections: None,
    });
    let mut app = builder.build().await.unwrap();
    assert!(app.start().await.is_err());
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert_eq!(application_starts.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn client_clones_allow_concurrent_calls() {
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let server = Running::start_remote(Remote {
        value: 7,
        barrier: Some(barrier.clone()),
        ..Remote::default()
    })
    .await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    let alpha = app.context().resolve::<Alpha>().unwrap();
    let mut first = alpha.client();
    let mut second = alpha.client();
    let (first, second, _) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(
            first.secret(pb::Item { value: 0 }),
            second.secret(pb::Item { value: 0 }),
            barrier.wait()
        )
    })
    .await
    .expect("calls were serialized");
    assert_eq!(first.unwrap().into_inner().value, 7);
    assert_eq!(second.unwrap().into_inner().value, 7);
    server.stop().await;
}
#[grpc_client(name = "alpha", client = pb::hidden_client::HiddenClient)]
struct DuplicateAlpha;
struct ConstructorSentinel;
impl Injector for ConstructorSentinel {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        panic!("invalid client metadata must fail before constructors")
    }
}
#[cauldron(providers = [Tokens, Alpha, DuplicateAlpha, ConstructorSentinel])]
struct DuplicateApp;
#[tokio::test]
async fn duplicate_selected_client_names_fail_preconstruction() {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("grpc.clients.alpha.endpoint", "http://127.0.0.1:1")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<DuplicateApp>().unwrap();
    let error = match builder.build().await {
        Ok(_) => panic!("duplicate client accepted"),
        Err(error) => error,
    };
    assert_eq!(error.code(), furnace_rs_grpc::FURNACE153);
}

#[tokio::test]
async fn client_does_not_forward_inbound_credentials() {
    let server = Running::start(7).await;
    let app = app(&server.endpoint, &server.endpoint, &[]).await;
    let mut incoming = tonic::Request::new(pb::Item { value: 0 });
    set_grpc_bearer_token(&mut incoming, "inbound-token-sentinel").unwrap();
    incoming
        .extensions_mut()
        .insert(String::from("inbound-principal-sentinel"));
    app.context()
        .resolve::<Beta>()
        .unwrap()
        .client()
        .secret(incoming.into_inner())
        .await
        .unwrap();
    assert!(server.remote.headers.lock().unwrap()[0].0.is_none());
    server.stop().await;
}
#[grpc_client(name = "streaming", client = pb::transport_client::TransportClient, token_provider = Tokens)]
struct StreamingClient;
#[cauldron(providers = [Tokens, StreamingClient])]
struct StreamingApp;
type Output = std::pin::Pin<
    Box<dyn tonic::codegen::tokio_stream::Stream<Item = Result<pb::Item, tonic::Status>> + Send>,
>;
#[derive(Clone)]
struct StreamRemote {
    gate: Arc<tokio::sync::Notify>,
}
#[tonic::async_trait]
impl pb::transport_server::Transport for StreamRemote {
    async fn unary(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        Ok(tonic::Response::new(request.into_inner()))
    }
    type ServerStreamStream = Output;
    async fn server_stream(
        &self,
        _: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        let gate = self.gate.clone();
        let (send, receive) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            gate.notified().await;
            let _ = send.send(Ok(pb::Item { value: 13 })).await;
        });
        Ok(tonic::Response::new(Box::pin(
            tonic::codegen::tokio_stream::wrappers::ReceiverStream::new(receive),
        )))
    }
    async fn client_stream(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        let mut input = request.into_inner();
        let mut count = 0;
        let mut total = 0;
        while let Some(item) = input.message().await? {
            count += 1;
            total += item.value;
        }
        Ok(tonic::Response::new(pb::Summary { count, total }))
    }
    type BidiStream = Output;
    async fn bidi(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<Output>, tonic::Status> {
        Ok(tonic::Response::new(Box::pin(request.into_inner())))
    }
}
#[tokio::test]
async fn response_stream_outlives_establishment_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let gate = Arc::new(tokio::sync::Notify::new());
    let server = pb::transport_server::TransportServer::new(StreamRemote { gate: gate.clone() });
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(server)
            .serve_with_incoming_shutdown(
                tonic::codegen::tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = signal.await;
                },
            )
            .await
            .unwrap();
    });
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "streaming",
            [
                ("grpc.clients.streaming.endpoint", endpoint.as_str()),
                ("grpc.clients.streaming.request_timeout_ms", "1000"),
            ],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<StreamingApp>().unwrap();
    let mut app = builder.build().await.unwrap();
    app.start().await.unwrap();
    let mut client = app.context().resolve::<StreamingClient>().unwrap().client();
    assert_eq!(
        client
            .unary(pb::Item { value: 7 })
            .await
            .unwrap()
            .into_inner()
            .value,
        7
    );
    let input =
        || tonic::codegen::tokio_stream::iter([pb::Item { value: 2 }, pb::Item { value: 3 }]);
    let summary = client.client_stream(input()).await.unwrap().into_inner();
    assert_eq!((summary.count, summary.total), (2, 5));
    let mut bidi = client.bidi(input()).await.unwrap().into_inner();
    assert_eq!(bidi.message().await.unwrap().unwrap().value, 2);
    assert_eq!(bidi.message().await.unwrap().unwrap().value, 3);
    assert!(bidi.message().await.unwrap().is_none());
    drop(bidi);
    let mut stream = client
        .server_stream(pb::Item { value: 0 })
        .await
        .unwrap()
        .into_inner();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::time::resume();
    gate.notify_one();
    assert_eq!(stream.message().await.unwrap().unwrap().value, 13);
    assert!(stream.message().await.unwrap().is_none());
    drop(stream);
    assert_eq!(
        app.context()
            .resolve::<Tokens>()
            .unwrap()
            .calls
            .load(Ordering::SeqCst),
        4
    );
    app.shutdown().await.unwrap();
    drop(client);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn establishment_deadline_bounds_token_provider_wait() {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "deadline",
            [("grpc.clients.alpha.endpoint", "http://127.0.0.1:1")],
        ))
        .build()
        .unwrap();
    let tokens = Tokens::default();
    tokens.stall.store(true, Ordering::SeqCst);
    let channel = furnace_rs_grpc::GrpcChannel::from_config(
        furnace_rs_grpc::GrpcClientConfig::parse(&config, "alpha").unwrap(),
        Some(furnace_rs_grpc::ErasedTokenProvider::of(tokens.clone())),
    )
    .await
    .unwrap();
    let mut client = pb::hidden_client::HiddenClient::new(channel);
    let mut request = tonic::Request::new(pb::Item { value: 0 });
    request.set_timeout(Duration::from_millis(50));
    let status = tokio::time::timeout(Duration::from_secs(2), client.secret(request))
        .await
        .expect("token provider outlived RPC deadline")
        .unwrap_err();
    assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
    assert_eq!(tokens.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn establishment_deadline_bounds_remote_response_wait() {
    let hold = Arc::new(tokio::sync::Notify::new());
    let server = Running::start_remote(Remote {
        hold: Some(hold.clone()),
        ..Remote::default()
    })
    .await;
    let mut app = app(&server.endpoint, &server.endpoint, &[]).await;
    app.start().await.unwrap();
    let mut request = tonic::Request::new(pb::Item { value: 0 });
    request.set_timeout(Duration::from_millis(50));
    let status = app
        .context()
        .resolve::<Beta>()
        .unwrap()
        .client()
        .secret(request)
        .await
        .unwrap_err();
    assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
    hold.notify_one();
    app.shutdown().await.unwrap();
    server.stop().await;
}

#[test]
fn bearer_helper_redacts_credentials_from_native_request_debug() {
    let mut request = tonic::Request::new(());
    set_grpc_bearer_token(&mut request, "private-credential-sentinel").unwrap();
    assert!(!format!("{request:?}").contains("private-credential-sentinel"));
}

#[tokio::test]
async fn bearer_sources_use_http2_never_indexed_encoding() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for provider in [true, false] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let capture = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut preface = [0; 24];
            socket.read_exact(&mut preface).await.unwrap();
            assert_eq!(&preface, b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
            socket
                .write_all(&[0, 0, 0, 4, 0, 0, 0, 0, 0])
                .await
                .unwrap();
            loop {
                let mut frame = [0; 9];
                socket.read_exact(&mut frame).await.unwrap();
                let length = (usize::from(frame[0]) << 16)
                    | (usize::from(frame[1]) << 8)
                    | usize::from(frame[2]);
                let mut payload = vec![0; length];
                socket.read_exact(&mut payload).await.unwrap();
                if frame[3] == 4 && frame[4] & 1 == 0 {
                    socket
                        .write_all(&[0, 0, 0, 4, 1, 0, 0, 0, 0])
                        .await
                        .unwrap();
                }
                if frame[3] == 1 {
                    assert_ne!(
                        frame[4] & 4,
                        0,
                        "small test metadata unexpectedly used CONTINUATION"
                    );
                    return payload;
                }
            }
        });
        let mut app = app(
            &endpoint,
            &endpoint,
            &[("grpc.clients.alpha.request_timeout_ms", "1000")],
        )
        .await;
        app.start().await.unwrap();
        let mut request = tonic::Request::new(pb::Item { value: 0 });
        if !provider {
            request.metadata_mut().insert(
                "authorization",
                "Bearer explicit-sensitive-credential".parse().unwrap(),
            );
        }
        // The capture peer closes after receiving headers, without returning an RPC response.
        assert!(
            app.context()
                .resolve::<Alpha>()
                .unwrap()
                .client()
                .secret(request)
                .await
                .is_err()
        );
        let headers = capture.await.unwrap();
        // HPACK literal never-indexed (four-bit prefix), static name index 23 = authorization.
        assert!(
            headers.windows(2).any(|pair| pair == [0x1f, 0x08]),
            "authorization was not encoded never-indexed: {headers:02x?}"
        );
        app.shutdown().await.unwrap();
    }
}
