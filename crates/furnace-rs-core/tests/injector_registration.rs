//! Explicit injector choices retain cauldron ownership and pre-construction validation.
use furnace_rs_core::{
    Cauldron, CauldronRegistration, FURNACE003, FURNACE005, FURNACE008, FURNACE009, Furnace,
    GraphAnalysis, Injector, LifecycleResource, Result,
};
use std::{
    any::TypeId,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone)]
struct Dependency(Arc<AtomicUsize>);
impl Injector for Dependency {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Self> {
        Ok(Self(Arc::new(AtomicUsize::new(0))))
    }
}
trait Service: Send + Sync {
    fn value(&self) -> usize;
}
struct Implementation(Dependency);
impl Service for Implementation {
    fn value(&self) -> usize {
        self.0.0.load(Ordering::SeqCst)
    }
}
impl Injector<Arc<dyn Service>> for Implementation {
    type Dependencies = (Dependency,);
    async fn inject((dependency,): Self::Dependencies) -> Result<Arc<dyn Service>> {
        dependency.0.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(Self(dependency)))
    }
    fn lifecycle(value: Arc<dyn Service>) -> LifecycleResource<Arc<dyn Service>> {
        assert_eq!(value.value(), 1);
        LifecycleResource::new(value)
    }
}
struct Alternative;
impl Service for Alternative {
    fn value(&self) -> usize {
        99
    }
}
impl Injector<Arc<dyn Service>> for Alternative {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Arc<dyn Service>> {
        Ok(Arc::new(Self))
    }
}

fn competing_linked_factory() -> Arc<dyn Service> {
    Arc::new(Alternative)
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct CompetingLinkedFactoryInjector;
impl furnace_rs_core::Injector<Arc<dyn Service>> for CompetingLinkedFactoryInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Arc<dyn Service>> {
        Ok(competing_linked_factory())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_COMPETING_LINKED_FACTORY
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_COMPETING_LINKED_FACTORY : furnace_rs_core :: ProviderDescriptor = furnace_rs_core :: __private :: InjectorMetadata :: < Arc < dyn Service > , CompetingLinkedFactoryInjector > :: DESCRIPTOR . with_authored_type_name (stringify ! (Arc < dyn Service >)) . with_namespace (module_path ! ()) . with_visibility (furnace_rs_core :: ProviderVisibility :: Private) . with_location (furnace_rs_core :: SourceLocation :: new (file ! () , line ! () , column ! ())) ;
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_COMPETING_LINKED_FACTORY }

macro_rules! cauldron {
    ($name:ident, $body:expr) => {
        #[furnace_rs_core::cauldron]
        struct $name;
        impl Cauldron for $name {
            fn register(self) -> CauldronRegistration<Self> {
                ($body)(self)
            }
        }
    };
}
cauldron!(Local, |m: Local| m
    .provide_with::<Dependency, Dependency>()
    .provide_with::<Arc<dyn Service>, Implementation>());
cauldron!(PublicDependency, |m: PublicDependency| m
    .provide_with::<Dependency, Dependency>()
    .export::<Dependency>());
cauldron!(PrivateDependency, |m: PrivateDependency| m
    .provide_with::<Dependency, Dependency>());
cauldron!(Imported, |m: Imported| m
    .provide_with::<Arc<dyn Service>, Implementation>()
    .import(PublicDependency));
cauldron!(Hidden, |m: Hidden| m
    .provide_with::<Arc<dyn Service>, Implementation>()
    .import(PrivateDependency));
cauldron!(Missing, |m: Missing| m
    .provide_with::<Arc<dyn Service>, Implementation>());
cauldron!(Duplicate, |m: Duplicate| m
    .provide_with::<Arc<dyn Service>, Implementation>()
    .provide_with::<Arc<dyn Service>, Alternative>());
cauldron!(OtherOwner, |m: OtherOwner| m
    .provide_with::<Arc<dyn Service>, Alternative>());
cauldron!(ConflictingOwners, |m: ConflictingOwners| m
    .provide_with::<Arc<dyn Service>, Implementation>()
    .import(OtherOwner));

static CYCLE_CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
#[derive(Clone)]
struct CycleA;
#[derive(Clone)]
struct CycleB;
impl Injector for CycleA {
    type Dependencies = (CycleB,);
    async fn inject(_: Self::Dependencies) -> Result<Self> {
        CYCLE_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Self)
    }
}
impl Injector for CycleB {
    type Dependencies = (CycleA,);
    async fn inject(_: Self::Dependencies) -> Result<Self> {
        CYCLE_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Self)
    }
}
cauldron!(Cyclic, |m: Cyclic| m
    .provide_with::<CycleA, CycleA>()
    .provide_with::<CycleB, CycleB>());

