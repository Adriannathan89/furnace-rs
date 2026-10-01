//! Integration tests for owned framework-neutral graph inspection snapshots.

use furnace_rs_core::{
    Furnace, GraphInspectionSnapshot, ProviderOrigin, ProviderState, ProviderVisibility, cauldron,
};

mod imported {
    use furnace_rs_core::cauldron;

    #[cauldron]
    pub struct RepositoryCauldron;

    impl furnace_rs_core::Cauldron for RepositoryCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            furnace_rs_core::CauldronRegistration::new(self)
        }
    }
}

use imported::RepositoryCauldron;

#[derive(Clone)]
struct UserRepository;
struct UserService {
    _repository: UserRepository,
}

#[furnace_rs_core::element]
fn user_repository() -> UserRepository {
    UserRepository
}

#[furnace_rs_core::element]
fn user_service(repository: UserRepository) -> UserService {
    UserService {
        _repository: repository,
    }
}

#[cauldron]
struct AppCauldron;

impl furnace_rs_core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.provide::<UserRepository>()
            .provide::<UserService>()
            .import(RepositoryCauldron)
    }
}

#[test]
fn snapshot_owns_rooted_graph_metadata_after_analysis_is_dropped() {
    let snapshot = {
        let mut builder = Furnace::builder();
        builder
            .root::<AppCauldron>()
            .expect("rooted fixture metadata should be valid");
        let analysis = builder.analyze();
        let snapshot = GraphInspectionSnapshot::from_analysis(&analysis);

        drop(analysis);
        drop(builder);
        snapshot
    };

    assert_eq!(
        snapshot.root_cauldron(),
        Some("inspection_snapshot::AppCauldron")
    );
    assert_eq!(
        snapshot.cauldrons()[0].type_name(),
        "inspection_snapshot::AppCauldron"
    );
    assert_eq!(snapshot.cauldrons()[0].namespace(), "inspection_snapshot");
    assert!(
        snapshot.cauldrons()[0]
            .location()
            .file()
            .ends_with("inspection_snapshot.rs")
    );
    assert_eq!(snapshot.imports().len(), 1);
    assert_eq!(
        snapshot.imports()[0].importer(),
        "inspection_snapshot::AppCauldron"
    );
    assert_eq!(
        snapshot.imports()[0].imported(),
        "inspection_snapshot::imported::RepositoryCauldron"
    );
    assert!(
        snapshot
            .providers()
            .windows(2)
            .all(|pair| pair[0].type_name() <= pair[1].type_name())
    );
    let service = snapshot
        .providers()
        .iter()
        .find(|provider| provider.type_name() == "inspection_snapshot::UserService")
        .expect("service provider should be retained");
    assert_eq!(service.owner(), Some("inspection_snapshot::AppCauldron"));
    assert_eq!(service.origin(), ProviderOrigin::Provider);
    assert_eq!(service.visibility(), ProviderVisibility::Private);
    assert_eq!(service.state(), ProviderState::Planned);
    assert!(
        service
            .location()
            .expect("declared provider should retain its location")
            .file()
            .ends_with("inspection_snapshot.rs")
    );
    let dependencies = snapshot
        .dependencies()
        .iter()
        .map(|edge| (edge.provider(), edge.dependency()))
        .collect::<Vec<_>>();
    assert!(
        dependencies.iter().any(|(provider, dependency)| {
            *provider == "inspection_snapshot::UserService"
                && *dependency == "inspection_snapshot::UserRepository"
        }),
        "dependencies: {dependencies:?}"
    );
    assert_eq!(
        snapshot.construction_order(),
        Some(
            [
                "inspection_snapshot::UserRepository".to_owned(),
                "inspection_snapshot::UserService".to_owned(),
            ]
            .as_slice()
        )
    );
    assert!(snapshot.auto_configurations().is_empty());
    assert!(snapshot.diagnostics().is_empty());
}
