//! Shared runtime lifecycle and supervised native protobuf connections.
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_core::{ConfigBuilder, Furnace, Injector, MapSource, cauldron};
use furnace_rs_grpc::{grpc_service, tonic};
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use tonic::codegen::tokio_stream::{self as streams, Stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<pb::Item, tonic::Status>> + Send>>;
#[derive(Default)]
struct StateInner {
    constructors: AtomicUsize,
    starts: AtomicUsize,
    stops: AtomicUsize,
    expect_drained: AtomicBool,
    calls: AtomicUsize,
    drops: AtomicUsize,
    dropped: tokio::sync::Notify,
}
#[derive(Clone)]
struct State(Arc<StateInner>);
impl Injector for State {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self(Arc::default()))
    }
}
#[derive(Clone)]
struct TransportService {
    state: State,
}
impl Injector for TransportService {
    type Dependencies = (State,);
    async fn inject((state,): Self::Dependencies) -> furnace_rs_core::Result<Self> {
        state.0.constructors.fetch_add(1, Ordering::SeqCst);
        Ok(Self { state })
    }
}
struct ProbeStream {
    inner: ItemStream,
    state: State,
}

struct CancellableInput {
    first: Option<pb::Item>,
    cancelled: tokio::sync::oneshot::Receiver<()>,
}
impl Stream for CancellableInput {
    type Item = pb::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Some(item) = this.first.take() {
            return Poll::Ready(Some(item));
        }
        std::future::Future::poll(Pin::new(&mut this.cancelled), cx).map(|_| None)
    }
}
impl Stream for ProbeStream {
    type Item = Result<pb::Item, tonic::Status>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}
impl Drop for ProbeStream {
    fn drop(&mut self) {
        self.state.0.drops.fetch_add(1, Ordering::SeqCst);
        self.state.0.dropped.notify_one();
    }
}

