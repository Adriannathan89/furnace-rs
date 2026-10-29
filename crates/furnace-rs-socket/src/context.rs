//! Connection metadata retained after HTTP upgrade.
use furnace_rs_core::ApplicationContext;
use http::{Extensions, HeaderMap, Method, Uri};
use std::{net::SocketAddr, sync::Arc};

/// Immutable handshake and authentication context for one connection.
#[derive(Clone)]
pub struct SocketHandshake {
    /// Application providers available to this connection.
    pub application: ApplicationContext,
    /// Original request headers; treat credentials as secrets.
    pub headers: HeaderMap,
    /// Original method.
    pub method: Method,
    /// Original URI; may contain a credential.
    pub uri: Uri,
    /// Actual transport peer, without trusting forwarded headers.
    pub peer: Option<SocketAddr>,
    /// Typed authentication and application metadata.
    pub extensions: Arc<Extensions>,
}

/// Unique identity of an active connection, scoped to its serving instance.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ConnectionId(pub(crate) String);

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Managed connection controls passed to gateway methods.
#[derive(Clone)]
pub struct SocketContext {
    pub(crate) transitions: Arc<tokio::sync::Mutex<()>>,
    pub(crate) id: ConnectionId,
    pub(crate) handshake: SocketHandshake,
    pub(crate) hub: crate::SocketGatewayHub,
    #[cfg(feature = "redis")]
    pub(crate) tasks: furnace_rs_common::runtime::RuntimeTasks,
    #[cfg(feature = "redis")]
    pub(crate) shutdown: tokio::sync::watch::Receiver<bool>,
    pub(crate) gateway: Arc<dyn crate::dispatch::SocketGateway>,
    pub(crate) acknowledge_join: bool,
    pub(crate) request_id: Option<String>,
}

impl SocketContext {
    /// Current connection identity.
    pub fn id(&self) -> &ConnectionId {
        &self.id
    }

    /// Original handshake and authenticated principal.
    pub fn handshake(&self) -> &SocketHandshake {
        &self.handshake
    }

    /// Gateway-scoped sending interface.
    pub fn hub(&self) -> &crate::SocketGatewayHub {
        &self.hub
    }

    /// Sends a native message to this connection.
    pub async fn send(&self, message: crate::Message) -> crate::SocketResult<()> {
        self.hub.send(&self.id, message).await
    }

    /// Sends a named response, preserving the current request correlation ID.
    pub async fn emit(&self, event: &str, data: serde_json::Value) -> crate::SocketResult<()> {
        let mut envelope = crate::SocketEnvelope::event(event, data);
        envelope.id = self.request_id.clone();

        self.send(crate::Message::Text(envelope.to_text()?.into()))
            .await
    }
}

#[cfg(feature = "jwt")]
impl SocketHandshake {
    /// Extracts the exact authenticated principal installed during admission.
    pub fn authenticated<P: furnace_rs_common::PassportPrincipal>(
        &self,
    ) -> crate::SocketResult<furnace_rs_common::Authenticated<P>> {
        self.extensions
            .get::<furnace_rs_common::Authenticated<P>>()
            .cloned()
            .ok_or_else(|| crate::SocketError::new("unauthenticated"))
    }
}

impl SocketContext {
    /// Requests room membership after approval by the application's join hook.
    pub async fn join(&self, room: &str, after: Option<&str>) -> crate::SocketResult<()> {
        let _transition = self.transitions.lock().await;
        self.gateway
            .call(
                self.clone(),
                crate::dispatch::SocketInput::Join(room.to_owned()),
            )
            .await?;

        self.hub.join(self, room, after).await
    }

    /// Leaves a room without requiring a new authorization decision.
    pub async fn leave(&self, room: &str) -> crate::SocketResult<()> {
        let _transition = self.transitions.lock().await;

        self.hub.leave(self.id(), room).await
    }
}

impl SocketContext {
    pub(crate) async fn acknowledge_join(&self, room: &str) -> crate::SocketResult<()> {
        if self.acknowledge_join {
            self.emit("furnace.ok", serde_json::json!({"room":room}))
                .await
        } else {
            Ok(())
        }
    }
}