fn analyze<M: Cauldron>() -> GraphAnalysis {
    let mut builder = Furnace::builder();
    builder.root::<M>().unwrap();
    builder.analyze()
}

#[tokio::test]
async fn explicit_binding_wins_over_linked_factory_and_does_not_register_implementer() {
    let mut builder = Furnace::builder();
    builder.root::<Local>().unwrap();
    for _ in 0..10 {
        let analysis = builder.analyze();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
        assert!(
            analysis
                .cauldron_graph()
                .unwrap()
                .owner_of(TypeId::of::<Implementation>())
                .is_none()
        );
    }
    let app = builder.build().await.unwrap();
    assert_eq!(
        app.context().resolve::<Arc<dyn Service>>().unwrap().value(),
        1
    );
    assert_eq!(
        app.context()
            .resolve::<Dependency>()
            .unwrap()
            .0
            .load(Ordering::SeqCst),
        1
    );
    assert!(app.context().resolve::<Implementation>().is_err());
}

#[tokio::test]
async fn direct_import_and_export_make_constructor_dependencies_accessible() {
    let mut builder = Furnace::builder();
    builder.root::<Imported>().unwrap();
    let app = builder.build().await.unwrap();
    assert_eq!(
        app.context().resolve::<Arc<dyn Service>>().unwrap().value(),
        1
    );
    assert!(
        analyze::<Hidden>()
            .diagnostics()
            .iter()
            .any(|d| d.code() == FURNACE009)
    );
}

#[tokio::test]
async fn invalid_graphs_fail_before_any_construction() {
    assert!(
        analyze::<Missing>()
            .diagnostics()
            .iter()
            .any(|d| d.code() == FURNACE003)
    );
    let mut builder = Furnace::builder();
    builder.root::<Missing>().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    builder.provide(Dependency(counter.clone())).unwrap();
    assert!(builder.build().await.is_err()); // unregistered overrides cannot hide a missing dependency
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    let mut builder = Furnace::builder();
    builder.root::<Cyclic>().unwrap();
    let analysis = builder.analyze();
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|d| d.code() == FURNACE005)
    );
    assert!(builder.build().await.is_err());
    assert_eq!(CYCLE_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn duplicate_bindings_and_owners_fail_and_unrelated_broken_roots_are_isolated() {
    for analysis in [analyze::<Duplicate>(), analyze::<ConflictingOwners>()] {
        assert!(
            analysis
                .diagnostics()
                .iter()
                .any(|d| d.code() == FURNACE008)
        );
    }
    assert!(analyze::<Local>().is_valid()); // Missing/Cyclic/Hidden remain linked but unreachable
}

#[test]
fn manual_injector_diagnostics_point_to_the_authored_registration() {
    for (analysis, code) in [
        (analyze::<Missing>(), FURNACE003),
        (analyze::<Hidden>(), FURNACE009),
        (analyze::<Cyclic>(), FURNACE005),
    ] {
        let diagnostic = analysis
            .diagnostics()
            .iter()
            .find(|d| d.code() == code)
            .unwrap();
        assert!(
            diagnostic
                .location()
                .unwrap()
                .file
                .ends_with("injector_registration.rs"),
            "unexpected location: {:?}",
            diagnostic.location()
        );
    }
}

#[tokio::test]
async fn supplied_output_skips_injection_and_lifecycle_attachment() {
    let mut builder = Furnace::builder();
    builder.root::<Local>().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    builder.provide(Dependency(counter.clone())).unwrap();
    builder
        .provide(Arc::new(Alternative) as Arc<dyn Service>)
        .unwrap();
    let app = builder.build().await.unwrap();
    assert_eq!(
        app.context().resolve::<Arc<dyn Service>>().unwrap().value(),
        99
    );
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn explicit_construction_uses_the_rooted_binding() {
    let mut builder = Furnace::builder();
    builder.root::<Local>().unwrap();
    builder
        .provide(Dependency(Arc::new(AtomicUsize::new(0))))
        .unwrap();
    builder.construct::<Arc<dyn Service>>().await.unwrap();
    let app = builder.build().await.unwrap();
    assert_eq!(
        app.context().resolve::<Arc<dyn Service>>().unwrap().value(),
        1
    );
}
