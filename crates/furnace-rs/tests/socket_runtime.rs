//! HTTP and socket listeners share one application lifecycle with separate ports.
#![cfg(all(feature = "socket", feature = "http"))]
use furnace_rs::prelude::*;
use furnace_rs::socket::WebSocket;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
struct Owned;
impl Injector for Owned {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
}
#[socket_endpoint(path = "/echo")]
impl Owned {
    async fn handle(&self, _: SocketHandshake, mut socket: WebSocket) -> SocketResult<()> {
        if let Some(Ok(message)) = socket.recv().await {
            socket
                .send(message)
                .await
                .map_err(|_| SocketError::new("closed"))?;
        }
        Ok(())
    }
}
#[furnace_rs::controller(route = "/")]
impl Http {
    #[get]
    async fn hello(&self) -> &'static str {
        "http"
    }
}
#[furnace_rs::controller]
struct Http;
#[cauldron(root,controllers=[Http],socket_endpoints=[Owned])]
struct App;
async fn address() -> std::net::SocketAddr {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    l.local_addr().unwrap()
}
#[tokio::test]
async fn separate_http_and_socket_listeners_serve_and_shutdown_together() {
    let http = address().await;
    let socket = address().await;
    let config = furnace_rs::core::ConfigBuilder::new()
        .source(furnace_rs::core::MapSource::new(
            "test",
            [("socket.address", socket.to_string())],
        ))
        .build()
        .unwrap();
    let mut builder = Furnace::builder_with_config(config.clone());
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    let routes = RuntimeRoutes::new()
        .with_http(furnace_rs::build_router(&app).unwrap())
        .with_socket(furnace_rs::socket::build_socket_routes(&app).unwrap());
    let bindings = RuntimeBindings::new()
        .with_http(http)
        .with_socket(SocketServerConfig::parse(&config).unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(serve_transports(app, routes, bindings, async {
        let _ = stopped.await;
    }));
    let mut client = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Ok((client, _)) =
                tokio_tungstenite::connect_async(format!("ws://{socket}/echo")).await
            {
                break client;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut request = tokio::net::TcpStream::connect(http).await.unwrap();
    request
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    let mut reply = Vec::new();
    request.read_to_end(&mut reply).await.unwrap();
    assert!(String::from_utf8(reply).unwrap().contains("http"));
    client
        .send(tokio_tungstenite::tungstenite::Message::Text(
            "hello".into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        client.next().await.unwrap().unwrap().into_text().unwrap(),
        "hello"
    );
    drop(client);
    stop.send(()).unwrap();
    assert!(server.await.unwrap().is_ok());
}
