//! Shared application exercising the public room API across separate servers.
#![allow(dead_code)]
use furnace_rs_core::{Injector, cauldron};
use furnace_rs_socket::{RoomMode, SocketContext, SocketResult, socket_gateway};
use serde_json::Value;
pub struct Chat;
impl Injector for Chat {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[socket_gateway(path = "/chat", mode = "events")]
impl Chat {
    #[on_event("delete")]
    async fn delete(&self, c: SocketContext, _: Value) -> SocketResult<()> {
        c.hub().delete_room("room").await?;
        c.emit("deleted", serde_json::json!(true)).await
    }
    #[on_event("direct")]
    async fn direct(&self, c: SocketContext, _: Value) -> SocketResult<()> {
        c.emit("direct", serde_json::json!(true)).await
    }
    #[on_event("create")]
    async fn create(&self, c: SocketContext, _: Value) -> SocketResult<()> {
        self.connect(c.clone()).await?;
        c.emit("created", serde_json::json!(true)).await
    }

    #[on_connect]
    async fn connect(&self, c: SocketContext) -> SocketResult<()> {
        let mode = if c.handshake().application.config().get("test.replay") == Some("true") {
            RoomMode::Replayable {
                max_age: std::time::Duration::from_millis(
                    c.handshake()
                        .application
                        .config()
                        .get("test.max_age_ms")
                        .unwrap_or("60000")
                        .parse()
                        .unwrap(),
                ),
                max_messages: c
                    .handshake()
                    .application
                    .config()
                    .get("test.max_messages")
                    .unwrap_or("2")
                    .parse()
                    .unwrap(),
            }
        } else {
            RoomMode::Live
        };
        c.hub().create_room("room", mode).await
    }
    #[on_join]
    async fn join(&self, _: SocketContext, _: String) -> SocketResult<()> {
        Ok(())
    }
    #[on_event("say")]
    async fn say(&self, c: SocketContext, data: Value) -> SocketResult<()> {
        c.hub().emit_room("room", "message", data).await.map(|_| ())
    }
}
#[cauldron(root,socket_gateways=[Chat])]
pub struct App;
pub type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
pub async fn send(c: &mut Client, event: &str, data: Value) {
    use futures_util::SinkExt;
    c.send(tokio_tungstenite::tungstenite::Message::Text(
        serde_json::json!({"event":event,"id":"r","data":data})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
}
pub async fn recv(c: &mut Client) -> Value {
    use futures_util::StreamExt;
    serde_json::from_str(
        &tokio::time::timeout(std::time::Duration::from_secs(3), c.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap(),
    )
    .unwrap()
}
