//! Native SeaORM PostgreSQL connector and opt-in database module.

mod config;
mod connector;
mod lifecycle;

pub use ::sea_orm::{ConnectOptions, DatabaseConnection};
pub use connector::SeaOrmPostgres;

use furnace_rs_core::{Injector, LifecycleResource};

use crate::DatabaseFactory;
use lifecycle::SeaOrmLifecycle;

/// Global module that provides one native SeaORM PostgreSQL connection.
#[furnace_rs_core::cauldron]
pub struct DatabaseCauldron;

fn database_factory() -> DatabaseFactory {
    DatabaseFactory
}

fn sea_orm_postgres_connector(
    config: furnace_rs_core::Config,
) -> furnace_rs_core::Result<SeaOrmPostgres> {
    SeaOrmPostgres::from_config(&config).map_err(Into::into)
}

/// Constructs the native database connection and contributes its lifecycle hook.
pub async fn sea_orm_database(
    factory: DatabaseFactory,
    connector: SeaOrmPostgres,
) -> furnace_rs_core::Result<LifecycleResource<DatabaseConnection>> {
    let database = SeaOrmDatabaseInjector::inject((factory, connector)).await?;
    Ok(SeaOrmDatabaseInjector::lifecycle(database))
}

impl Injector for DatabaseFactory {
    type Dependencies = ();
    async fn inject((): ()) -> furnace_rs_core::Result<Self> {
        Ok(database_factory())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &FACTORY_DESCRIPTOR
    }
}
impl Injector for SeaOrmPostgres {
    type Dependencies = (furnace_rs_core::Config,);
    async fn inject((config,): Self::Dependencies) -> furnace_rs_core::Result<Self> {
        sea_orm_postgres_connector(config)
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &CONNECTOR_DESCRIPTOR
    }
}

/// Constructs a native database connection and registers its readiness/shutdown hook.
pub struct SeaOrmDatabaseInjector;
impl Injector<DatabaseConnection> for SeaOrmDatabaseInjector {
    type Dependencies = (DatabaseFactory, SeaOrmPostgres);
    async fn inject(
        (factory, connector): Self::Dependencies,
    ) -> furnace_rs_core::Result<DatabaseConnection> {
        factory
            .provide(connector)
            .await
            .map_err(furnace_rs_core::Error::from)
    }
    fn lifecycle(database: DatabaseConnection) -> LifecycleResource<DatabaseConnection> {
        LifecycleResource::new(database)
            .with_infrastructure_hook("furnace.persistence.seaorm.postgres", SeaOrmLifecycle)
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &DATABASE_DESCRIPTOR
    }
}

const FACTORY_DESCRIPTOR: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<DatabaseFactory, DatabaseFactory>::DESCRIPTOR
        .with_authored_type_name("DatabaseFactory")
        .with_namespace(module_path!())
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
const CONNECTOR_DESCRIPTOR: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<SeaOrmPostgres, SeaOrmPostgres>::DESCRIPTOR
        .with_authored_type_name("SeaOrmPostgres")
        .with_namespace(module_path!())
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
const DATABASE_DESCRIPTOR: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<DatabaseConnection, SeaOrmDatabaseInjector>::DESCRIPTOR
        .with_authored_type_name("DatabaseConnection")
        .with_visibility(furnace_rs_core::ProviderVisibility::Public)
        .with_namespace(module_path!())
        .with_location(furnace_rs_core::SourceLocation::new(file!(), line!(), column!()));
furnace_rs_core::__private::inventory::submit! { FACTORY_DESCRIPTOR }
furnace_rs_core::__private::inventory::submit! { CONNECTOR_DESCRIPTOR }
furnace_rs_core::__private::inventory::submit! { DATABASE_DESCRIPTOR }

impl furnace_rs_core::Cauldron for DatabaseCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.provide::<DatabaseFactory>()
            .provide::<SeaOrmPostgres>()
            .provide_with::<DatabaseConnection, SeaOrmDatabaseInjector>()
            .export::<DatabaseConnection>()
            .global()
    }
}
