//! Shared entity-storage error contracts without database connector providers.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::fmt;

/// Classifies an entity storage operation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StorageErrorKind {
    /// Page arguments were zero or their offset could not be represented.
    InvalidPagination,
    /// Pagination was requested without explicit ordering.
    MissingOrder,
    /// The update target did not exist.
    NotFound,
    /// A native database operation failed.
    Database,
}

/// A redacted storage failure with an optional native database cause.
pub struct StorageError {
    kind: StorageErrorKind,
    source: Option<sea_orm::DbErr>,
}

impl StorageError {
    /// Returns the stable category without inspecting native error messages.
    pub const fn kind(&self) -> StorageErrorKind {
        self.kind
    }

    /// Creates a storage failure without a native database cause.
    #[doc(hidden)]
    pub fn __new(kind: StorageErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Creates a storage failure retaining a native cause without exposing it in formatting.
    #[doc(hidden)]
    pub fn __with_source(kind: StorageErrorKind, source: sea_orm::DbErr) -> Self {
        Self {
            kind,
            source: Some(source),
        }
    }
}

impl From<sea_orm::DbErr> for StorageError {
    /// Wraps a native database error as a redacted database storage failure.
    fn from(source: sea_orm::DbErr) -> Self {
        Self::__with_source(StorageErrorKind::Database, source)
    }
}

impl fmt::Display for StorageError {
    /// Formats only the stable failure category.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "storage operation failed ({:?})", self.kind)
    }
}

impl fmt::Debug for StorageError {
    /// Formats the category and whether a native cause exists, without showing the cause.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StorageError")
            .field("kind", &self.kind)
            .field("has_source", &self.source.is_some())
            .finish()
    }
}

impl std::error::Error for StorageError {
    /// Returns the native cause for explicit error-chain inspection.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &dyn std::error::Error)
    }
}

/// The result of an entity storage operation.
pub type StorageResult<T> = Result<T, StorageError>;
