//! Integration tests for explicit application construction.

use std::any::TypeId;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

use furnace_rs_core::{
    ApplicationContext, Catalog, ConstructionContext, ConstructionStep, DependencyDescriptor,
    ErasedProvider, FURNACE003, FURNACE008, Furnace, LifecycleFuture, LifecycleHook,
    LifecycleState, ProviderDescriptor, ProviderFuture, ProviderKind, ProviderOrigin,
    ProviderState, ProviderVisibility, SourceLocation,
};

mod rooted {
    pub mod app {
        #[furnace_rs_core::cauldron]
        pub struct AppCauldron;

        impl furnace_rs_core::Cauldron for AppCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.provide::<ReachableService>()
                    .export::<ReachableService>()
            }
        }

        #[derive(Clone)]
        pub struct ReachableService;

        #[furnace_rs_core::element]
        pub fn reachable_service() -> ReachableService {
            ReachableService
        }
    }

    pub mod unreachable {
        #[furnace_rs_core::cauldron]
        pub struct UnreachableCauldron;

        impl furnace_rs_core::Cauldron for UnreachableCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.provide::<UnreachableService>()
                    .export::<UnreachableService>()
            }
        }

        #[derive(Clone)]
        pub struct UnreachableService;

        #[furnace_rs_core::element]
        pub fn unreachable_service() -> UnreachableService {
            UnreachableService
        }
    }
}

struct Database;

impl Database {
    fn new() -> Self {
        Self
    }
}

struct Repository {
    database: Arc<Database>,
}

fn database_type_id() -> TypeId {
    TypeId::of::<Database>()
}

fn repository_type_id() -> TypeId {
    TypeId::of::<Repository>()
}

thread_local! {
    static DATABASE_CONSTRUCTIONS: Cell<usize> = const { Cell::new(0) };
}

fn reset_database_constructions() {
    DATABASE_CONSTRUCTIONS.set(0);
}

fn database_constructions() -> usize {
    DATABASE_CONSTRUCTIONS.get()
}

fn database_constructor<'a>(_: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
    Box::pin(async {
        DATABASE_CONSTRUCTIONS.set(DATABASE_CONSTRUCTIONS.get() + 1);
        Ok(Arc::new(Database::new()) as ErasedProvider)
    })
}

fn repository_constructor<'a>(context: &'a ConstructionContext<'a>) -> ProviderFuture<'a> {
    Box::pin(async move {
        let database = context.resolve::<Database>()?;
        Ok(Arc::new(Repository { database }) as ErasedProvider)
    })
}

static REPOSITORY_DEPENDENCIES: [DependencyDescriptor; 1] = [DependencyDescriptor::new(
    "builder::Database",
    database_type_id,
)];

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Provider,
        "builder::Database",
        database_type_id,
        &[],
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        database_constructor,
    )
}

inventory::submit! {
    ProviderDescriptor::new(
        ProviderKind::Repository,
        "builder::Repository",
        repository_type_id,
        &REPOSITORY_DEPENDENCIES,
        ProviderVisibility::Private,
        SourceLocation::new(file!(), line!(), column!()),
        repository_constructor,
    )
}

#[tokio::test]
async fn explicitly_constructs_a_provider_after_its_dependency_is_provided() {
    let mut builder = Furnace::builder();
    builder
        .provide(Database::new())
        .expect("database insertion should work");
    builder
        .construct::<Repository>()
        .await
        .expect("explicit construction should work");
    let app = builder
        .build()
        .await
        .expect("the application graph should build");

    let repository = app
        .context()
        .resolve::<Repository>()
        .expect("repository should be available after explicit construction");
    let database = app
        .context()
        .resolve::<Database>()
        .expect("provided database should remain available");
    assert!(Arc::ptr_eq(&repository.database, &database));
}

#[tokio::test]
async fn construct_does_not_recursively_create_missing_dependencies() {
    let mut builder = Furnace::builder();

    let Err(error) = builder.construct::<Repository>().await else {
        panic!("repository construction should require an explicit database");
    };

    assert_eq!(error.code(), FURNACE003);
    assert!(Catalog::provider_for::<Database>().is_ok());
}

#[test]
fn builder_analysis_is_repeatable_and_side_effect_free() {
    reset_database_constructions();
    let builder = Furnace::builder();

    let first = builder.analyze();
    let second = builder.analyze();

    assert!(first.is_valid());
    assert!(second.is_valid());
    assert!(first.cauldron_graph().is_none());
    assert!(second.cauldron_graph().is_none());
    assert_eq!(database_constructions(), 0);
    assert_eq!(
        first
            .construction_plan()
            .unwrap()
            .steps()
            .iter()
            .map(ConstructionStep::type_name)
            .collect::<Vec<_>>(),
        second
            .construction_plan()
            .unwrap()
            .steps()
            .iter()
            .map(ConstructionStep::type_name)
            .collect::<Vec<_>>(),
    );
}

