//! Integration validators observe virtual selection and reject before construction.
use furnace_rs_core::__private::{
    AutoConfigurationApplyContext, AutoConfigurationContext, AutoConfigurationContribution,
    AutoConfigurationDescriptor, AutoConfigurationEvaluation, PreflightContext,
    PreflightDescriptor,
};
use furnace_rs_core::AutoConfigurationReasonCode;
use furnace_rs_core::{Diagnostic, FURNACE030, Furnace, SourceLocation, burner};
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
static ORDER: std::sync::Mutex<Vec<&str>> = std::sync::Mutex::new(Vec::new());
static PROVIDERS: AtomicUsize = AtomicUsize::new(0);
static DEFAULTS: AtomicUsize = AtomicUsize::new(0);
#[derive(Clone)]
struct DefaultResource;
#[burner]
struct Consumer {
    _default: DefaultResource,
}

fn tracked() -> Tracked {
    PROVIDERS.fetch_add(1, Ordering::SeqCst);
    Tracked
}
#[doc = "Explicit constructor for the fixture's provider output."]
struct TrackedInjector;
impl furnace_rs_core::Injector<Tracked> for TrackedInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Tracked> {
        Ok(tracked())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_TRACKED
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_TRACKED: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<Tracked, TrackedInjector>::DESCRIPTOR
        .with_authored_type_name("Tracked")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Private)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_TRACKED }

struct Tracked;
fn evaluate(_: &AutoConfigurationContext<'_>) -> AutoConfigurationEvaluation {
    AutoConfigurationEvaluation::active(
        AutoConfigurationReasonCode::new("test"),
        "test default",
        vec![],
        vec![],
    )
}
fn apply(
    _: &AutoConfigurationApplyContext<'_>,
) -> furnace_rs_core::Result<AutoConfigurationContribution> {
    DEFAULTS.fetch_add(1, Ordering::SeqCst);
    Ok(AutoConfigurationContribution::new(DefaultResource))
}
furnace_rs_core::__private::inventory::submit! {
    AutoConfigurationDescriptor::new("test.default", "DefaultResource", TypeId::of::<DefaultResource>, SourceLocation::new(file!(), line!(), column!()), evaluate, apply)
}
fn reject(context: &PreflightContext<'_>) -> Vec<Diagnostic> {
    ORDER.lock().unwrap().push("reject");
    assert!(context.has_output::<DefaultResource>());
    assert!(context.has_output::<Consumer>());
    assert!(context.cauldron_graph().is_none());
    assert!(context.focus_type_id().is_none());
    assert!(context.config().get("unused").is_none());
    let diagnostic = Diagnostic::new(
        FURNACE030,
        "test rejection",
        "integration preflight rejected the selected graph",
    );
    vec![diagnostic.clone(), diagnostic]
}
furnace_rs_core::__private::inventory::submit! {
    PreflightDescriptor::new("test.reject", SourceLocation::new(file!(), line!(), column!()), reject)
}
fn alpha(_: &PreflightContext<'_>) -> Vec<Diagnostic> {
    ORDER.lock().unwrap().push("alpha");
    vec![]
}
fn zeta(_: &PreflightContext<'_>) -> Vec<Diagnostic> {
    ORDER.lock().unwrap().push("zeta");
    vec![]
}
furnace_rs_core::__private::inventory::submit! { PreflightDescriptor::new("a.first", SourceLocation::new(file!(), line!(), column!()), alpha) }
furnace_rs_core::__private::inventory::submit! { PreflightDescriptor::new("z.last", SourceLocation::new(file!(), line!(), column!()), zeta) }
#[tokio::test]
async fn rejected_virtual_analysis_never_applies_defaults_or_constructs_providers() {
    let builder = Furnace::builder();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert_eq!(analysis.diagnostics().len(), 1);
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE030);
    assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
    assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    assert!(builder.build().await.is_err());
    assert_eq!(PROVIDERS.load(Ordering::SeqCst), 0);
    assert_eq!(DEFAULTS.load(Ordering::SeqCst), 0);
    assert_eq!(
        *ORDER.lock().unwrap(),
        ["alpha", "reject", "zeta", "alpha", "reject", "zeta"]
    );
}
