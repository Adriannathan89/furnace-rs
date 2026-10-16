//! Runnable socket gateways using only public Furnace APIs.
use furnace::prelude::*;
#[cfg(feature = "jwt")]
use furnace::{ClaimsPrincipal, PassportPrincipal};
use serde_json::Value;
/// Example JWT payload.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Claims {
    /// Application user identity.
    pub id: u64,
}
#[cfg(feature = "jwt")]
impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
#[cfg(feature = "jwt")]
#[furnace::guard(strategy="jwt",principal=ClaimsPrincipal<Claims>)]
struct Protected;
/// Chat gateway registered in the selected application root.
pub struct Chat;
impl Injector for Chat {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace::core::Result<Self> {
        Ok(Self)
    }
}
#[cfg_attr(feature="jwt",socket_gateway(path="/chat",mode="events",guard=Protected))]
#[cfg_attr(not(feature = "jwt"), socket_gateway(path = "/chat", mode = "events"))]
impl Chat {
    #[on_connect]
    async fn connected(&self, c: SocketContext) -> SocketResult<()> {
        let mode = if c.handshake().application.config().get("demo.replay") == Some("true") {
            RoomMode::Replayable {
                max_age: std::time::Duration::from_secs(3600),
                max_messages: 1000,
            }
        } else {
            RoomMode::Live
        };
        c.hub().create_room("lobby", mode).await
    }
    #[on_join]
    async fn authorize_join(&self, _: SocketContext, room: String) -> SocketResult<()> {
        if room == "lobby" {
            Ok(())
        } else {
            Err(SocketError::new("join_denied"))
        }
    }
    #[on_event("chat.send")]
    async fn send_chat(&self, c: SocketContext, data: Value) -> SocketResult<()> {
        c.hub()
            .emit_room("lobby", "chat.message", data)
            .await
            .map(|_| ())
    }
}
/// Native frame gateway.
pub struct Raw;
impl Injector for Raw {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace::core::Result<Self> {
        Ok(Self)
    }
}
#[socket_gateway(path = "/raw", mode = "raw")]
impl Raw {
    #[on_text]
    async fn echo(&self, c: SocketContext, text: String) -> SocketResult<()> {
        c.send(furnace::socket::Message::Text(text.into())).await
    }
}
/// Application-defined connection loop.
pub struct Owned;
impl Injector for Owned {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace::core::Result<Self> {
        Ok(Self)
    }
}
#[socket_endpoint(path = "/owned")]
impl Owned {
    async fn serve(
        &self,
        _: SocketHandshake,
        mut socket: furnace::socket::WebSocket,
    ) -> SocketResult<()> {
        while let Some(Ok(message)) = socket.recv().await {
            match message {
                furnace::socket::Message::Text(_) | furnace::socket::Message::Binary(_) => socket
                    .send(message)
                    .await
                    .map_err(|_| SocketError::new("closed"))?,
                furnace::socket::Message::Close(_) => break,
                _ => {}
            }
        }
        Ok(())
    }
}
/// Selected application root containing all three endpoint styles.
#[cauldron(root,socket_gateways=[Chat,Raw],socket_endpoints=[Owned])]
pub struct App;
