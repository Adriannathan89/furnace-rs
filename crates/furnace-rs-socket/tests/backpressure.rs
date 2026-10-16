//! Processing order, bounded queues, and responsive heartbeat controls.
mod support;
use furnace_rs_core::{Injector, cauldron};
use furnace_rs_socket::{SocketContext, SocketError, SocketResult, socket_gateway};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::Duration;
struct Sequential;
struct Concurrent;
impl Injector for Sequential {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
impl Injector for Concurrent {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
async fn work(c: SocketContext, v: Value) -> SocketResult<()> {
    tokio::time::sleep(Duration::from_millis(v["delay"].as_u64().unwrap())).await;
    c.emit("done", v["n"].clone()).await
}
#[socket_gateway(path = "/sequential", mode = "events")]
impl Sequential {
    #[on_event("flood")]
    async fn flood(&self, c: SocketContext, _: Value) -> SocketResult<()> {
        for _ in 0..256 {
            c.send(furnace_rs_socket::Message::Binary(
                vec![7; 3_000_000].into(),
            ))
            .await?;
            tokio::task::yield_now().await;
        }
        Ok(())
    }

    #[on_event("work")]
    async fn work(&self, c: SocketContext, v: Value) -> SocketResult<()> {
        work(c, v).await
    }
    #[on_event("overflow")]
    async fn overflow(&self, c: SocketContext, _: Value) -> SocketResult<()> {
        c.emit("too-large", json!("x".repeat(2000))).await
    }
}
#[socket_gateway(
    path = "/concurrent",
    mode = "events",
    processing = "concurrent",
    max_concurrency = 2
)]
impl Concurrent {
    #[on_event("work")]
    async fn work(&self, c: SocketContext, v: Value) -> SocketResult<()> {
        work(c, v).await
    }
}
#[cauldron(root,socket_gateways=[Sequential,Concurrent])]
struct App;
#[tokio::test]
async fn processing_modes_preserve_sequential_and_allow_bounded_concurrency() {
    let s = support::Server::start::<App>().await;
    for (path, want) in [("/sequential", [0, 1]), ("/concurrent", [1, 0])] {
        let mut c = s.connect(path).await;
        for (n, delay) in [(0, 80), (1, 1)] {
            c.send(tokio_tungstenite::tungstenite::Message::Text(
                json!({"event":"work","data":{"n":n,"delay":delay}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        }
        for n in want {
            let v: Value =
                serde_json::from_str(&c.next().await.unwrap().unwrap().into_text().unwrap())
                    .unwrap();
            assert_eq!(v["data"], n);
        }
        c.close(None).await.unwrap();
    }
    s.stop().await;
}
#[tokio::test]
async fn queue_byte_limit_closes_connection_instead_of_silently_dropping() {
    let s = support::Server::configured::<App>(&[("socket.queue_bytes", "1024")]).await;
    let mut c = s.connect("/sequential").await;
    c.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"event":"overflow","data":null}).to_string().into(),
    ))
    .await
    .unwrap();
    let response = tokio::time::timeout(Duration::from_secs(2), c.next())
        .await
        .unwrap();
    assert!(
        matches!(
            response,
            Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None | Some(Err(_))
        ),
        "{response:?}"
    );
    drop(c);
    s.stop().await;
}
#[tokio::test]
async fn heartbeat_controls_are_received_while_handler_is_busy() {
    let s = support::Server::configured::<App>(&[
        ("socket.heartbeat_interval_ms", "20"),
        ("socket.pong_timeout_ms", "200"),
    ])
    .await;
    let mut c = s.connect("/sequential").await;
    c.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"event":"work","data":{"n":1,"delay":80}})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    let mut saw_ping = false;
    loop {
        let message = tokio::time::timeout(Duration::from_secs(2), c.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        match message {
            tokio_tungstenite::tungstenite::Message::Ping(_) => {
                saw_ping = true;
                c.flush().await.unwrap();
            }
            tokio_tungstenite::tungstenite::Message::Text(_) => break,
            _ => panic!("unexpected heartbeat response"),
        }
    }
    assert!(saw_ping);
    c.close(None)
        .await
        .map_err(|_| SocketError::new("closed"))
        .unwrap();
    s.stop().await;
}

#[tokio::test]
async fn fragmented_messages_enforce_the_assembled_message_limit() {
    use tokio_tungstenite::tungstenite::{
        Message,
        protocol::frame::{
            Frame,
            coding::{Data, OpCode},
        },
    };
    let server = support::Server::configured::<App>(&[("socket.max_message_bytes", "1024")]).await;
    let mut client = server.connect("/sequential").await;
    client
        .send(Message::Frame(Frame::message(
            vec![b'x'; 512],
            OpCode::Data(Data::Text),
            false,
        )))
        .await
        .unwrap();
    client
        .send(Message::Frame(Frame::message(
            vec![b'x'; 513],
            OpCode::Data(Data::Continue),
            true,
        )))
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), client.next())
        .await
        .unwrap();
    assert!(
        matches!(result, Some(Ok(Message::Close(_))) | Some(Err(_)) | None),
        "{result:?}"
    );
    drop(client);
    server.stop().await;
}

#[tokio::test]
async fn a_blocked_writer_is_cancelled_without_exhausting_shutdown_drain() {
    let server = support::Server::configured::<App>(&[("socket.shutdown_timeout_ms", "100")]).await;
    let mut client = server.connect("/sequential").await;
    client
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({"event":"flood","data":null}).to_string().into(),
        ))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    server.stop().await;
    drop(client);
}
