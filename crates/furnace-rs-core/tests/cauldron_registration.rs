//! Explicit registration must not construct dependencies during analysis.
use furnace_rs_core::{Cauldron, CauldronRegistration, Furnace};
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

#[furnace_rs_core::burner]
struct NamedService {
    _dependency: Dependency,
}
#[derive(Clone)]
struct Dependency;
#[furnace_rs_core::element]
fn dependency() -> Dependency {
    CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
    Dependency
}
#[furnace_rs_core::cauldron]
struct Root;
impl Cauldron for Root {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<NamedService>()
            .provide::<Dependency>()
            .export::<NamedService>()
            .global()
    }
}
#[test]
fn named_field_types_register_without_construction() {
    let mut builder = Furnace::builder();
    builder.root::<Root>().unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    let graph = analysis.cauldron_graph().unwrap();
    assert_eq!(
        graph
            .owner_of(TypeId::of::<NamedService>())
            .unwrap()
            .type_id(),
        TypeId::of::<Root>()
    );
    assert!(graph.exports(TypeId::of::<Root>(), TypeId::of::<NamedService>()));
    assert!(graph.is_global(TypeId::of::<Root>()));
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 0);
}

#[furnace_rs_core::cauldron]
struct Duplicate;
impl Cauldron for Duplicate {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Dependency>().provide::<Dependency>()
    }
}
#[furnace_rs_core::cauldron]
struct BadExport;
impl Cauldron for BadExport {
    fn register(self) -> CauldronRegistration<Self> {
        self.export::<Dependency>()
    }
}
#[furnace_rs_core::cauldron]
struct Other;
impl Cauldron for Other {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Dependency>()
    }
}
#[furnace_rs_core::cauldron]
struct Conflict;
impl Cauldron for Conflict {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Dependency>().import(Other)
    }
}
#[test]
fn duplicate_members_and_nonlocal_exports_fail_before_construction() {
    for analysis in [
        analyze::<Duplicate>(),
        analyze::<BadExport>(),
        analyze::<Conflict>(),
    ] {
        assert!(!analysis.is_valid());
        assert!(
            analysis
                .diagnostics()
                .iter()
                .any(|d| d.code() == furnace_rs_core::FURNACE008)
        );
    }
}
fn analyze<M: furnace_rs_core::Cauldron>() -> furnace_rs_core::GraphAnalysis {
    let mut builder = Furnace::builder();
    builder.root::<M>().unwrap();
    builder.analyze()
}

#[furnace_rs_core::cauldron]
struct Omitted;
impl Cauldron for Omitted {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<NamedService>()
    }
}
#[test]
fn linked_factory_is_not_implicitly_registered() {
    let analysis = analyze::<Omitted>();
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|d| d.code() == furnace_rs_core::FURNACE003)
    );
}
