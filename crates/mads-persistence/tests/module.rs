//! Rooted module selection for the native PostgreSQL database provider.
#![cfg(feature = "sea-orm-postgres")]

use mads_core::{Config, LifecycleResource, MADS008, MADS009, Mads};
use mads_persistence::sea_orm::{DatabaseConnection, DatabaseModule};

mod imported {
    use super::*;

    #[mads_core::furnace]
    pub struct Root;

    impl mads_core::Furnace for Root {
        fn register(self) -> mads_core::FurnaceRegistration<Self> {
            self.provide::<Repository>()
                .import(DatabaseModule)
                .export::<Repository>()
        }
    }

    pub struct Repository;

    #[mads_core::element]
    pub fn repository(_database: DatabaseConnection) -> Repository {
        Repository
    }
}

mod unimported {
    #[mads_core::furnace]
    pub struct Root;

    impl mads_core::Furnace for Root {
        fn register(self) -> mads_core::FurnaceRegistration<Self> {
            mads_core::FurnaceRegistration::new(self)
        }
    }
}

mod private_infrastructure {
    use mads_persistence::{DatabaseFactory, sea_orm::SeaOrmPostgres};

    #[mads_core::burner]
    pub struct Consumer {
        _factory: DatabaseFactory,
        _connector: SeaOrmPostgres,
    }

    #[mads_core::furnace]
    pub struct Root;

    impl mads_core::Furnace for Root {
        fn register(self) -> mads_core::FurnaceRegistration<Self> {
            self.provide::<Consumer>().import(super::DatabaseModule)
        }
    }
}

#[test]
fn database_furnace_keeps_factory_and_connector_private() {
    let mut builder = Mads::builder_with_config(Config::empty());
    builder.root::<private_infrastructure::Root>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    let inaccessible: Vec<_> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == MADS009)
        .collect();
    assert_eq!(inaccessible.len(), 2, "{:?}", analysis.diagnostics());
}

mod duplicate {
    pub mod other {
        use super::super::DatabaseConnection;

        #[mads_core::furnace]
        pub struct OtherModule;

        impl mads_core::Furnace for OtherModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.provide::<DatabaseConnection>()
                    .export::<DatabaseConnection>()
            }
        }
    }

    pub mod app {
        use super::{super::DatabaseModule, other::OtherModule};

        #[mads_core::furnace]
        pub struct Root;

        impl mads_core::Furnace for Root {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(DatabaseModule).import(OtherModule)
            }
        }
    }
}

#[test]
fn imported_module_selects_only_native_database_without_parsing_configuration() {
    let mut builder = Mads::builder_with_config(Config::empty());
    builder.root::<imported::Root>().unwrap();
    for _ in 0..2 {
        let analysis = builder.analyze();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
        assert!(analysis.graph().provider::<DatabaseConnection>().is_some());
        let ownership = analysis.module_graph().unwrap().provider_ownership();
        assert!(
            ownership.iter().any(|item| {
                item.provider_type_name() == "DatabaseConnection"
                    && item.module_type_name() == Some(std::any::type_name::<DatabaseModule>())
            }),
            "{:?}",
            ownership
                .iter()
                .map(|item| (item.provider_type_name(), item.module_type_name()))
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
    let mut builder = Mads::builder_with_config(Config::empty());
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
    let mut builder = Mads::builder_with_config(Config::empty());
    builder.root::<duplicate::app::Root>().unwrap();
    let analysis = builder.analyze();
    assert_eq!(
        analysis.diagnostics()[0].code(),
        MADS008,
        "{:?}",
        analysis.diagnostics()
    );
}
