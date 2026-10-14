//! Declarative registration preserves runtime graph selection and construction.
use furnace_rs_core::{Cauldron, CauldronRegistration, Furnace, Injector};
use std::{
    any::TypeId,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);

struct NativeInjector;
impl Injector<Arc<String>> for NativeInjector {
    type Dependencies = ();
    async fn inject((): ()) -> furnace_rs_core::Result<Arc<String>> {
        CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new("native output".into()))
    }
}

#[furnace_rs_core::burner]
struct Service {
    #[inject]
    _native: Arc<String>,
}

#[furnace_rs_core::cauldron(
    providers = [Arc<String> => NativeInjector],
    exports = [Arc<String>],
    global,
)]
struct Infrastructure;

#[furnace_rs_core::cauldron(
    imports = [Infrastructure],
    providers = [Service],
    exports = [Service],
)]
struct Declarative;

#[furnace_rs_core::cauldron]
struct Legacy;
impl Cauldron for Legacy {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(Infrastructure)
            .provide::<Service>()
            .export::<Service>()
    }
}

#[test]
fn declarative_and_legacy_roots_select_the_same_outputs_without_construction() {
    for (select, root_id) in [
        (
            select::<Declarative> as fn() -> furnace_rs_core::GraphAnalysis,
            TypeId::of::<Declarative>(),
        ),
        (select::<Legacy>, TypeId::of::<Legacy>()),
    ] {
        let analysis = select();
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
        let graph = analysis.cauldron_graph().unwrap();
        assert_eq!(
            graph.owner_of(TypeId::of::<Service>()).unwrap().type_id(),
            root_id
        );
        assert_eq!(
            graph
                .owner_of(TypeId::of::<Arc<String>>())
                .unwrap()
                .type_id(),
            TypeId::of::<Infrastructure>()
        );
        assert!(graph.exports(root_id, TypeId::of::<Service>()));
        assert!(graph.is_global(TypeId::of::<Infrastructure>()));
    }
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 0);
}

fn select<M: Cauldron>() -> furnace_rs_core::GraphAnalysis {
    let mut builder = Furnace::builder();
    builder.root::<M>().unwrap();
    builder.analyze()
}

#[furnace_rs_core::cauldron(providers = [])]
struct Empty;

#[furnace_rs_core::cauldron()]
struct ParenthesesLegacy;
impl Cauldron for ParenthesesLegacy {
    fn register(self) -> CauldronRegistration<Self> {
        CauldronRegistration::new(self)
    }
}

#[test]
fn empty_declarations_and_zero_argument_legacy_attributes_keep_their_contracts() {
    assert!(select::<Empty>().is_valid());
    assert!(select::<ParenthesesLegacy>().is_valid());
}

struct Controller;
impl Injector for Controller {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace_rs_core::Result<Self> {
        Ok(Self)
    }
}
#[furnace_rs_core::cauldron(controllers = [Controller])]
struct ControllerModule;

#[test]
fn controller_entries_record_owned_members_in_the_module_topology() {
    let graph = furnace_rs_core::__private::build_cauldron_graph::<ControllerModule>().unwrap();
    assert_eq!(
        graph
            .owner_of(TypeId::of::<Controller>())
            .unwrap()
            .type_id(),
        TypeId::of::<ControllerModule>()
    );
    assert!(!graph.exports(TypeId::of::<ControllerModule>(), TypeId::of::<Controller>()));
}
