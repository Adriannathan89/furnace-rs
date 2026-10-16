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
    pub(crate) id: ConnectionId,
    pub(crate) handshake: SocketHandshake,
    pub(crate) hub: crate::SocketGatewayHub,
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
