//! Rooted module selection for the native PostgreSQL database provider.
#![cfg(feature = "sea-orm-postgres")]

use furnace_rs_core::{Config, FURNACE008, FURNACE009, Furnace, LifecycleResource};
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};

#[test]
fn native_database_injector_declares_inputs_and_lifecycle_without_io() {
    use furnace_rs_core::Injector;
    use furnace_rs_persistence::{
        DatabaseFactory,
        sea_orm::{SeaOrmDatabaseInjector, SeaOrmPostgres},
    };
    let descriptor = SeaOrmDatabaseInjector::descriptor();
    assert_eq!(
        descriptor.type_id(),
        std::any::TypeId::of::<DatabaseConnection>()
    );
    assert_eq!(
        descriptor.dependencies()[0].type_id(),
        std::any::TypeId::of::<DatabaseFactory>()
    );
    assert_eq!(
        descriptor.dependencies()[1].type_id(),
        std::any::TypeId::of::<SeaOrmPostgres>()
    );
    assert!(descriptor.lifecycle_constructor().is_some());
    assert_eq!(
        descriptor.visibility(),
        furnace_rs_core::ProviderVisibility::Public
    );
}

mod imported {
    use super::*;

    #[furnace_rs_core::cauldron]
    pub struct Root;

    impl furnace_rs_core::Cauldron for Root {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.provide_with::<Repository, RepositoryInjector>()
                .import(DatabaseCauldron)
                .export::<Repository>()
        }
    }

    pub struct Repository;

    pub fn repository(_database: DatabaseConnection) -> Repository {
        Repository
    }
    #[doc = "Explicit constructor for the fixture's provider output."]
    pub struct RepositoryInjector;
    impl furnace_rs_core::Injector<Repository> for RepositoryInjector {
        type Dependencies = (DatabaseConnection,);
        async fn inject(
            (dependency_0,): Self::Dependencies,
        ) -> furnace_rs_core::Result<Repository> {
            Ok(repository(dependency_0))
        }
        fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
            &__FURNACE_INJECTOR_DESCRIPTOR_REPOSITORY
        }
    }
    const __FURNACE_INJECTOR_DESCRIPTOR_REPOSITORY: furnace_rs_core::ProviderDescriptor =
        furnace_rs_core::__private::InjectorMetadata::<Repository, RepositoryInjector>::DESCRIPTOR
            .with_authored_type_name("Repository")
            .with_namespace(module_path!())
            .with_visibility(furnace_rs_core::ProviderVisibility::Public)
            .with_location(furnace_rs_core::SourceLocation::new(
                file!(),
                line!(),
                column!(),
            ));
    furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_REPOSITORY }
}

mod unimported {
    #[furnace_rs_core::cauldron]
    pub struct Root;

    impl furnace_rs_core::Cauldron for Root {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            furnace_rs_core::CauldronRegistration::new(self)
        }
    }
}

mod private_infrastructure {
    use furnace_rs_persistence::{DatabaseFactory, sea_orm::SeaOrmPostgres};

    #[furnace_rs_core::burner]
    pub struct Consumer {
        _factory: DatabaseFactory,
        _connector: SeaOrmPostgres,
    }

    #[furnace_rs_core::cauldron]
    pub struct Root;

    impl furnace_rs_core::Cauldron for Root {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.provide::<Consumer>().import(super::DatabaseCauldron)
        }
    }
}

#[test]
fn database_furnace_keeps_factory_and_connector_private() {
    let mut builder = Furnace::builder_with_config(Config::empty());
    builder.root::<private_infrastructure::Root>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    let inaccessible: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == FURNACE009)
        .collect();
    assert_eq!(inaccessible.len(), 2, "{:?}", analysis.diagnostics());
}

mod duplicate {
    pub mod other {
        use super::super::DatabaseConnection;

        #[furnace_rs_core::cauldron]
        pub struct OtherCauldron;

        impl furnace_rs_core::Cauldron for OtherCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.provide_with::<DatabaseConnection, furnace_rs_persistence::sea_orm::SeaOrmDatabaseInjector>()
                    .export::<DatabaseConnection>()
            }
        }
    }

    pub mod app {
        use super::{super::DatabaseCauldron, other::OtherCauldron};

        #[furnace_rs_core::cauldron]
        pub struct Root;

        impl furnace_rs_core::Cauldron for Root {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(DatabaseCauldron).import(OtherCauldron)
            }
        }
    }
}

#[test]
fn imported_module_selects_only_native_database_without_parsing_configuration() {
    let mut builder = Furnace::builder_with_config(Config::empty());
    builder.root::<imported::Root>().unwrap();
    for _ in 0..2 {
        let analysis = builder.analyze();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
        assert!(analysis.graph().provider::<DatabaseConnection>().is_some());
        let ownership = analysis.cauldron_graph().unwrap().provider_ownership();
        assert!(
            ownership.iter().any(|item| {
                item.provider_type_name() == "DatabaseConnection"
                    && item.cauldron_type_name() == Some(std::any::type_name::<DatabaseCauldron>())
            }),
            "{:?}",
            ownership
                .iter()
                .map(|item| (item.provider_type_name(), item.cauldron_type_name()))
                .collect::<Vec<_>>()
        );
        assert!(
            analysis
                .graph()
                .provider::<imported::Repository>()
                .is_some()
        );
        assert!(
            analysis
                .graph()
                .provider::<LifecycleResource<DatabaseConnection>>()
                .is_none()
        );
    }
}

#[tokio::test]
async fn unimported_root_builds_without_database_configuration() {
    let mut builder = Furnace::builder_with_config(Config::empty());
    builder.root::<unimported::Root>().unwrap();
    assert!(
        builder
            .analyze()
            .graph()
            .provider::<DatabaseConnection>()
            .is_none()
    );
    builder.build().await.unwrap();
}

#[test]
fn duplicate_native_database_reports_conflicting_furnace_ownership() {
    let mut builder = Furnace::builder_with_config(Config::empty());
    builder.root::<duplicate::app::Root>().unwrap();
    let analysis = builder.analyze();
    assert_eq!(
        analysis.diagnostics()[0].code(),
        FURNACE008,
        "{:?}",
        analysis.diagnostics()
    );
}