#[tokio::test]
async fn rooted_builder_constructs_only_the_selected_application() {
    use rooted::{
        app::{AppCauldron, ReachableService},
        unreachable::UnreachableService,
    };

    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>().unwrap();

    let analysis = builder.analyze();
    assert_eq!(
        analysis.cauldron_graph().unwrap().root().type_name(),
        std::any::type_name::<AppCauldron>()
    );
    assert!(analysis.graph().provider::<ReachableService>().is_some());
    assert!(analysis.graph().provider::<UnreachableService>().is_none());

    let application = builder.build().await.unwrap();
    assert_eq!(
        application.cauldron_graph().unwrap().root().type_name(),
        std::any::type_name::<AppCauldron>()
    );
    assert!(application.context().resolve::<ReachableService>().is_ok());
    assert!(
        application
            .context()
            .resolve::<UnreachableService>()
            .is_err()
    );
}

#[test]
fn root_can_be_selected_only_once() {
    use rooted::app::AppCauldron;

    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>().unwrap();
    let error = match builder.root::<AppCauldron>() {
        Ok(_) => panic!("a second root selection must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE008);
}

#[tokio::test]
async fn explicit_and_preconstructed_values_have_distinct_states() {
    let mut builder = Furnace::builder();
    builder.provide(Database::new()).unwrap();
    builder.construct::<Repository>().await.unwrap();

    let analysis = builder.analyze();
    assert_eq!(
        analysis.graph().provider::<Database>().unwrap().state(),
        ProviderState::Provided
    );
    assert_eq!(
        analysis.graph().provider::<Repository>().unwrap().state(),
        ProviderState::Preconstructed
    );
    assert_eq!(
        analysis
            .construction_plan()
            .unwrap()
            .steps()
            .iter()
            .map(ConstructionStep::type_name)
            .collect::<Vec<_>>(),
        ["ReachableService", "UnreachableService"]
    );
}

#[tokio::test]
async fn build_constructs_the_complete_graph_in_dependency_order() {
    reset_database_constructions();
    let application = Furnace::builder().build().await.unwrap();

    assert_eq!(database_constructions(), 1);
    assert!(application.context().resolve::<Database>().is_ok());
    assert!(application.context().resolve::<Repository>().is_ok());
    assert_eq!(
        application
            .construction_plan()
            .steps()
            .iter()
            .map(ConstructionStep::type_name)
            .collect::<Vec<_>>(),
        [
            "ReachableService",
            "UnreachableService",
            "builder::Database",
            "builder::Repository",
        ],
    );
}

#[tokio::test]
async fn supplied_values_suppress_matching_constructors() {
    reset_database_constructions();
    let mut builder = Furnace::builder();
    builder.provide(Database::new()).unwrap();

    let application = builder.build().await.unwrap();

    assert_eq!(database_constructions(), 0);
    assert_eq!(
        application.graph().provider::<Database>().unwrap().origin(),
        ProviderOrigin::Provided
    );
    assert!(application.context().resolve::<Repository>().is_ok());
}

struct ApplicationHook(Arc<Mutex<Vec<&'static str>>>);

impl LifecycleHook for ApplicationHook {
    fn name(&self) -> &str {
        "application"
    }

    fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .expect("event lock should not be poisoned")
                .push("start");
            Ok(())
        })
    }

    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .expect("event lock should not be poisoned")
                .push("stop");
            Ok(())
        })
    }
}

struct NamedHook {
    name: &'static str,
    events: Arc<Mutex<Vec<String>>>,
}

impl NamedHook {
    fn new(name: &'static str, events: Arc<Mutex<Vec<String>>>) -> Self {
        Self { name, events }
    }
}

impl LifecycleHook for NamedHook {
    fn name(&self) -> &str {
        self.name
    }

    fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.events
                .lock()
                .expect("event lock should not be poisoned")
                .push(format!("start:{}", self.name));
            Ok(())
        })
    }

    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            self.events
                .lock()
                .expect("event lock should not be poisoned")
                .push(format!("stop:{}", self.name));
            Ok(())
        })
    }
}

#[tokio::test]
async fn built_application_owns_and_runs_registered_lifecycle_hooks() {
    reset_database_constructions();
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut builder = Furnace::builder();
    builder.lifecycle_hook(ApplicationHook(Arc::clone(&events)));
    let mut application = builder
        .build()
        .await
        .expect("the application graph should build");

    assert_eq!(database_constructions(), 1);
    assert!(
        events
            .lock()
            .expect("event lock should not be poisoned")
            .is_empty()
    );
    assert_eq!(application.state(), LifecycleState::Created);
    application.start().await.expect("application should start");
    assert_eq!(application.state(), LifecycleState::Running);
    application
        .shutdown()
        .await
        .expect("application should stop");

    assert_eq!(application.state(), LifecycleState::Stopped);
    assert_eq!(
        *events.lock().expect("event lock should not be poisoned"),
        ["start", "stop"]
    );
}

#[tokio::test]
async fn builder_groups_infrastructure_before_application_regardless_of_call_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut builder = Furnace::builder();
    builder.lifecycle_hook(NamedHook::new("application", Arc::clone(&events)));
    builder.__infrastructure_lifecycle_hook(
        "furnace_rs.test.infrastructure",
        NamedHook::new("infrastructure", Arc::clone(&events)),
    );
    let mut application = builder.build().await.unwrap();

    application.start().await.unwrap();
    application.shutdown().await.unwrap();

    assert_eq!(
        *events.lock().unwrap(),
        [
            "start:infrastructure",
            "start:application",
            "stop:application",
            "stop:infrastructure",
        ],
    );
}
