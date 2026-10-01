//! Contract tests for a module-free, single-subject provider build.

use std::any::TypeId;

use mads_core::{MADS002, MADS003, MADS005, MADS008, Mads, ProviderVisibility, SourceLocation};

mod owned {
    #[mads_core::module]
    pub struct PrivateModule;

    #[derive(Clone)]
    pub struct Seed(pub u32);

    #[mads_core::provider]
    fn seed() -> Seed {
        Seed(7)
    }

    #[derive(Clone)]
    pub struct Repository(pub u32);

    #[mads_core::provider]
    fn repository(seed: Seed) -> Repository {
        Repository(seed.0)
    }
}

mod consumer {
    use super::owned::Repository;

    #[mads_core::module]
    pub struct ConsumerModule;

    #[derive(Clone)]
    pub struct Service(pub u32);

    #[mads_core::provider]
    fn service(repository: Repository) -> Service {
        Service(repository.0 + 1)
    }
}

struct Unrelated;

#[mads_core::provider]
fn unrelated() -> Unrelated {
    panic!("a focused build must not construct an unrelated provider")
}

struct Ambiguous;

#[mads_core::provider]
fn ambiguous_first() -> Ambiguous {
    Ambiguous
}

#[mads_core::provider]
fn ambiguous_second() -> Ambiguous {
    Ambiguous
}

struct Missing;

#[derive(Clone)]
struct CycleA;

#[derive(Clone)]
struct CycleB;

#[mads_core::provider]
fn cycle_a(_dependency: CycleB) -> CycleA {
    CycleA
}

#[mads_core::provider]
fn cycle_b(_dependency: CycleA) -> CycleB {
    CycleB
}

struct UnusedAutoConfiguration;

fn unused_auto_configuration_type_id() -> TypeId {
    TypeId::of::<UnusedAutoConfiguration>()
}

fn evaluate_unused_auto_configuration(
    _: &mads_core::__private::AutoConfigurationContext<'_>,
) -> mads_core::__private::AutoConfigurationEvaluation {
    panic!("a focused build must not evaluate an unrelated auto-configuration")
}

fn apply_unused_auto_configuration(
    _: &mads_core::__private::AutoConfigurationApplyContext<'_>,
) -> mads_core::Result<mads_core::__private::AutoConfigurationContribution> {
    panic!("a focused build must not apply an unrelated auto-configuration")
}

mads_core::__private::inventory::submit! {
    mads_core::__private::AutoConfigurationDescriptor::new(
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
        mads_core::Catalog::provider_for::<owned::Repository>()
            .unwrap()
            .visibility(),
        ProviderVisibility::Private,
    );

    let mut builder = Mads::builder();
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
    assert!(application.module_graph().is_none());
}

#[tokio::test]
async fn focus_uses_supplied_value_instead_of_its_registered_constructor() {
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
    builder.__test_focus::<consumer::Service>().unwrap();
    builder.__test_require_provided::<owned::Seed>();

    let analysis = builder.analyze();
    assert!(analysis.diagnostics().iter().any(|d| d.code() == MADS003));
    let Err(error) = builder.build().await else {
        panic!("a required test supply must not fall back to the registered constructor");
    };
    assert_eq!(error.code(), MADS003);
}

#[tokio::test]
async fn focus_accepts_a_required_value_when_it_is_supplied() {
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
    builder.__test_focus::<Ambiguous>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MADS002)
    );
}

#[test]
fn focus_checks_ambiguous_target_even_when_supplied() {
    let mut builder = Mads::builder();
    builder.provide(Ambiguous).unwrap();
    builder.__test_focus::<Ambiguous>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MADS002)
    );
}

#[test]
fn focus_reports_missing_selected_provider() {
    let mut builder = Mads::builder();
    builder.__test_focus::<Missing>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MADS003)
    );
}

#[test]
fn focus_reports_cycle_only_in_the_selected_chain() {
    let mut builder = Mads::builder();
    builder.__test_focus::<CycleA>().unwrap();

    assert!(
        builder
            .analyze()
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MADS005)
    );
}

#[test]
fn focus_and_root_cannot_be_combined() {
    let mut rooted = Mads::builder();
    rooted.root::<consumer::ConsumerModule>().unwrap();
    let Err(error) = rooted.__test_focus::<consumer::Service>() else {
        panic!("focus after a root must fail");
    };
    assert_eq!(error.code(), MADS008);

    let mut focused = Mads::builder();
    focused.__test_focus::<consumer::Service>().unwrap();
    let Err(error) = focused.root::<consumer::ConsumerModule>() else {
        panic!("root after focus must fail");
    };
    assert_eq!(error.code(), MADS008);
}

#[tokio::test]
async fn focus_does_not_evaluate_unrelated_auto_configuration() {
    let mut builder = Mads::builder();
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
#[mads_core::service]
struct DefaultConsumer {
    value: FocusedDefault,
}
fn focused_default_id() -> TypeId {
    TypeId::of::<FocusedDefault>()
}
fn evaluate_focused_default(
    context: &mads_core::__private::AutoConfigurationContext<'_>,
) -> mads_core::__private::AutoConfigurationEvaluation {
    let requirements = context.requirements::<FocusedDefault>();
    if context.has_provider::<FocusedDefault>() {
        mads_core::__private::AutoConfigurationEvaluation::overridden(
            mads_core::AutoConfigurationReasonCode::new("supplied"),
            "already supplied",
            requirements,
            Vec::new(),
        )
    } else {
        mads_core::__private::AutoConfigurationEvaluation::active(
            mads_core::AutoConfigurationReasonCode::new("required"),
            "selected chain requires it",
            requirements,
            Vec::new(),
        )
    }
}
fn apply_focused_default(
    _: &mads_core::__private::AutoConfigurationApplyContext<'_>,
) -> mads_core::Result<mads_core::__private::AutoConfigurationContribution> {
    Ok(mads_core::__private::AutoConfigurationContribution::new(
        FocusedDefault(12),
    ))
}
mads_core::__private::inventory::submit! {
    mads_core::__private::AutoConfigurationDescriptor::new(
        "focused_selection.default", "FocusedDefault", focused_default_id,
        SourceLocation::new(file!(),line!(),column!()), evaluate_focused_default, apply_focused_default)
}
#[tokio::test]
async fn focus_evaluates_only_auto_configuration_needed_by_selected_chain() {
    let mut builder = Mads::builder();
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
    let mut builder = Mads::builder();
    builder.__test_focus::<DefaultConsumer>().unwrap();
    builder.__test_require_provided::<FocusedDefault>();
    assert_eq!(builder.build().await.err().unwrap().code(), MADS003);
}
