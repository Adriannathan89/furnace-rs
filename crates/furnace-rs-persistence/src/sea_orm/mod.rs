//! Native SeaORM PostgreSQL connector and opt-in database module.

mod config;
mod connector;
mod lifecycle;

pub use ::sea_orm::{ConnectOptions, DatabaseConnection};
pub use connector::SeaOrmPostgres;

use furnace_rs_core::LifecycleResource;

use crate::DatabaseFactory;
use lifecycle::SeaOrmLifecycle;

/// Global module that provides one native SeaORM PostgreSQL connection.
#[furnace_rs_core::cauldron]
pub struct DatabaseCauldron;

#[furnace_rs_core::element]
fn database_factory() -> DatabaseFactory {
    DatabaseFactory
}

#[furnace_rs_core::element]
fn sea_orm_postgres_connector(
    config: furnace_rs_core::Config,
) -> furnace_rs_core::Result<SeaOrmPostgres> {
    SeaOrmPostgres::from_config(&config).map_err(Into::into)
}

/// Constructs the native database connection and contributes its lifecycle hook.
#[furnace_rs_core::element(lifecycle)]
pub async fn sea_orm_database(
    factory: DatabaseFactory,
    connector: SeaOrmPostgres,
) -> furnace_rs_core::Result<LifecycleResource<DatabaseConnection>> {
    let database = factory
        .provide(connector)
        .await
        .map_err(furnace_rs_core::Error::from)?;
    Ok(LifecycleResource::new(database)
        .with_infrastructure_hook("furnace.persistence.seaorm.postgres", SeaOrmLifecycle))
}

impl furnace_rs_core::Cauldron for DatabaseCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.provide::<DatabaseFactory>()
            .provide::<SeaOrmPostgres>()
            .provide::<DatabaseConnection>()
            .export::<DatabaseConnection>()
            .global()
    }
}
