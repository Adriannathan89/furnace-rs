//! Shared runtime lifecycle and supervised native protobuf connections.
#![cfg(all(feature = "http", feature = "grpc"))]
use furnace_grpc_test_support::{FILE_DESCRIPTOR_SET, pb};
use furnace_rs_core::{ConfigBuilder, Furnace, Injector, MapSource, cauldron};
use furnace_rs_grpc::{grpc_service, tonic};
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
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

#[allow(dead_code)]
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
        Ok(tonic::Response::new(pb::Item {
            value: if request.get_ref().value == 0 {
                Arc::as_ptr(&self.state.0) as usize as u64
            } else {
                request.into_inner().value
            },
        }))
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
#[cauldron(root, providers = [State], controllers = [HttpController], grpc_services = [TransportService])]
struct App;

#[furnace_rs::controller]
struct HttpController {
    #[inject]
    state: State,
}
impl furnace_rs::Sealable for HttpController {
    fn seals() -> furnace_rs::SealRegistration<Self> {
        furnace_rs::SealRegistration::new()
    }
}
#[furnace_rs::controller]
impl HttpController {
    #[furnace_rs::get("/identity")]
    async fn identity(&self) -> String {
        (Arc::as_ptr(&self.state.0) as usize as u64).to_string()
    }
}
#[tokio::test]
async fn burn_configuration_selects_http_grpc_or_both() {
    for values in [vec!["http"], vec!["grpc"], vec!["http", "grpc"]] {
        let config = ConfigBuilder::new()
            .source(
                MapSource::new("test", std::iter::empty::<(&str, &str)>())
                    .with_string_array("furnace.transports", values.clone()),
            )
            .build()
            .unwrap();
        let (_, prepared) = furnace_rs_grpc::common::runtime::prepare_application::<App>(config)
            .await
            .unwrap();
        let names: std::collections::BTreeSet<_> = prepared.iter().map(|t| t.name).collect();
        assert_eq!(names, values.into_iter().collect());
    }
}
#[tokio::test]
async fn missing_transports_retains_http() {
    let (_, prepared) = furnace_rs_grpc::common::runtime::prepare_application::<App>(
        furnace_rs_core::Config::empty(),
    )
    .await
    .unwrap();
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].name, "http");
}
struct Hook(State);
impl furnace_rs_core::LifecycleHook for Hook {
    fn name(&self) -> &str {
        "mixed runtime"
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
            self.0.0.stops.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}
#[tokio::test]
async fn mixed_transports_share_provider_identity() {
    use furnace_rs::prelude::{RuntimeBindings, RuntimeGrpcExt, RuntimeRoutes};
    use furnace_rs_grpc::common::runtime::{
        RuntimeTransportDescriptor, serve_prepared_transports_with,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    let state = State(Arc::default());
    builder.provide(state.clone()).unwrap();
    builder.lifecycle_hook(Hook(state.clone()));
    let app = builder.build().await.unwrap();
    let http = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_address = http.local_addr().unwrap();
    let grpc = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let grpc_address = grpc.local_addr().unwrap();
    let mut routes = RuntimeRoutes::new()
        .with_http(furnace_rs::build_router(&app).unwrap())
        .with_grpc(furnace_rs::build_grpc_routes(&app).unwrap());
    let mut bindings = RuntimeBindings::new().with_http(http_address).with_grpc(
        furnace_rs::GrpcServerConfig::parse(app.context().config())
            .unwrap()
            .with_address(grpc_address),
    );
    let mut transports = Vec::new();
    for descriptor in furnace_rs_core::__private::inventory::iter::<RuntimeTransportDescriptor> {
        transports.push(
            (descriptor.prepare)(&app, Some(&mut routes), Some(&mut bindings))
                .await
                .unwrap(),
        );
    }
    let mut listeners = std::collections::BTreeMap::from([
        (http_address.to_string(), http),
        (grpc_address.to_string(), grpc),
    ]);
    let (stop, signal) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(serve_prepared_transports_with(
        app,
        transports,
        async {
            let _ = signal.await;
        },
        move |address| {
            let listener = listeners.remove(&address).unwrap();
            Box::pin(async { Ok(listener) })
        },
    ));
    let mut client =
        pb::transport_client::TransportClient::connect(format!("http://{grpc_address}"))
            .await
            .unwrap();
    let grpc_identity = client
        .unary(pb::Item { value: 0 })
        .await
        .unwrap()
        .into_inner()
        .value
        .to_string();
    let mut socket = tokio::net::TcpStream::connect(http_address).await.unwrap();
    socket
        .write_all(b"GET /identity HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).await.unwrap();
    assert!(response.ends_with(&grpc_identity));
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(state.0.starts.load(Ordering::SeqCst), 1);
    assert_eq!(state.0.stops.load(Ordering::SeqCst), 1);
    assert!(tokio::net::TcpListener::bind(http_address).await.is_ok());
    assert!(tokio::net::TcpListener::bind(grpc_address).await.is_ok());
}
