//! Dynamic room authorization, lifecycle, and fan-out.
mod support;
use furnace_rs_core::{Injector, cauldron};
use furnace_rs_socket::{RoomMode, SocketContext, SocketResult, socket_gateway};
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
    #[on_connect]
    async fn connect(&self, c: SocketContext) -> SocketResult<()> {
        c.hub().create_room("public", RoomMode::Live).await
    }
    #[on_join]
    async fn join(&self, _: SocketContext, room: String) -> SocketResult<()> {
        if room == "public" {
            Ok(())
        } else {
            Err(furnace_rs_socket::SocketError::new("join_denied"))
        }
    }
    #[on_event("say")]
    async fn say(&self, c: SocketContext, data: Value) -> SocketResult<()> {
        c.hub()
            .emit_room("public", "message", data)
            .await
            .map(|_| ())
    }
    #[on_event("delete")]
    async fn delete(&self, c: SocketContext, _: Value) -> SocketResult<()> {
        c.hub().delete_room("public").await
    }
}
#[cauldron(root,socket_gateways=[Chat])]
struct App;
type Client =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
async fn send(c: &mut Client, event: &str, data: Value) {
    c.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"event":event,"id":"request","data":data})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
}
async fn recv(c: &mut Client) -> Value {
    serde_json::from_str(
        &tokio::time::timeout(std::time::Duration::from_secs(2), c.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn built_in_join_requires_approval_and_delete_requires_empty_room() {
    let server = support::Server::start::<App>().await;
    let mut a = server.connect("/chat").await;
    let mut b = server.connect("/chat").await;
    send(&mut a, "furnace.join", json!({"room":"private"})).await;
    assert_eq!(recv(&mut a).await["data"]["code"], "join_denied");
    for c in [&mut a, &mut b] {
        send(c, "furnace.join", json!({"room":"public"})).await;
        assert_eq!(recv(c).await["event"], "furnace.ok");
    }
    send(&mut a, "say", json!({"text":"hello"})).await;
    assert_eq!(recv(&mut a).await["event"], "message");
    assert_eq!(recv(&mut b).await["data"]["text"], "hello");
    send(&mut a, "delete", json!({})).await;
    assert_eq!(recv(&mut a).await["data"]["code"], "room_not_empty");
    for c in [&mut a, &mut b] {
        send(c, "furnace.leave", json!({"room":"public"})).await;
        assert_eq!(recv(c).await["event"], "furnace.ok");
    }
    a.close(None).await.unwrap();
    b.close(None).await.unwrap();
    server.stop().await;
}
#[tokio::test]
async fn local_room_definitions_reject_conflicts_and_replay_without_redis() {
    let hub = furnace_rs_socket::SocketHub::default().gateway("/test");
    hub.create_room("x", RoomMode::Live).await.unwrap();
    hub.create_room("x", RoomMode::Live).await.unwrap();
    assert!(
        hub.create_room(
            "x",
            RoomMode::Replayable {
                max_age: std::time::Duration::from_secs(60),
                max_messages: 100
            }
        )
        .await
        .is_err()
    );
    hub.delete_room("x").await.unwrap();
    assert_eq!(
        hub.delete_room("x").await.unwrap_err().code(),
        "room_not_found"
    );
}
