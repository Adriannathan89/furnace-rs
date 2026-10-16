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
