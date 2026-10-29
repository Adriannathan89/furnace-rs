//! Fixed public errors that never include credentials or implementation details.
use std::fmt;

/// Result of a socket operation.
pub type SocketResult<T> = Result<T, SocketError>;

/// A stable socket error code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SocketError(pub(crate) &'static str);

impl SocketError {
    /// Creates an application-visible failure with a static code.
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }

    /// Stable machine-readable code.
    pub const fn code(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for SocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for SocketError {}

impl From<serde_json::Error> for SocketError {
    fn from(_: serde_json::Error) -> Self {
        Self("invalid_message")
    }
}
