//! Contract tests for a module-free, single-subject provider build.

use std::any::TypeId;

use furnace_rs_core::{
    FURNACE002, FURNACE003, FURNACE005, FURNACE008, Furnace, ProviderVisibility, SourceLocation,
};

mod owned {
    #[furnace_rs_core::cauldron]
    pub struct PrivateCauldron;

    impl furnace_rs_core::Cauldron for PrivateCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.provide_with::<Seed, SeedInjector>()
                .provide_with::<Repository, RepositoryInjector>()
        }
    }

    #[derive(Clone)]
    pub struct Seed(pub u32);

    fn seed() -> Seed {
        Seed(7)
    }
    #[doc = "Explicit constructor for the fixture's provider output."]
    struct SeedInjector;
    impl furnace_rs_core::Injector<Seed> for SeedInjector {
        type Dependencies = ();
        async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Seed> {
            Ok(seed())
        }
        fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
            &__FURNACE_INJECTOR_DESCRIPTOR_SEED
        }
    }
    const __FURNACE_INJECTOR_DESCRIPTOR_SEED: furnace_rs_core::ProviderDescriptor =
        furnace_rs_core::__private::InjectorMetadata::<Seed, SeedInjector>::DESCRIPTOR
            .with_authored_type_name("Seed")
            .with_namespace(module_path!())
            .with_visibility(furnace_rs_core::ProviderVisibility::Private)
            .with_location(furnace_rs_core::SourceLocation::new(
                file!(),
                line!(),
                column!(),
            ));
    furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_SEED }

    #[derive(Clone)]
    pub struct Repository(pub u32);

    fn repository(seed: Seed) -> Repository {
        Repository(seed.0)
    }
    #[doc = "Explicit constructor for the fixture's provider output."]
    struct RepositoryInjector;
    impl furnace_rs_core::Injector<Repository> for RepositoryInjector {
        type Dependencies = (Seed,);
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
            .with_visibility(furnace_rs_core::ProviderVisibility::Private)
            .with_location(furnace_rs_core::SourceLocation::new(
                file!(),
                line!(),
                column!(),
            ));
    furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_REPOSITORY }
}

mod consumer {
    use super::owned::Repository;

    #[furnace_rs_core::cauldron]
    pub struct ConsumerCauldron;

    impl furnace_rs_core::Cauldron for ConsumerCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.provide_with::<Service, ServiceInjector>()
        }
    }

    #[derive(Clone)]
    pub struct Service(pub u32);

    fn service(repository: Repository) -> Service {
        Service(repository.0 + 1)
    }
    #[doc = "Explicit constructor for the fixture's provider output."]
    struct ServiceInjector;
    impl furnace_rs_core::Injector<Service> for ServiceInjector {
        type Dependencies = (Repository,);
        async fn inject((dependency_0,): Self::Dependencies) -> furnace_rs_core::Result<Service> {
            Ok(service(dependency_0))
        }
        fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
            &__FURNACE_INJECTOR_DESCRIPTOR_SERVICE
        }
    }
    const __FURNACE_INJECTOR_DESCRIPTOR_SERVICE: furnace_rs_core::ProviderDescriptor =
        furnace_rs_core::__private::InjectorMetadata::<Service, ServiceInjector>::DESCRIPTOR
            .with_authored_type_name("Service")
            .with_namespace(module_path!())
            .with_visibility(furnace_rs_core::ProviderVisibility::Private)
            .with_location(furnace_rs_core::SourceLocation::new(
                file!(),
                line!(),
                column!(),
            ));
    furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_SERVICE }
}

struct Unrelated;

fn unrelated() -> Unrelated {
    panic!("a focused build must not construct an unrelated provider")
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct UnrelatedInjector;
impl furnace_rs_core::Injector<Unrelated> for UnrelatedInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Unrelated> {
        Ok(unrelated())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_UNRELATED
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_UNRELATED: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<Unrelated, UnrelatedInjector>::DESCRIPTOR
        .with_authored_type_name("Unrelated")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Private)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_UNRELATED }

struct Ambiguous;

fn ambiguous_first() -> Ambiguous {
    Ambiguous
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct AmbiguousFirstInjector;
impl furnace_rs_core::Injector<Ambiguous> for AmbiguousFirstInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Ambiguous> {
        Ok(ambiguous_first())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_AMBIGUOUS_FIRST
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_AMBIGUOUS_FIRST: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<Ambiguous, AmbiguousFirstInjector>::DESCRIPTOR
        .with_authored_type_name("Ambiguous")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Private)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_AMBIGUOUS_FIRST }

