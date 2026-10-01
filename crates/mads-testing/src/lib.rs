//! Focused in-process test fixtures for registered MADS providers.
//!
//! Use `#[mads::test]` on an async test function to obtain its local
//! `test_fixture()` builder. Select one subject, supply its dependencies,
//! and execute assertions inside `run`; lifecycle shutdown is awaited.

mod error;
mod fixture;
pub use error::{TestError, TestResult};
pub use fixture::{SubjectFixture, TestContext, TestFixtureBuilder};

/// Native SeaORM types for preparing a scripted SQLite mock database.
pub mod sea_orm {
    pub use ::sea_orm::{
        ConnectionTrait, DatabaseConnection, DbBackend, DbErr, MockDatabase, MockExecResult,
        MockRow, Statement, Transaction, Value,
    };
}

/// Implementation support for the function-level test macro.
#[doc(hidden)]
pub mod __private {
    pub use tokio;
    /// Creates the builder for a macro-generated function-local entry point.
    pub fn test_fixture() -> crate::TestFixtureBuilder {
        crate::TestFixtureBuilder::new()
    }
}
