//! Declarative registration preserves runtime graph selection and construction.
use furnace_rs_core::{__private::ModuleContract, Furnace, Injector};
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

#[furnace_rs_core::cauldron(imports = [Infrastructure], providers = [Service], exports = [Service])]
struct SecondDeclarative;

#[test]
fn declarative_roots_select_the_same_outputs_without_construction() {
    for (select, root_id) in [
        (
            select::<Declarative> as fn() -> furnace_rs_core::GraphAnalysis,
            TypeId::of::<Declarative>(),
        ),
        (
            select::<SecondDeclarative>,
            TypeId::of::<SecondDeclarative>(),
        ),
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

fn select<M: ModuleContract>() -> furnace_rs_core::GraphAnalysis {
    let mut builder = Furnace::builder();
    builder.root::<M>().unwrap();
    builder.analyze()
}

#[furnace_rs_core::cauldron(providers = [])]
struct Empty;

#[furnace_rs_core::cauldron()]
struct ParenthesesEmpty;

#[furnace_rs_core::cauldron]
struct BareEmpty;

#[test]
fn empty_declarations_and_zero_argument_attributes_register_empty_cauldrons() {
    assert!(select::<Empty>().is_valid());
    assert!(select::<ParenthesesEmpty>().is_valid());
    let bare = select::<BareEmpty>();
    assert!(bare.is_valid());
    let graph = bare.cauldron_graph().unwrap();
    assert_eq!(graph.cauldrons().len(), 1);
    assert!(graph.owner_of(TypeId::of::<Service>()).is_none());
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
