//! Macro registration and managed dispatch over real sockets.
mod support;
use furnace_rs_core::{Injector, cauldron};
use furnace_rs_socket::{
    SocketContext, SocketHandshake, SocketResult, WebSocket, socket_endpoint, socket_gateway,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
struct Chat;
impl Injector for Chat {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[socket_gateway(path = "/chat", mode = "events")]
impl Chat {
    #[on_event("echo")]
    async fn echo(&self, ctx: SocketContext, data: Value) -> SocketResult<()> {
        ctx.emit("echo", data).await
    }
    #[on_event("fail")]
    async fn fail(&self, _: SocketContext, _: Value) -> SocketResult<()> {
        Err(furnace_rs_socket::SocketError::new("denied"))
    }
}
struct Raw;
impl Injector for Raw {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[socket_gateway(path = "/raw", mode = "raw")]
impl Raw {
    #[on_connect]
    async fn connected(&self, c: SocketContext) -> SocketResult<()> {
        c.hub()
            .create_room("raw", furnace_rs_socket::RoomMode::Live)
            .await?;
        c.join("raw", None).await
    }
    #[on_join]
    async fn authorize(&self, _: SocketContext, _: String) -> SocketResult<()> {
        Ok(())
    }

    #[on_text]
    async fn text(&self, ctx: SocketContext, data: String) -> SocketResult<()> {
        ctx.send(furnace_rs_socket::Message::Text(data.into()))
            .await
    }
    #[on_binary]
    async fn binary(&self, ctx: SocketContext, data: axum::body::Bytes) -> SocketResult<()> {
        ctx.send(furnace_rs_socket::Message::Binary(data)).await
    }
}
struct Owned;
impl Injector for Owned {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[socket_endpoint(path = "/owned")]
impl Owned {
    async fn handle(&self, _: SocketHandshake, mut socket: WebSocket) -> SocketResult<()> {
        let _ = socket
            .send(furnace_rs_socket::Message::Text("owned".into()))
            .await;
        Ok(())
    }
}
#[cauldron(root,socket_gateways=[Chat,Raw],socket_endpoints=[Owned])]
struct App;
#[tokio::test]
async fn macros_dispatch_events_raw_and_owned_and_keep_errors_recoverable() {
    let server = support::Server::start::<App>().await;
    let mut client = server.connect("/chat").await;
    for (event, want) in [
        ("echo", "echo"),
        ("missing", "furnace.error"),
        ("fail", "furnace.error"),
        ("echo", "echo"),
    ] {
        client
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({"event":event,"id":"r1","data":{"text":"hi"}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        let text = client.next().await.unwrap().unwrap().into_text().unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["event"], want);
        assert_eq!(value["id"], "r1");
    }
    let mut raw = server.connect("/raw").await;
    for message in [
        tokio_tungstenite::tungstenite::Message::Text("hello".into()),
        tokio_tungstenite::tungstenite::Message::Binary(vec![1, 2].into()),
    ] {
        raw.send(message.clone()).await.unwrap();
        assert_eq!(raw.next().await.unwrap().unwrap(), message);
    }
    let mut owned = server.connect("/owned").await;
    assert_eq!(
        owned.next().await.unwrap().unwrap().into_text().unwrap(),
        "owned"
    );
    client.close(None).await.unwrap();
    raw.close(None).await.unwrap();
    drop(owned);
    server.stop().await;
}