#[grpc_service(server = pb::transport_server::TransportServer, descriptor = FILE_DESCRIPTOR_SET)]
#[tonic::async_trait]
impl pb::transport_server::Transport for TransportService {
    async fn unary(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<pb::Item>, tonic::Status> {
        self.state.0.calls.fetch_add(1, Ordering::SeqCst);
        if request.get_ref().value == 42 {
            let peer = request
                .remote_addr()
                .ok_or_else(|| tonic::Status::internal("missing trusted peer"))?;
            assert!(peer.ip().is_loopback());
            return Ok(tonic::Response::new(pb::Item {
                value: u64::from(peer.port()),
            }));
        }
        Ok(tonic::Response::new(request.into_inner()))
    }
    type ServerStreamStream = ItemStream;
    async fn server_stream(
        &self,
        request: tonic::Request<pb::Item>,
    ) -> Result<tonic::Response<ItemStream>, tonic::Status> {
        self.state.0.calls.fetch_add(1, Ordering::SeqCst);
        let count = request.into_inner().value;
        Ok(tonic::Response::new(Box::pin(streams::iter(
            (0..count).map(|value| Ok(pb::Item { value })),
        ))))
    }
    async fn client_stream(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<pb::Summary>, tonic::Status> {
        self.state.0.calls.fetch_add(1, Ordering::SeqCst);
        let mut input = request.into_inner();
        let (mut count, mut total) = (0_u64, 0_u64);
        while let Some(item) = input.message().await? {
            count += 1;
            total = total
                .checked_add(item.value)
                .ok_or_else(|| tonic::Status::invalid_argument("sum overflow"))?;
        }
        Ok(tonic::Response::new(pb::Summary { count, total }))
    }
    type BidiStream = ItemStream;
    async fn bidi(
        &self,
        request: tonic::Request<tonic::Streaming<pb::Item>>,
    ) -> Result<tonic::Response<ItemStream>, tonic::Status> {
        self.state.0.calls.fetch_add(1, Ordering::SeqCst);
        Ok(tonic::Response::new(Box::pin(ProbeStream {
            inner: Box::pin(request.into_inner()),
            state: self.state.clone(),
        })))
    }
}
#[cauldron(root, providers = [State], grpc_services = [TransportService])]
struct App;

use furnace_rs_grpc::common::runtime::{
    RuntimeTransportDescriptor, serve_prepared_transports_with,
};
use furnace_rs_grpc::{
    ApplicationRuntimeError, GrpcServerConfig, RuntimeBindings, RuntimeGrpcExt, RuntimeRoutes,
};
struct Hook(State);
impl furnace_rs_core::LifecycleHook for Hook {
    fn name(&self) -> &str {
        "runtime probe"
    }
    fn start<'a>(
        &'a self,
        _: &'a furnace_rs_core::ApplicationContext,
    ) -> furnace_rs_core::LifecycleFuture<'a> {
        Box::pin(async {
            self.0.0.starts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn stop<'a>(
        &'a self,
        _: &'a furnace_rs_core::ApplicationContext,
    ) -> furnace_rs_core::LifecycleFuture<'a> {
        Box::pin(async {
            if self.0.0.expect_drained.load(Ordering::SeqCst) {
                assert_eq!(
                    self.0.0.drops.load(Ordering::SeqCst),
                    1,
                    "provider shutdown preceded stream cancellation"
                );
            }
            self.0.0.stops.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}
async fn running() -> (
    pb::transport_client::TransportClient<tonic::transport::Channel>,
    State,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<(), ApplicationRuntimeError>>,
    std::net::SocketAddr,
) {
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [("grpc.server.shutdown_timeout_ms", "100")],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    let state = State(Arc::default());
    builder.provide(state.clone()).unwrap();
    builder.lifecycle_hook(Hook(state.clone()));
    let app = builder.build().await.unwrap();
    let routes = furnace_rs_grpc::build_grpc_routes(&app).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let config = GrpcServerConfig::parse(app.context().config())
        .unwrap()
        .with_address(address);
    let mut routes = RuntimeRoutes::new().with_grpc(routes);
    let mut bindings = RuntimeBindings::new().with_grpc(config);
    let descriptor = furnace_rs_core::__private::inventory::iter::<RuntimeTransportDescriptor>
        .into_iter()
        .find(|d| d.name == "grpc")
        .unwrap();
    let transport = (descriptor.prepare)(&app, Some(&mut routes), Some(&mut bindings))
        .await
        .unwrap();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let mut listener = Some(listener);
    let task = tokio::spawn(serve_prepared_transports_with(
        app,
        vec![transport],
        async move {
            let _ = signal.await;
        },
        move |_| {
            let listener = listener.take().unwrap();
            Box::pin(async move { Ok(listener) })
        },
    ));
    let client = pb::transport_client::TransportClient::connect(format!("http://{address}"))
        .await
        .unwrap();
    (client, state, stop, task, address)
}
#[tokio::test]
async fn runtime_serves_all_four_shapes_and_stops_once() {
    let (mut client, state, stop, task, address) = running().await;
    assert_eq!(
        client
            .unary(pb::Item { value: 7 })
            .await
            .unwrap()
            .into_inner()
            .value,
        7
    );
    let mut stream = client
        .server_stream(pb::Item { value: 3 })
        .await
        .unwrap()
        .into_inner();
    for value in 0..3 {
        assert_eq!(stream.message().await.unwrap().unwrap().value, value);
    }
    assert!(stream.message().await.unwrap().is_none());
    let summary = client
        .client_stream(streams::iter([
            pb::Item { value: 2 },
            pb::Item { value: 5 },
        ]))
        .await
        .unwrap()
        .into_inner();
    assert_eq!((summary.count, summary.total), (2, 7));
    let mut bidi = client
        .bidi(streams::iter([pb::Item { value: 8 }]))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bidi.message().await.unwrap().unwrap().value, 8);
    assert!(bidi.message().await.unwrap().is_none());
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(state.0.constructors.load(Ordering::SeqCst), 1);
    assert_eq!(state.0.starts.load(Ordering::SeqCst), 1);
    assert_eq!(state.0.stops.load(Ordering::SeqCst), 1);
    assert!(tokio::net::TcpListener::bind(address).await.is_ok());
}
#[tokio::test]
async fn outstanding_bidi_is_destroyed_before_provider_shutdown() {
    let (mut client, state, stop, task, _) = running().await;
    let (_cancel, cancelled) = tokio::sync::oneshot::channel();
    let mut bidi = client
        .bidi(CancellableInput {
            first: Some(pb::Item { value: 1 }),
            cancelled,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(bidi.message().await.unwrap().unwrap().value, 1);
    assert_eq!(state.0.drops.load(Ordering::SeqCst), 0);
    state.0.expect_drained.store(true, Ordering::SeqCst);
    stop.send(()).unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(ApplicationRuntimeError::DrainTimeout)
    ));
    assert_eq!(state.0.drops.load(Ordering::SeqCst), 1);
    assert_eq!(state.0.stops.load(Ordering::SeqCst), 1);
    assert!(bidi.message().await.is_err());
}
#[cauldron(root)]
struct Empty;
#[tokio::test]
async fn uncompiled_transport_and_empty_application_fail_before_start() {
    use furnace_rs_grpc::common::runtime::prepare_application;
    let selected = |transport| {
        ConfigBuilder::new()
            .source(
                MapSource::new("test", std::iter::empty::<(&str, &str)>())
                    .with_string_array("furnace.transports", [transport]),
            )
            .build()
            .unwrap()
    };
    let error = prepare_application::<Empty>(selected("grpc"))
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("no runnable gRPC service"));
    assert!(furnace_rs_grpc::RuntimeSelection::parse(&selected("http"), false, true).is_err());
    assert!(furnace_rs_grpc::RuntimeSelection::parse(&selected("grpc"), true, false).is_err());
}
#[furnace_rs_grpc::grpc_client(name = "eager", client = pb::hidden_client::HiddenClient)]
struct Eager;
#[cauldron(root, providers = [Eager])]
struct EagerApp;
#[tokio::test]
async fn eager_client_failure_binds_no_listener() {
    let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = unavailable.local_addr().unwrap();
    drop(unavailable);
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "test",
            [
                ("grpc.clients.eager.endpoint", format!("http://{address}")),
                ("grpc.clients.eager.connect", "eager".into()),
            ],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<EagerApp>().unwrap();
    let app = builder.build().await.unwrap();
    let binds = Arc::new(AtomicUsize::new(0));
    let observed = binds.clone();
    let prepared = furnace_rs_grpc::common::runtime::PreparedTransport {
        name: "grpc",
        address: "127.0.0.1:0".into(),
        summary: String::new(),
        drain_timeout: None,
        transition: None,
        serve: Box::new(|_, _, _| Box::pin(async { Ok(()) })),
    };
    let result =
        serve_prepared_transports_with(app, vec![prepared], std::future::pending(), move |_| {
            observed.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { unreachable!("eager failure must precede binding") })
        })
        .await;
    assert!(matches!(result, Err(ApplicationRuntimeError::Lifecycle(_))));
    assert_eq!(binds.load(Ordering::SeqCst), 0);
}
#[cfg(feature = "tls")]
#[path = "support/tls.rs"]
mod tls_support;
#[cfg(feature = "tls")]
#[tokio::test]
async fn supervised_runtime_serves_verified_tls() {
    let certificates = tls_support::Certificates::new();
    let config = ConfigBuilder::new()
        .source(MapSource::new(
            "tls",
            [
                (
                    "grpc.server.tls.certificate",
                    certificates.certificate.display().to_string(),
                ),
                (
                    "grpc.server.tls.private_key",
                    certificates.key.display().to_string(),
                ),
            ],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut routes =
        RuntimeRoutes::new().with_grpc(furnace_rs_grpc::build_grpc_routes(&app).unwrap());
    let mut bindings = RuntimeBindings::new().with_grpc(
        GrpcServerConfig::parse(app.context().config())
            .unwrap()
            .with_address(address),
    );
    let descriptor = furnace_rs_core::__private::inventory::iter::<RuntimeTransportDescriptor>
        .into_iter()
        .find(|d| d.name == "grpc")
        .unwrap();
    let transport = (descriptor.prepare)(&app, Some(&mut routes), Some(&mut bindings))
        .await
        .unwrap();
    let (stop, signal) = tokio::sync::oneshot::channel();
    let mut listener = Some(listener);
    let task = tokio::spawn(serve_prepared_transports_with(
        app,
        vec![transport],
        async {
            let _ = signal.await;
        },
        move |_| {
            let listener = listener.take().unwrap();
            Box::pin(async { Ok(listener) })
        },
    ));
    let tls = tonic::transport::ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(tonic::transport::Certificate::from_pem(
            std::fs::read(&certificates.ca).unwrap(),
        ));
    let endpoint = tonic::transport::Endpoint::from_shared(format!("https://{address}"))
        .unwrap()
        .tls_config(tls)
        .unwrap();
    let mut client = pb::transport_client::TransportClient::new(endpoint.connect().await.unwrap());
    assert_eq!(
        client
            .unary(pb::Item { value: 41 })
            .await
            .unwrap()
            .into_inner()
            .value,
        41
    );
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn trusted_socket_peer_survives_supervised_dispatch() {
    let (mut client, _, stop, task, _) = running().await;
    let mut request = tonic::Request::new(pb::Item { value: 42 });
    request
        .metadata_mut()
        .insert("x-forwarded-for", "192.0.2.1:9876".parse().unwrap());
    let port = client.unary(request).await.unwrap().into_inner().value;
    assert!(port > 0 && port != 9876);
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}
