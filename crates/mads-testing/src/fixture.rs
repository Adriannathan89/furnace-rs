//! Focused fixture setup and scoped lifecycle ownership.

use crate::{TestError, TestResult};
use futures_util::FutureExt;
use mads_core::{ApplicationContext, Mads, MadsBuilder};
use sea_orm::{DatabaseConnection, DbBackend, MockDatabase, MockDatabaseTrait};
use std::{
    any::TypeId, collections::HashSet, future::Future, marker::PhantomData,
    panic::AssertUnwindSafe, sync::Arc,
};

/// Supplies concrete values for one registered provider dependency chain.
///
/// Obtain this builder from `test_fixture()` inside a `#[mads::test]` function.
/// Setup errors are returned by the selected fixture's `run` method.
#[must_use]
pub struct TestFixtureBuilder {
    builder: MadsBuilder,
    supplied: HashSet<TypeId>,
    error: Option<TestError>,
}
impl TestFixtureBuilder {
    pub(crate) fn new() -> Self {
        let mut builder = Mads::builder();
        builder.__test_require_provided::<DatabaseConnection>();
        Self {
            builder,
            supplied: HashSet::new(),
            error: None,
        }
    }

    /// Supplies a scripted SQLite mock as the native SeaORM connection.
    pub fn mock_database(mut self, mock: MockDatabase) -> Self {
        if self.error.is_some() {
            return self;
        }
        let backend = mock.get_database_backend();
        if backend != DbBackend::Sqlite {
            self.error = Some(TestError::UnsupportedMockBackend(backend));
        } else if !self.supplied.insert(TypeId::of::<DatabaseConnection>()) {
            self.error = Some(TestError::DuplicateSupply(std::any::type_name::<
                DatabaseConnection,
            >()));
        } else if let Err(error) = self.builder.provide(mock.into_connection()) {
            self.error = Some(error.into());
        }
        self
    }

    /// Supplies one concrete dependency, replacing its registered constructor.
    pub fn provide<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        if self.error.is_some() {
            return self;
        }
        if TypeId::of::<T>() == TypeId::of::<DatabaseConnection>() {
            self.error = Some(TestError::DirectDatabaseSupply);
        } else if !self.supplied.insert(TypeId::of::<T>()) {
            self.error = Some(TestError::DuplicateSupply(std::any::type_name::<T>()));
        } else if let Err(error) = self.builder.provide(value) {
            self.error = Some(error.into());
        }
        self
    }

    /// Selects a registered subject and its transitive dependencies.
    pub fn subject<T: Send + Sync + 'static>(self) -> SubjectFixture<T> {
        SubjectFixture {
            setup: self,
            subject: PhantomData,
        }
    }

    pub(crate) async fn build<T: Send + Sync + 'static>(mut self) -> TestResult<Mads> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.builder.__test_focus::<T>()?;
        let analysis = self.builder.analyze();
        if !self.supplied.contains(&TypeId::of::<DatabaseConnection>())
            && (TypeId::of::<T>() == TypeId::of::<DatabaseConnection>()
                || analysis.graph().providers().iter().any(|provider| {
                    provider.declared_dependencies().iter().any(|dependency| {
                        dependency.type_id() == TypeId::of::<DatabaseConnection>()
                    })
                }))
        {
            return Err(TestError::MissingMockDatabase);
        }
        Ok(self.builder.build().await?)
    }
}

/// A registered subject ready for scoped execution.
#[must_use]
pub struct SubjectFixture<T> {
    setup: TestFixtureBuilder,
    subject: PhantomData<fn() -> T>,
}
impl<T: Send + Sync + 'static> SubjectFixture<T> {
    /// Builds and starts the chain, runs the body, and awaits shutdown.
    ///
    /// Unwinding body panics resume after shutdown. Cancellation and process
    /// abort cannot guarantee asynchronous cleanup.
    pub async fn run<F, Fut>(self, body: F) -> TestResult<()>
    where
        F: FnOnce(TestContext) -> Fut,
        Fut: Future<Output = ()>,
    {
        let app = self.setup.build::<T>().await?;
        let context = TestContext::new(app.context().clone());
        run_scoped(app, context, body).await
    }
}

/// Resolves the fixture's immutable application-scoped providers.
#[derive(Clone)]
pub struct TestContext {
    context: ApplicationContext,
}
impl TestContext {
    pub(crate) fn new(context: ApplicationContext) -> Self {
        Self { context }
    }
    /// Resolves a selected or explicitly supplied concrete provider.
    #[allow(clippy::result_large_err)]
    pub fn resolve<T: Send + Sync + 'static>(&self) -> mads_core::Result<Arc<T>> {
        self.context.resolve::<T>()
    }
}

pub(crate) async fn run_scoped<C, F, Fut>(mut app: Mads, context: C, body: F) -> TestResult<()>
where
    F: FnOnce(C) -> Fut,
    Fut: Future<Output = ()>,
{
    app.start().await?;
    // Construct the body's future inside the unwind boundary too.
    let outcome = AssertUnwindSafe(async move { body(context).await })
        .catch_unwind()
        .await;
    let shutdown = app.shutdown().await;
    match outcome {
        Ok(()) => shutdown.map_err(Into::into),
        Err(panic) => {
            if let Err(error) = shutdown {
                eprintln!("fixture shutdown failed after test panic: {error}");
                let mut source = std::error::Error::source(&error);
                while let Some(cause) = source {
                    eprintln!("caused by: {cause}");
                    source = cause.source();
                }
            }
            std::panic::resume_unwind(panic)
        }
    }
}
