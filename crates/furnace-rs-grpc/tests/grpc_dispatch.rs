//! Real protobuf framing, four RPC shapes, limits, and cancellation through DI services.
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
    time::Duration,
};
use tonic::codegen::tokio_stream::{self as streams, Stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<pb::Item, tonic::Status>> + Send>>;
#[derive(Default)]
struct StateInner {
    constructors: AtomicUsize,
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

struct Running {
    channel: tonic::transport::Channel,
    client: pb::transport_client::TransportClient<tonic::transport::Channel>,
    state: State,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
}
impl Running {
    async fn start(settings: &[(&str, &str)]) -> Self {
        let config = ConfigBuilder::new()
            .source(MapSource::new("test", settings.iter().copied()))
            .build()
            .unwrap();
        let mut builder = Furnace::builder_with_config(config);
        builder.root::<App>().unwrap();
        let application = builder.build().await.unwrap();
        let state = application
            .context()
            .resolve::<State>()
            .unwrap()
            .as_ref()
            .clone();
        let routes = furnace_rs_grpc::build_grpc_routes(&application).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, signal) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_routes(routes)
                .serve_with_incoming_shutdown(
                    streams::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = signal.await;
                    },
                )
                .await
        });
        let channel = tonic::transport::Endpoint::from_shared(format!("http://{address}"))
            .unwrap()
            .connect()
            .await
            .unwrap();
        let client = pb::transport_client::TransportClient::new(channel.clone());
        Self {
            channel,
            client,
            state,
            stop,
            task,
        }
    }
    async fn shutdown(self) {
        let _ = self.stop.send(());
        tokio::time::timeout(Duration::from_secs(3), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn managed_service_dispatches_all_rpc_shapes() {
    let mut server = Running::start(&[]).await;
    assert_eq!(
        server
            .client
            .unary(pb::Item { value: 7 })
            .await
            .unwrap()
            .into_inner()
            .value,
        7
    );
    let mut stream = server
        .client
        .server_stream(pb::Item { value: 3 })
        .await
        .unwrap()
        .into_inner();
    let mut values = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        values.push(item.value);
    }
    assert_eq!(values, [0, 1, 2]);
    let summary = server
        .client
        .client_stream(streams::iter([2, 3, 5].map(|value| pb::Item { value })))
        .await
        .unwrap()
        .into_inner();
    assert_eq!((summary.count, summary.total), (3, 10));
    let mut stream = server
        .client
        .bidi(streams::iter([8, 13, 21].map(|value| pb::Item { value })))
        .await
        .unwrap()
        .into_inner();
    let mut values = Vec::new();
    while let Some(item) = stream.message().await.unwrap() {
        values.push(item.value);
    }
    assert_eq!(values, [8, 13, 21]);
    let (mut first, mut second) = (server.client.clone(), server.client.clone());
    let (a, b) = tokio::join!(
        first.unary(pb::Item { value: 1 }),
        second.unary(pb::Item { value: 2 })
    );
    assert_eq!(
        (a.unwrap().into_inner().value, b.unwrap().into_inner().value),
        (1, 2)
    );
    assert_eq!(server.state.0.constructors.load(Ordering::SeqCst), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn oversized_messages_rejected_in_both_directions() {
    let mut decoding = Running::start(&[("grpc.server.max_decoding_message_bytes", "2")]).await;
    assert_eq!(
        decoding
            .client
            .unary(pb::Item { value: 300 })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::OutOfRange
    );
    assert_eq!(decoding.state.0.calls.load(Ordering::SeqCst), 0);
    decoding.shutdown().await;
    let mut encoding = Running::start(&[("grpc.server.max_encoding_message_bytes", "2")]).await;
    assert_eq!(
        encoding
            .client
            .unary(pb::Item { value: 300 })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::OutOfRange
    );
    encoding.shutdown().await;
}

#[tokio::test]
async fn cancelled_bidi_releases_handler_state() {
    let mut server = Running::start(&[]).await;
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    let input = CancellableInput {
        first: Some(pb::Item { value: 8 }),
        cancelled,
    };
    let mut response = server.client.bidi(input).await.unwrap().into_inner();
    assert_eq!(response.message().await.unwrap().unwrap().value, 8);
    drop(response);
    cancel.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), server.state.0.dropped.notified())
        .await
        .unwrap();
    assert_eq!(server.state.0.drops.load(Ordering::SeqCst), 1);
    server.shutdown().await;
}

#[tokio::test]
async fn unknown_wire_method_is_unimplemented() {
    let server = Running::start(&[]).await;
    let mut client = tonic::client::Grpc::new(server.channel.clone());
    client.ready().await.unwrap();
    let result: Result<tonic::Response<pb::Item>, _> = client
        .unary(
            tonic::Request::new(pb::Item { value: 1 }),
            http::uri::PathAndQuery::from_static("/furnace.test.v1.Transport/Missing"),
            tonic_prost::ProstCodec::default(),
        )
        .await;
    assert_eq!(result.unwrap_err().code(), tonic::Code::Unimplemented);
    assert_eq!(server.state.0.calls.load(Ordering::SeqCst), 0);
    server.shutdown().await;
}
