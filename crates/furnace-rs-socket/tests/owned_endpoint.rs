//! Real WebSocket owned-socket serving and runtime drain.
use furnace_rs_core::{Furnace, Injector};
use furnace_rs_socket::runtime::{
    RuntimeBindings, RuntimeRoutes, RuntimeSocketExt, serve_transports,
};
use furnace_rs_socket::{
    EndpointMode, ProcessingMode, SocketEndpointDescriptor, SocketEndpointRegistration,
    SocketHandshake, SocketServerConfig, WebSocket,
};
use futures_util::{SinkExt, StreamExt};
use std::{any::TypeId, sync::Arc, time::Duration};
struct Echo;
impl Injector for Echo {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
impl furnace_rs_core::__private::TransportProvider for Echo {
    const TRANSPORT: &'static str = "socket";
}
fn route(
    app: furnace_rs_core::ApplicationContext,
) -> furnace_rs_core::Result<axum::routing::MethodRouter> {
    furnace_rs_socket::__private::owned_route::<Echo>(
        app,
        &ENDPOINT,
        |_: Arc<Echo>, _: SocketHandshake, mut socket: WebSocket| {
            Box::pin(async move {
                while let Some(Ok(message)) = socket.recv().await {
                    if matches!(
                        message,
                        furnace_rs_socket::Message::Text(_) | furnace_rs_socket::Message::Binary(_)
                    ) {
                        socket
                            .send(message)
                            .await
                            .map_err(|_| furnace_rs_socket::SocketError::new("closed"))?;
                    }
                }
                Ok(())
            })
        },
    )
}
static ENDPOINT: SocketEndpointDescriptor = SocketEndpointDescriptor::new(
    TypeId::of::<Echo>,
    "Echo",
    "/echo",
    EndpointMode::Raw,
    ProcessingMode::Sequential,
    &[],
)
.with_route(route);
furnace_rs_core::__private::inventory::submit! {SocketEndpointRegistration {descriptor:&ENDPOINT}}
#[furnace_rs_core::cauldron(grpc_services = [Echo])]
struct App;

#[tokio::test]
async fn owned_text_binary_and_bounded_shutdown() {
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = probe.local_addr().unwrap();
    drop(probe);
    let config = furnace_rs_core::ConfigBuilder::new()
        .source(furnace_rs_core::MapSource::new(
            "test",
            [
                ("socket.address", address.to_string()),
                ("socket.shutdown_timeout_ms", "100".into()),
            ],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config.clone());
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    let routes = furnace_rs_socket::build_socket_routes(&app).unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(serve_transports(
        app,
        RuntimeRoutes::new().with_socket(routes),
        RuntimeBindings::new().with_socket(SocketServerConfig::parse(&config).unwrap()),
        async {
            let _ = stopped.await;
        },
    ));
    let mut client = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok((client, _)) =
                tokio_tungstenite::connect_async(format!("ws://{address}/echo")).await
            {
                break client;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    for message in [
        tokio_tungstenite::tungstenite::Message::Text("hello".into()),
        tokio_tungstenite::tungstenite::Message::Binary(vec![1, 2, 3].into()),
    ] {
        client.send(message.clone()).await.unwrap();
        assert_eq!(client.next().await.unwrap().unwrap(), message);
    }
    stop.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.is_ok()
            || matches!(
                result,
                Err(furnace_rs_socket::runtime::ApplicationRuntimeError::DrainTimeout)
            )
    );
}
