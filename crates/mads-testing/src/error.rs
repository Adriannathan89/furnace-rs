//! Setup and HTTP transport errors from focused test fixtures.

use std::{error::Error, fmt};

/// The result of fixture setup, execution, or HTTP transport.
pub type TestResult<T> = Result<T, TestError>;

/// A fixture setup, lifecycle, or HTTP transport failure.
#[derive(Debug)]
pub enum TestError {
    /// A MADS graph, provider, route, or lifecycle diagnostic.
    Mads(Box<mads_core::Error>),
    /// The selected chain requires an explicitly supplied mock database.
    MissingMockDatabase,
    /// Only SQLite mock databases are supported.
    UnsupportedMockBackend(sea_orm::DbBackend),
    /// A concrete type was supplied more than once.
    DuplicateSupply(&'static str),
    /// Database connections must be supplied through `mock_database`.
    DirectDatabaseSupply,
    /// An HTTP request could not be constructed.
    Request(axum::http::Error),
    /// A JSON request could not be serialized.
    Serialization(serde_json::Error),
    /// An HTTP service failed to handle a request.
    Service(Box<dyn Error + Send + Sync>),
    /// The response body could not be buffered.
    ResponseBody(axum::Error),
}

impl From<mads_core::Error> for TestError {
    fn from(error: mads_core::Error) -> Self {
        Self::Mads(Box::new(error))
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mads(error) => write!(f, "{error}"),
            Self::MissingMockDatabase => f.write_str("selected dependency chain requires a SQLite MockDatabase; supply it with mock_database"),
            Self::UnsupportedMockBackend(backend) => write!(f, "unsupported mock database backend {backend:?}; expected Sqlite"),
            Self::DuplicateSupply(name) => write!(f, "duplicate fixture supply for {name}"),
            Self::DirectDatabaseSupply => f.write_str("use mock_database to supply DatabaseConnection"),
            Self::Request(error) => write!(f, "request construction failed: {error}"),
            Self::Serialization(error) => write!(f, "JSON serialization failed: {error}"),
            Self::Service(error) => write!(f, "HTTP service failed: {error}"),
            Self::ResponseBody(error) => write!(f, "response body failed: {error}"),
        }
    }
}
impl Error for TestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Mads(error) => Some(error.as_ref()),
            Self::Request(error) => Some(error),
            Self::Serialization(error) => Some(error),
            Self::Service(error) => Some(error.as_ref()),
            Self::ResponseBody(error) => Some(error),
            _ => None,
        }
    }
}
