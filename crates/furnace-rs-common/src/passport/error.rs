//! Transport-independent redacted Passport errors.

use std::fmt;

/// The result type used by Passport strategy APIs.
pub type PassportResult<T> = std::result::Result<T, PassportError>;

/// A stable category for a Passport failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PassportErrorKind {
    /// Authentication was rejected by an application strategy.
    Rejected,
    /// A verified principal did not satisfy a route policy.
    Forbidden,
    /// A strategy or framework operation failed unexpectedly.
    Internal,
}

/// A normalized Passport error whose formatting never exposes sensitive data.
#[non_exhaustive]
pub struct PassportError {
    kind: PassportErrorKind,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl PassportError {
    /// Creates an expected authentication rejection.
    #[must_use]
    pub const fn reject() -> Self {
        Self {
            kind: PassportErrorKind::Rejected,
            source: None,
        }
    }

    pub(crate) const fn forbidden() -> Self {
        Self {
            kind: PassportErrorKind::Forbidden,
            source: None,
        }
    }

    /// Creates an operational failure while retaining its source for inspection.
    pub fn internal<E>(source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind: PassportErrorKind::Internal,
            source: Some(Box::new(source)),
        }
    }

    /// Returns the stable category of this error.
    #[must_use]
    pub const fn kind(&self) -> PassportErrorKind {
        self.kind
    }
}

impl fmt::Debug for PassportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PassportError")
            .field("kind", &self.kind)
            .field("has_source", &self.source.is_some())
            .finish()
    }
}

impl fmt::Display for PassportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            PassportErrorKind::Rejected => "authentication was rejected",
            PassportErrorKind::Forbidden => "access was denied",
            PassportErrorKind::Internal => "Passport operation failed",
        })
    }
}

impl std::error::Error for PassportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}

impl From<crate::JwtError> for PassportError {
    fn from(error: crate::JwtError) -> Self {
        match error.kind() {
            crate::JwtErrorKind::InvalidConfiguration
            | crate::JwtErrorKind::InvalidKeyMaterial
            | crate::JwtErrorKind::UnavailableSigningKey
            | crate::JwtErrorKind::Serialization => Self::internal(error),
            _ => Self::reject(),
        }
    }
}
