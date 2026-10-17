//! Dependency access follows explicit imports and exports, never Rust namespaces.

#[macro_use]
#[path = "support/cauldron_metadata.rs"]
mod cauldron_metadata;
use furnace_rs_core::{__private::ModuleContract, Furnace, GraphAnalysis};
use std::any::TypeId;
#[derive(Clone)]
/// Shared output used to exercise public Rust visibility.
pub struct Value;

/// Constructs the shared output.
pub fn value() -> Value {
    Value
}
#[doc = "Explicit constructor for the fixture's provider output."]
pub struct ValueInjector;
impl furnace_rs_core::Injector<Value> for ValueInjector {
    type Dependencies = ();
    async fn inject((): Self::Dependencies) -> furnace_rs_core::Result<Value> {
        Ok(value())
    }
    fn descriptor() -> &'static furnace_rs_core::ProviderDescriptor {
        &__FURNACE_INJECTOR_DESCRIPTOR_VALUE
    }
}
const __FURNACE_INJECTOR_DESCRIPTOR_VALUE: furnace_rs_core::ProviderDescriptor =
    furnace_rs_core::__private::InjectorMetadata::<Value, ValueInjector>::DESCRIPTOR
        .with_authored_type_name("Value")
        .with_namespace(module_path!())
        .with_visibility(furnace_rs_core::ProviderVisibility::Public)
        .with_location(furnace_rs_core::SourceLocation::new(
            file!(),
            line!(),
            column!(),
        ));
furnace_rs_core::__private::inventory::submit! { __FURNACE_INJECTOR_DESCRIPTOR_VALUE }

#[furnace_rs_core::burner]
struct Consumer {
    #[inject]
    _value: Value,
}

cauldron_metadata! {
    struct Public;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide_with::<Value, ValueInjector>()
        .export::<Value>()
}
cauldron_metadata! {
    struct Private;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide_with::<Value, ValueInjector>()
}
cauldron_metadata! {
    struct Global;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide_with::<Value, ValueInjector>()
        .export::<Value>()
        .global()
}
cauldron_metadata! {
    struct PrivateGlobal;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide_with::<Value, ValueInjector>()
        .global()
}
cauldron_metadata! {
    struct Direct;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide::<Consumer>()
        .import::<Public>()
}
cauldron_metadata! {
    struct Hidden;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide::<Consumer>()
        .import::<Private>()
}
cauldron_metadata! {
    struct Middle;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<Public>()
}
cauldron_metadata! {
    struct Transitive;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide::<Consumer>()
        .import::<Middle>()
}
cauldron_metadata! {
    struct GlobalConsumer;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide::<Consumer>()
}
cauldron_metadata! {
    struct GlobalRoot;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<Global>()
        .import::<GlobalConsumer>()
}
cauldron_metadata! {
    struct PrivateGlobalRoot;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<PrivateGlobal>()
        .import::<GlobalConsumer>()
}
cauldron_metadata! {
    struct UnreachableGlobal;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide::<Consumer>()
}
cauldron_metadata! {
    struct Repeated;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<Public>()
        .import::<Public>()
}
cauldron_metadata! {
    struct CycleA;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<CycleB>()
}
cauldron_metadata! {
    struct CycleB;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<CycleA>()
}
cauldron_metadata! {
    struct Shared;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .provide_with::<Value, ValueInjector>()
        .export::<Value>()
}

cauldron_metadata! {
    struct Left;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<Shared>()
}
cauldron_metadata! {
    struct Right;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<Shared>()
}
cauldron_metadata! {
    struct Diamond;
    furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        .import::<Left>()
        .import::<Right>()
}
fn analyze<M: ModuleContract>() -> GraphAnalysis {
    let mut builder = Furnace::builder();
    builder.root::<M>().unwrap();
    builder.analyze()
}
#[test]
fn direct_import_requires_explicit_export() {
    assert!(analyze::<Direct>().is_valid());
    let hidden = analyze::<Hidden>();
    assert!(!hidden.is_valid());
    assert!(
        hidden
            .diagnostics()
            .iter()
            .any(|d| d.code() == furnace_rs_core::FURNACE009)
    );
    assert!(!analyze::<Transitive>().is_valid());
}
#[test]
fn global_exports_must_be_reachable() {
    assert!(analyze::<GlobalRoot>().is_valid());
    assert!(!analyze::<PrivateGlobalRoot>().is_valid());
    assert!(!analyze::<UnreachableGlobal>().is_valid());
}
#[test]
fn repeated_imports_and_cycles_fail_but_diamonds_register_once() {
    assert!(!analyze::<Repeated>().is_valid());
    assert!(!analyze::<CycleA>().is_valid());
    let diamond = analyze::<Diamond>();
    assert!(diamond.is_valid(), "{:?}", diamond.diagnostics());
    assert_eq!(diamond.cauldron_graph().unwrap().cauldrons().len(), 4);
    assert_eq!(
        diamond
            .cauldron_graph()
            .unwrap()
            .cauldrons()
            .iter()
            .filter(|node| node.type_id() == TypeId::of::<Shared>())
            .count(),
        1
    );
}
#[test]
fn overrides_keep_ownership_and_cannot_supply_unregistered_outputs() {
    let mut builder = Furnace::builder();
    builder.root::<Hidden>().unwrap();
    builder.provide(Value).unwrap();
    assert!(!builder.analyze().is_valid());
    let mut builder = Furnace::builder();
    builder.root::<Direct>().unwrap();
    builder.provide(Value).unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    assert_eq!(
        analysis
            .cauldron_graph()
            .unwrap()
            .owner_of(TypeId::of::<Value>())
            .unwrap()
            .type_id(),
        TypeId::of::<Public>()
    );
    let mut builder = Furnace::builder();
    builder.root::<UnreachableGlobal>().unwrap();
    builder.provide(Value).unwrap();
    assert!(!builder.analyze().is_valid());
}
#[test]
fn independent_roots_keep_separate_ownership() {
    assert_eq!(
        analyze::<Public>()
            .cauldron_graph()
            .unwrap()
            .owner_of(TypeId::of::<Value>())
            .unwrap()
            .type_id(),
        TypeId::of::<Public>()
    );
    assert_eq!(
        analyze::<Private>()
            .cauldron_graph()
            .unwrap()
            .owner_of(TypeId::of::<Value>())
            .unwrap()
            .type_id(),
        TypeId::of::<Private>()
    );
}
