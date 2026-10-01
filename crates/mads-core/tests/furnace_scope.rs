//! Dependency access follows explicit imports and exports, never Rust namespaces.
use mads_core::{Furnace, FurnaceRegistration, GraphAnalysis, Mads};
use std::{
    any::TypeId,
    sync::atomic::{AtomicUsize, Ordering},
};
#[derive(Clone)]
/// Shared output used to exercise public Rust visibility.
pub struct Value;
#[mads_core::element]
/// Constructs the shared output.
pub fn value() -> Value {
    Value
}
#[mads_core::burner]
struct Consumer {
    _value: Value,
}
macro_rules! furnace {
    ($name:ident, $body:expr) => {
        #[mads_core::furnace]
        struct $name;
        impl Furnace for $name {
            fn register(self) -> FurnaceRegistration<Self> {
                ($body)(self)
            }
        }
    };
}
furnace!(Public, |module: Public| module
    .provide::<Value>()
    .export::<Value>());
furnace!(Private, |module: Private| module.provide::<Value>());
furnace!(Global, |module: Global| module
    .provide::<Value>()
    .export::<Value>()
    .global());
furnace!(PrivateGlobal, |module: PrivateGlobal| module
    .provide::<Value>()
    .global());
furnace!(Direct, |module: Direct| module
    .provide::<Consumer>()
    .import(Public));
furnace!(Hidden, |module: Hidden| module
    .provide::<Consumer>()
    .import(Private));
furnace!(Middle, |module: Middle| module.import(Public));
furnace!(Transitive, |module: Transitive| module
    .provide::<Consumer>()
    .import(Middle));
furnace!(GlobalConsumer, |module: GlobalConsumer| module
    .provide::<Consumer>());
furnace!(GlobalRoot, |module: GlobalRoot| module
    .import(Global)
    .import(GlobalConsumer));
furnace!(PrivateGlobalRoot, |module: PrivateGlobalRoot| module
    .import(PrivateGlobal)
    .import(GlobalConsumer));
furnace!(UnreachableGlobal, |module: UnreachableGlobal| module
    .provide::<Consumer>(
));
furnace!(Repeated, |module: Repeated| module
    .import(Public)
    .import(Public));
furnace!(CycleA, |module: CycleA| module.import(CycleB));
furnace!(CycleB, |module: CycleB| module.import(CycleA));
static REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
#[mads_core::furnace]
struct Shared;
impl Furnace for Shared {
    fn register(self) -> FurnaceRegistration<Self> {
        REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
        self.provide::<Value>().export::<Value>()
    }
}
furnace!(Left, |module: Left| module.import(Shared));
furnace!(Right, |module: Right| module.import(Shared));
furnace!(Diamond, |module: Diamond| module.import(Left).import(Right));
fn analyze<M: Furnace>() -> GraphAnalysis {
    let mut builder = Mads::builder();
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
            .any(|d| d.code() == mads_core::MADS009)
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
    assert_eq!(diamond.module_graph().unwrap().modules().len(), 4);
    assert_eq!(REGISTRATIONS.load(Ordering::SeqCst), 1);
}
#[test]
fn overrides_keep_ownership_and_cannot_supply_unregistered_outputs() {
    let mut builder = Mads::builder();
    builder.root::<Hidden>().unwrap();
    builder.provide(Value).unwrap();
    assert!(!builder.analyze().is_valid());
    let mut builder = Mads::builder();
    builder.root::<Direct>().unwrap();
    builder.provide(Value).unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    assert_eq!(
        analysis
            .module_graph()
            .unwrap()
            .owner_of(TypeId::of::<Value>())
            .unwrap()
            .type_id(),
        TypeId::of::<Public>()
    );
    let mut builder = Mads::builder();
    builder.root::<UnreachableGlobal>().unwrap();
    builder.provide(Value).unwrap();
    assert!(!builder.analyze().is_valid());
}
#[test]
fn independent_roots_keep_separate_ownership() {
    assert_eq!(
        analyze::<Public>()
            .module_graph()
            .unwrap()
            .owner_of(TypeId::of::<Value>())
            .unwrap()
            .type_id(),
        TypeId::of::<Public>()
    );
    assert_eq!(
        analyze::<Private>()
            .module_graph()
            .unwrap()
            .owner_of(TypeId::of::<Value>())
            .unwrap()
            .type_id(),
        TypeId::of::<Private>()
    );
}
