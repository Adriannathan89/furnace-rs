//! Dependency access follows explicit imports and exports, never Rust namespaces.
use furnace_rs_core::{Cauldron, CauldronRegistration, Furnace, GraphAnalysis};
use std::{
    any::TypeId,
    sync::atomic::{AtomicUsize, Ordering},
};
#[derive(Clone)]
/// Shared output used to exercise public Rust visibility.
pub struct Value;
#[furnace_rs_core::element]
/// Constructs the shared output.
pub fn value() -> Value {
    Value
}
#[furnace_rs_core::burner]
struct Consumer {
    _value: Value,
}
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
cauldron!(Public, |module: Public| module
    .provide::<Value>()
    .export::<Value>());
cauldron!(Private, |module: Private| module.provide::<Value>());
cauldron!(Global, |module: Global| module
    .provide::<Value>()
    .export::<Value>()
    .global());
cauldron!(PrivateGlobal, |module: PrivateGlobal| module
    .provide::<Value>()
    .global());
cauldron!(Direct, |module: Direct| module
    .provide::<Consumer>()
    .import(Public));
cauldron!(Hidden, |module: Hidden| module
    .provide::<Consumer>()
    .import(Private));
cauldron!(Middle, |module: Middle| module.import(Public));
cauldron!(Transitive, |module: Transitive| module
    .provide::<Consumer>()
    .import(Middle));
cauldron!(GlobalConsumer, |module: GlobalConsumer| module
    .provide::<Consumer>());
cauldron!(GlobalRoot, |module: GlobalRoot| module
    .import(Global)
    .import(GlobalConsumer));
cauldron!(PrivateGlobalRoot, |module: PrivateGlobalRoot| module
    .import(PrivateGlobal)
    .import(GlobalConsumer));
cauldron!(UnreachableGlobal, |module: UnreachableGlobal| module
    .provide::<Consumer>(
));
cauldron!(Repeated, |module: Repeated| module
    .import(Public)
    .import(Public));
cauldron!(CycleA, |module: CycleA| module.import(CycleB));
cauldron!(CycleB, |module: CycleB| module.import(CycleA));
static REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
#[furnace_rs_core::cauldron]
struct Shared;
impl Cauldron for Shared {
    fn register(self) -> CauldronRegistration<Self> {
        REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
        self.provide::<Value>().export::<Value>()
    }
}
cauldron!(Left, |module: Left| module.import(Shared));
cauldron!(Right, |module: Right| module.import(Shared));
cauldron!(Diamond, |module: Diamond| module.import(Left).import(Right));
fn analyze<M: Cauldron>() -> GraphAnalysis {
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
    assert_eq!(REGISTRATIONS.load(Ordering::SeqCst), 1);
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
