//! Explicit named-event envelopes; raw endpoints never parse this format.
use crate::{SocketError, SocketResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Application payload and optional transport metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocketEnvelope {
    /// Event handler name.
    pub event: String,
    /// Optional request correlation identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// JSON application payload.
    pub data: Value,
    /// Server-side room identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    /// Server-side replay cursor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl SocketEnvelope {
    /// Parses an envelope, rejecting empty event names and duplicate fields.
    pub fn parse(text: &str) -> SocketResult<Self> {
        let value: Self = serde_json::from_str(text)?;

        if value.event.is_empty() {
            return Err(SocketError::new("invalid_message"));
        }

        Ok(value)
    }

    /// Serializes this envelope to WebSocket text.
    pub fn to_text(&self) -> SocketResult<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Creates an application event.
    pub fn event(event: impl Into<String>, data: Value) -> Self {
        Self {
            event: event.into(),
            data,
            id: None,
            room: None,
            cursor: None,
        }
    }
}

/// Explicit endpoint protocol selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointMode {
    /// Native text/binary messages.
    Raw,
    /// Named JSON envelopes.
    Events,
}

/// How messages on one connection are dispatched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessingMode {
    /// One handler at a time, in arrival order.
    Sequential,
    /// Bounded work whose completion order is unspecified.
    Concurrent {
        /// Maximum active handlers on one connection.
        max_concurrency: usize,
    },
}
