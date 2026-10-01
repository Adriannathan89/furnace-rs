//! Explicit registration must not construct dependencies during analysis.
use mads_core::{Furnace, FurnaceRegistration, Mads};
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

#[mads_core::burner]
struct NamedService {
    _dependency: Dependency,
}
#[derive(Clone)]
struct Dependency;
#[mads_core::element]
fn dependency() -> Dependency {
    CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
    Dependency
}
#[mads_core::furnace]
struct Root;
impl Furnace for Root {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<NamedService>()
            .provide::<Dependency>()
            .export::<NamedService>()
            .global()
    }
}
#[test]
fn named_field_types_register_without_construction() {
    let mut builder = Mads::builder();
    builder.root::<Root>().unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    let graph = analysis.module_graph().unwrap();
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

#[mads_core::furnace]
struct Duplicate;
impl Furnace for Duplicate {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<Dependency>().provide::<Dependency>()
    }
}
#[mads_core::furnace]
struct BadExport;
impl Furnace for BadExport {
    fn register(self) -> FurnaceRegistration<Self> {
        self.export::<Dependency>()
    }
}
#[mads_core::furnace]
struct Other;
impl Furnace for Other {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<Dependency>()
    }
}
#[mads_core::furnace]
struct Conflict;
impl Furnace for Conflict {
    fn register(self) -> FurnaceRegistration<Self> {
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
                .any(|d| d.code() == mads_core::MADS008)
        );
    }
}
fn analyze<M: mads_core::Furnace>() -> mads_core::GraphAnalysis {
    let mut builder = Mads::builder();
    builder.root::<M>().unwrap();
    builder.analyze()
}

#[mads_core::furnace]
struct Omitted;
impl Furnace for Omitted {
    fn register(self) -> FurnaceRegistration<Self> {
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
            .any(|d| d.code() == mads_core::MADS003)
    );
}