fn ambiguous_second() -> Ambiguous {
    Ambiguous
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct AmbiguousSecondInjector;
impl furnace_rs_core::Injector<Ambiguous> for AmbiguousSecondInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Ambiguous> {
        Ok(ambiguous_second())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_AMBIGUOUS_SECOND
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_AMBIGUOUS_SECOND: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<Ambiguous, AmbiguousSecondInjector>::DESCRIPTOR
        .with_authored_type_name("Ambiguous")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Private)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_AMBIGUOUS_SECOND }

struct Missing;

#[derive(Clone)]
struct CycleA;

#[derive(Clone)]
struct CycleB;

fn cycle_a(_dependency: CycleB) -> CycleA {
    CycleA
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct CycleAInjector;
impl furnace_rs_core::Injector<CycleA> for CycleAInjector {
    type Dependencies = (CycleB,);
    async fn inject((dependency_0,): Self::Dependencies) -> furnace_rs_core::Result<CycleA> {
        Ok(cycle_a(dependency_0))
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_CYCLE_A
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_CYCLE_A: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<CycleA, CycleAInjector>::DESCRIPTOR
        .with_authored_type_name("CycleA")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Private)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_CYCLE_A }

fn cycle_b(_dependency: CycleA) -> CycleB {
    CycleB
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct CycleBInjector;
impl furnace_rs_core::Injector<CycleB> for CycleBInjector {
    type Dependencies = (CycleA,);
    async fn inject((dependency_0,): Self::Dependencies) -> furnace_rs_core::Result<CycleB> {
        Ok(cycle_b(dependency_0))
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_CYCLE_B
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_CYCLE_B: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<CycleB, CycleBInjector>::DESCRIPTOR
        .with_authored_type_name("CycleB")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Private)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_CYCLE_B }

struct UnusedAutoConfiguration;

fn unused_auto_configuration_type_id() -> TypeId {
    TypeId::of::<UnusedAutoConfiguration>()
}

fn evaluate_unused_auto_configuration(
    _: &furnace_rs_core::__private::AutoConfigurationContext<'_>,
) -> furnace_rs_core::__private::AutoConfigurationEvaluation {
    panic!("a focused build must not evaluate an unrelated auto-configuration")
}

fn apply_unused_auto_configuration(
    _: &furnace_rs_core::__private::AutoConfigurationApplyContext<'_>,
) -> furnace_rs_core::Result<furnace_rs_core::__private::AutoConfigurationContribution> {
    panic!("a focused build must not apply an unrelated auto-configuration")
}

furnace_rs_core::__private::inventory::submit! {
    furnace_rs_core::__private::AutoConfigurationDescriptor::new(
        "focused_selection.unused",
        "focused_selection::UnusedAutoConfiguration",
        unused_auto_configuration_type_id,
        SourceLocation::new(file!(), line!(), column!()),
        evaluate_unused_auto_configuration,
        apply_unused_auto_configuration,
    )
}

#[tokio::test]
async fn focus_builds_private_transitive_chain_without_module() {
    assert_eq!(
        furnace_rs_core::Catalog::provider_for::<owned::Repository>()
            .unwrap()
            .visibility(),
        ProviderVisibility::Private,
    );

    let mut builder = Furnace::builder();
    builder.__test_focus::<consumer::Service>().unwrap();
    let application = builder.build().await.unwrap();

    assert_eq!(
        application
            .context()
            .resolve::<consumer::Service>()
            .unwrap()
            .0,
        8
    );
    assert!(
        application
            .graph()
            .provider::<consumer::Service>()
            .is_some()
    );
    assert!(
        application
            .graph()
            .provider::<owned::Repository>()
            .is_some()
    );
    assert!(application.graph().provider::<owned::Seed>().is_some());
    assert!(application.graph().provider::<Unrelated>().is_none());
    assert!(application.cauldron_graph().is_none());
}

#[tokio::test]
async fn focus_uses_supplied_value_instead_of_its_registered_constructor() {
    let mut builder = Furnace::builder();
    builder.provide(owned::Seed(40)).unwrap();
    builder.__test_focus::<consumer::Service>().unwrap();

    let application = builder.build().await.unwrap();
    assert_eq!(
        application
            .context()
            .resolve::<consumer::Service>()
            .unwrap()
            .0,
        41
    );
    assert_eq!(
        application.context().resolve::<owned::Seed>().unwrap().0,
        40
    );
}

#[tokio::test]
async fn focus_requires_an_explicit_value_even_when_a_provider_is_registered() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<consumer::Service>().unwrap();
    builder.__test_require_provided::<owned::Seed>();

    let analysis = builder.analyze();
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|d| d.code() == FURNACE003)
    );
    let Err(error) = builder.build().await else {
        panic!("a required test supply must not fall back to the registered constructor");
    };
    assert_eq!(error.code(), FURNACE003);
}

