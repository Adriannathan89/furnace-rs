//! Typed injector construction, dependency metadata, and lifecycle attachment.

use furnace_rs_core::{
    Config, ConstructionContext, Diagnostic, Error, FURNACE020, InjectionDependencies, Injector,
    LifecycleResource, ProviderRegistry, Result,
};
use std::any::TypeId;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Zero;
impl Injector for Zero {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Self> {
        Ok(Self)
    }
}

struct FromConfig(usize);
impl Injector for FromConfig {
    type Dependencies = (Config,);
    async fn inject((config,): Self::Dependencies) -> Result<Self> {
        Ok(Self(config.len()))
    }
}

#[derive(Clone)]
struct Shared(Arc<AtomicUsize>);
struct Pair(Shared, String);
impl Injector for Pair {
    type Dependencies = (Shared, String);
    async fn inject((shared, text): Self::Dependencies) -> Result<Self> {
        Ok(Self(shared, text))
    }
}

struct Failing;
impl Injector for Failing {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Self> {
        tokio::task::yield_now().await;
        Err(Error::new(Diagnostic::new(
            FURNACE020,
            "failed",
            "expected constructor failure",
        )))
    }
}

struct Counted(Arc<AtomicUsize>);
impl Injector for Counted {
    type Dependencies = (Shared,);
    async fn inject((shared,): Self::Dependencies) -> Result<Self> {
        shared.0.fetch_add(1, Ordering::SeqCst);
        Ok(Self(shared.0))
    }
    fn lifecycle(value: Self) -> LifecycleResource<Self> {
        value.0.fetch_add(1, Ordering::SeqCst);
        LifecycleResource::new(value)
    }
}

#[furnace_rs_core::storage]
struct ManagedStorage {
    state: Shared,
}
#[furnace_rs_core::burner]
struct ManagedBurner {
    storage: ManagedStorage,
}
#[furnace_rs_core::burner]
struct Recursive {
    previous: Arc<Self>,
}

#[tokio::test]
async fn managed_injectors_preserve_shared_fields_and_roles() {
    let _read_recursive: fn(&Recursive) = |value| {
        let _ = &value.previous;
    };
    let state = Arc::new(AtomicUsize::new(0));
    let storage = ManagedStorage::inject((Shared(state.clone()),))
        .await
        .unwrap();
    let burner = ManagedBurner::inject((storage,)).await.unwrap();
    burner.storage.state.0.store(8, Ordering::SeqCst);
    assert_eq!(burner.clone().storage.state.0.load(Ordering::SeqCst), 8);
    assert_eq!(
        ManagedStorage::descriptor().kind(),
        furnace_rs_core::ProviderKind::Repository
    );
    assert_eq!(
        ManagedBurner::descriptor().kind(),
        furnace_rs_core::ProviderKind::Service
    );
    assert_eq!(
        Recursive::descriptor().dependencies()[0].type_id(),
        TypeId::of::<Arc<Recursive>>()
    );
}

#[tokio::test]
async fn focused_managed_construction_keeps_catalog_discovery() {
    let mut builder = furnace_rs_core::Furnace::builder();
    builder.__test_focus::<ManagedBurner>().unwrap();
    builder
        .provide(Shared(Arc::new(AtomicUsize::new(2))))
        .unwrap();
    let app = builder.build().await.unwrap();
    assert_eq!(
        app.context()
            .resolve::<ManagedBurner>()
            .unwrap()
            .storage
            .state
            .0
            .load(Ordering::SeqCst),
        2
    );
}

#[tokio::test]
async fn zero_and_single_dependencies_construct_the_native_outputs() {
    let mut registry = ProviderRegistry::new();
    registry.insert(Config::empty()).unwrap();
    let config = Config::empty();
    let context = ConstructionContext::new(&registry, &config);
    let output = (Zero::descriptor().constructor())(&context).await.unwrap();
    assert!(output.is::<Zero>());
    let descriptor = FromConfig::descriptor();
    assert_eq!(descriptor.type_id(), TypeId::of::<FromConfig>());
    assert_eq!(descriptor.type_name(), "injector::FromConfig");
    assert_eq!(
        descriptor.dependencies()[0].type_name(),
        "furnace_rs_core::config::Config"
    );
    let output = (descriptor.constructor())(&context).await.unwrap();
    assert_eq!(
        Arc::downcast::<FromConfig>(output)
            .unwrap_or_else(|_| panic!("wrong output"))
            .0,
        0
    );
}

#[tokio::test]
async fn dependency_tuple_preserves_order_and_shared_handles() {
    let counter = Arc::new(AtomicUsize::new(4));
    let mut registry = ProviderRegistry::new();
    registry.insert(Shared(counter.clone())).unwrap();
    registry.insert(String::from("second")).unwrap();
    let config = Config::empty();
    let context = ConstructionContext::new(&registry, &config);
    let output = (Pair::descriptor().constructor())(&context).await.unwrap();
    let output = Arc::downcast::<Pair>(output).unwrap_or_else(|_| panic!("wrong output"));
    assert!(Arc::ptr_eq(&output.0.0, &counter));
    assert_eq!(output.1, "second");
    assert_eq!(
        Pair::descriptor().dependencies()[1].type_id(),
        TypeId::of::<String>()
    );
}

#[test]
fn tuple_boundary_resolves_all_sixteen_dependencies() {
    type Sixteen = (
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
    );
    let mut registry = ProviderRegistry::new();
    registry.insert(7_u8).unwrap();
    let config = Config::empty();
    let context = ConstructionContext::new(&registry, &config);
    let result = Sixteen::resolve(&context).unwrap();
    assert_eq!(result.0, 7);
    assert_eq!(result.15, 7);
    assert_eq!(Sixteen::descriptors().len(), 16);
}

#[tokio::test]
async fn constructor_errors_are_propagated() {
    let registry = ProviderRegistry::new();
    let config = Config::empty();
    let context = ConstructionContext::new(&registry, &config);
    let error = (Failing::descriptor().constructor())(&context)
        .await
        .unwrap_err();
    assert_eq!(error.code(), FURNACE020);
}

#[tokio::test]
async fn repeated_metadata_access_is_static_and_does_not_construct() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut registry = ProviderRegistry::new();
    registry.insert(Shared(count.clone())).unwrap();
    let config = Config::empty();
    let context = ConstructionContext::new(&registry, &config);
    let descriptor = Counted::descriptor();
    for _ in 0..100 {
        assert!(std::ptr::eq(descriptor, Counted::descriptor()));
        assert_eq!(
            descriptor.dependencies()[0].type_id(),
            TypeId::of::<Shared>()
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let contribution = (descriptor.lifecycle_constructor().unwrap())(&context)
        .await
        .unwrap();
    assert!(contribution.into_provider().is::<Counted>());
    assert_eq!(count.load(Ordering::SeqCst), 2);
}
