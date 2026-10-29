//! Redacted transport errors and stable gRPC diagnostic categories.
use furnace_rs_core::DiagnosticCode;
use std::{error::Error as StdError, fmt};

/// Invalid gRPC service registration or protobuf descriptors.
pub const FURNACE150: DiagnosticCode = DiagnosticCode::new("FURNACE150");

/// Invalid source-aware gRPC configuration.
pub const FURNACE151: DiagnosticCode = DiagnosticCode::new("FURNACE151");

/// Invalid gRPC protection metadata or selected strategy.
pub const FURNACE152: DiagnosticCode = DiagnosticCode::new("FURNACE152");

/// Invalid named client or failed connection readiness.
pub const FURNACE153: DiagnosticCode = DiagnosticCode::new("FURNACE153");

/// A client operation or credential preparation result.
pub type GrpcClientResult<T> = Result<T, GrpcClientError>;

/// The category of a locally reported client error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum GrpcClientErrorKind {
    /// An outbound credential cannot form valid Bearer metadata.
    InvalidCredential,
    /// The application's credential provider failed.
    TokenProvider,
    /// An underlying gRPC transport operation failed.
    Transport,
    /// The managed channel has shut down.
    Closed,
}

/// A redacted error retaining an operational cause for explicit inspection.
pub struct GrpcClientError {
    kind: GrpcClientErrorKind,
    source: Option<RedactedCause>,
}

impl GrpcClientError {
    /// Creates an outbound-credential rejection without retaining the token.
    pub const fn invalid_credential() -> Self {
        Self {
            kind: GrpcClientErrorKind::InvalidCredential,
            source: None,
        }
    }

    /// Creates a redacted application token-provider failure.
    pub fn token_provider(error: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            kind: GrpcClientErrorKind::TokenProvider,
            source: Some(RedactedCause(Box::new(error))),
        }
    }

    /// Creates a redacted transport failure with an inspectable source.
    pub fn transport(error: impl StdError + Send + Sync + 'static) -> Self {
        Self {
            kind: GrpcClientErrorKind::Transport,
            source: Some(RedactedCause(Box::new(error))),
        }
    }

    /// Creates an error for use of a shut-down managed channel.
    pub const fn closed() -> Self {
        Self {
            kind: GrpcClientErrorKind::Closed,
            source: None,
        }
    }

    /// Returns the original operational cause for deliberate inspection.
    ///
    /// The standard error chain uses a redacted boundary so transport libraries
    /// cannot convert a nested application status into a credential-bearing status.
    pub fn cause(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        self.source.as_ref().map(|source| source.0.as_ref())
    }

    /// Returns the stable category.
    pub const fn kind(&self) -> GrpcClientErrorKind {
        self.kind
    }
}

impl fmt::Display for GrpcClientError {
    /// Formats the stable client error category without exposing its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.kind {
            GrpcClientErrorKind::InvalidCredential => "invalid gRPC credential",
            GrpcClientErrorKind::TokenProvider => "gRPC token provider failed",
            GrpcClientErrorKind::Transport => "gRPC transport failed",
            GrpcClientErrorKind::Closed => "gRPC channel is closed",
        })
    }
}

impl fmt::Debug for GrpcClientError {
    /// Formats the stable client error category without exposing its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GrpcClientError")
            .field("kind", &self.kind)
            .field("has_source", &self.source.is_some())
            .finish()
    }
}

impl StdError for GrpcClientError {
    /// Returns only the redacted boundary around the underlying operational cause.
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|s| s as _)
    }
}

struct RedactedCause(Box<dyn StdError + Send + Sync>);

impl fmt::Debug for RedactedCause {
    /// Formats the stable client error category without exposing its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RedactedCause(..)")
    }
}

impl fmt::Display for RedactedCause {
    /// Formats the stable client error category without exposing its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("gRPC operation failed")
    }
}

impl StdError for RedactedCause {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_status_cause_requires_deliberate_inspection() {
        let error =
            GrpcClientError::token_provider(tonic::Status::internal("private-token-sentinel"));
        assert!(!format!("{error:?} {error}").contains("private-token-sentinel"));
        assert!(
            !error
                .source()
                .unwrap()
                .to_string()
                .contains("private-token-sentinel")
        );
        assert!(error.source().unwrap().source().is_none());
        assert!(
            error
                .cause()
                .unwrap()
                .downcast_ref::<tonic::Status>()
                .is_some()
        );
        assert!(
            !tonic::Status::from_error(Box::new(error))
                .message()
                .contains("private-token-sentinel")
        );
    }
}