#[tokio::test]
async fn focus_accepts_a_required_value_when_it_is_supplied() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<consumer::Service>().unwrap();
    builder.__test_require_provided::<owned::Seed>();
    builder.provide(owned::Seed(9)).unwrap();

    let application = builder.build().await.unwrap();
    assert_eq!(
        application
            .context()
            .resolve::<consumer::Service>()
            .unwrap()
            .0,
        10
    );
}

#[test]
fn focus_reports_ambiguous_selected_provider() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<Ambiguous>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == FURNACE002)
    );
}

#[test]
fn focus_checks_ambiguous_target_even_when_supplied() {
    let mut builder = Furnace::builder();
    builder.provide(Ambiguous).unwrap();
    builder.__test_focus::<Ambiguous>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == FURNACE002)
    );
}

#[test]
fn focus_reports_missing_selected_provider() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<Missing>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == FURNACE003)
    );
}

#[test]
fn focus_reports_cycle_only_in_the_selected_chain() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<CycleA>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == FURNACE005)
    );
}

#[test]
fn focus_and_root_cannot_be_combined() {
    let mut rooted = Furnace::builder();
    rooted.root::<consumer::ConsumerCauldron>().unwrap();
    let Err(error) = rooted.__test_focus::<consumer::Service>() else {
        panic!("focus after a root must fail");
    };
    assert_eq!(error.code(), FURNACE008);

    let mut focused = Furnace::builder();
    focused.__test_focus::<consumer::Service>().unwrap();
    let Err(error) = focused.root::<consumer::ConsumerCauldron>() else {
        panic!("root after focus must fail");
    };
    assert_eq!(error.code(), FURNACE008);
}

#[tokio::test]
async fn focus_does_not_evaluate_unrelated_auto_configuration() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<consumer::Service>().unwrap();

    let application = builder.build().await.unwrap();
    assert_eq!(
        application
            .context()
            .resolve::<consumer::Service>()
            .unwrap()
            .0,
        8
    );
}

#[derive(Clone)]
struct FocusedDefault(u32);
#[furnace_rs_core::burner]
struct DefaultConsumer {
    value: FocusedDefault,
}
fn focused_default_id() -> TypeId {
    TypeId::of::<FocusedDefault>()
}
fn evaluate_focused_default(
    context: &furnace_rs_core::__private::AutoConfigurationContext<'_>,
) -> furnace_rs_core::__private::AutoConfigurationEvaluation {
    let requirements = context.requirements::<FocusedDefault>();
    if context.has_provider::<FocusedDefault>() {
        furnace_rs_core::__private::AutoConfigurationEvaluation::overridden(
            furnace_rs_core::AutoConfigurationReasonCode::new("supplied"),
            "already supplied",
            requirements,
            Vec::new(),
        )
    } else {
        furnace_rs_core::__private::AutoConfigurationEvaluation::active(
            furnace_rs_core::AutoConfigurationReasonCode::new("required"),
            "selected chain requires it",
            requirements,
            Vec::new(),
        )
    }
}
fn apply_focused_default(
    _: &furnace_rs_core::__private::AutoConfigurationApplyContext<'_>,
) -> furnace_rs_core::Result<furnace_rs_core::__private::AutoConfigurationContribution> {
    Ok(furnace_rs_core::__private::AutoConfigurationContribution::new(FocusedDefault(12)))
}
furnace_rs_core::__private::inventory::submit! {
    furnace_rs_core::__private::AutoConfigurationDescriptor::new(
        "focused_selection.default", "FocusedDefault", focused_default_id,
        SourceLocation::new(file!(),line!(),column!()), evaluate_focused_default, apply_focused_default)
}
#[tokio::test]
async fn focus_evaluates_only_auto_configuration_needed_by_selected_chain() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<DefaultConsumer>().unwrap();
    let app = builder.build().await.unwrap();
    assert_eq!(
        app.context().resolve::<DefaultConsumer>().unwrap().value.0,
        12
    );
    assert_eq!(app.auto_configurations().len(), 1);
}
#[tokio::test]
async fn focus_required_supply_blocks_even_a_relevant_auto_configuration() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<DefaultConsumer>().unwrap();
    builder.__test_require_provided::<FocusedDefault>();
    assert_eq!(builder.build().await.err().unwrap().code(), FURNACE003);
}
