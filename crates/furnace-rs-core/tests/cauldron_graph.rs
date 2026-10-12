//! Integration tests for deterministic rooted module graph construction.

use furnace_rs_core::{CauldronNode, FURNACE008, cauldron};

struct UndeclaredRoot;

impl furnace_rs_core::Cauldron for UndeclaredRoot {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        furnace_rs_core::CauldronRegistration::new(self)
    }
}

mod diamond {
    use super::cauldron;

    pub mod shared {
        use super::cauldron;

        #[cauldron]
        pub struct SharedCauldron;

        impl furnace_rs_core::Cauldron for SharedCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                furnace_rs_core::CauldronRegistration::new(self)
            }
        }
    }

    pub mod left {
        use super::{cauldron, shared::SharedCauldron};

        #[cauldron]
        pub struct LeftCauldron;

        impl furnace_rs_core::Cauldron for LeftCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(SharedCauldron)
            }
        }
    }

    pub mod right {
        use super::{cauldron, shared::SharedCauldron};

        #[cauldron]
        pub struct RightCauldron;

        impl furnace_rs_core::Cauldron for RightCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(SharedCauldron)
            }
        }
    }

    pub mod root {
        use super::{cauldron, left::LeftCauldron, right::RightCauldron};

        #[cauldron]
        pub struct DiamondRoot;

        impl furnace_rs_core::Cauldron for DiamondRoot {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(LeftCauldron).import(RightCauldron)
            }
        }
    }
}

mod duplicate_import {
    use super::cauldron;

    pub mod leaf {
        use super::cauldron;

        #[cauldron]
        pub struct LeafCauldron;

        impl furnace_rs_core::Cauldron for LeafCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                furnace_rs_core::CauldronRegistration::new(self)
            }
        }
    }

    pub mod root {
        use super::{cauldron, leaf::LeafCauldron};

        #[cauldron]
        pub struct DuplicateImportRoot;

        impl furnace_rs_core::Cauldron for DuplicateImportRoot {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(LeafCauldron).import(LeafCauldron)
            }
        }
    }
}

mod self_import {
    use super::cauldron;

    #[cauldron]
    pub struct SelfImportCauldron;

    impl furnace_rs_core::Cauldron for SelfImportCauldron {
        fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
            self.import(SelfImportCauldron)
        }
    }
}

mod cycle {
    use super::cauldron;

    pub mod first {
        use super::{cauldron, second::SecondCycleCauldron};

        #[cauldron]
        pub struct FirstCycleCauldron;

        impl furnace_rs_core::Cauldron for FirstCycleCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(SecondCycleCauldron)
            }
        }
    }

    pub mod second {
        use super::{cauldron, first::FirstCycleCauldron};

        #[cauldron]
        pub struct SecondCycleCauldron;

        impl furnace_rs_core::Cauldron for SecondCycleCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(FirstCycleCauldron)
            }
        }
    }
}

mod namespace_collision {
    use super::cauldron;

    pub mod shared_namespace {
        use super::cauldron;

        #[cauldron]
        pub struct FirstCauldron;

        impl furnace_rs_core::Cauldron for FirstCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                furnace_rs_core::CauldronRegistration::new(self)
            }
        }

        #[cauldron]
        pub struct SecondCauldron;

        impl furnace_rs_core::Cauldron for SecondCauldron {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                furnace_rs_core::CauldronRegistration::new(self)
            }
        }
    }

    pub mod root {
        use super::{cauldron, shared_namespace::FirstCauldron};

        #[cauldron]
        pub struct CollisionRoot;

        impl furnace_rs_core::Cauldron for CollisionRoot {
            fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
                self.import(FirstCauldron)
            }
        }
    }
}

#[test]
fn diamond_graph_is_deterministic_and_deduplicated() {
    use diamond::{
        left::LeftCauldron, right::RightCauldron, root::DiamondRoot, shared::SharedCauldron,
    };

    let graph = furnace_rs_core::__private::build_cauldron_graph::<DiamondRoot>()
        .expect("diamond graph should be valid");
    assert_eq!(
        graph
            .cauldrons()
            .iter()
            .map(CauldronNode::type_name)
            .collect::<Vec<_>>(),
        [
            std::any::type_name::<DiamondRoot>(),
            std::any::type_name::<LeftCauldron>(),
            std::any::type_name::<SharedCauldron>(),
            std::any::type_name::<RightCauldron>(),
        ],
    );
    assert_eq!(
        graph.root().type_name(),
        std::any::type_name::<DiamondRoot>()
    );
    assert_eq!(graph.imports().len(), 4);
    assert_eq!(
        graph.imports()[0].importer(&graph).type_name(),
        std::any::type_name::<DiamondRoot>()
    );
    assert_eq!(
        graph.imports()[0].imported(&graph).type_name(),
        std::any::type_name::<LeftCauldron>()
    );
}

#[test]
fn missing_root_metadata_uses_a_stable_subject() {
    let error = match furnace_rs_core::__private::build_cauldron_graph::<UndeclaredRoot>() {
        Ok(_) => panic!("undeclared root must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE008);
    assert!(error.to_string().contains("subject: requested module"));
}

#[test]
fn duplicate_direct_import_reports_stable_subject_and_location() {
    use duplicate_import::root::DuplicateImportRoot;

    let error = match furnace_rs_core::__private::build_cauldron_graph::<DuplicateImportRoot>() {
        Ok(_) => panic!("duplicate direct imports must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE008);
    let rendered = error.to_string();
    assert!(rendered.contains(std::any::type_name::<DuplicateImportRoot>()));
    assert!(rendered.contains("cauldron_graph.rs"));
}

#[test]
fn self_import_reports_the_cycle_and_source_location() {
    use self_import::SelfImportCauldron;

    let error = match furnace_rs_core::__private::build_cauldron_graph::<SelfImportCauldron>() {
        Ok(_) => panic!("self import must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE008);
    let cycle = format!(
        "{} -> {}",
        std::any::type_name::<SelfImportCauldron>(),
        std::any::type_name::<SelfImportCauldron>()
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains(&cycle),
        "unexpected diagnostic: {rendered}"
    );
    assert!(rendered.contains("cauldron_graph.rs"));
}

#[test]
fn multi_module_cycle_reports_stable_chain_and_locations() {
    use cycle::{first::FirstCycleCauldron, second::SecondCycleCauldron};

    let error = match furnace_rs_core::__private::build_cauldron_graph::<FirstCycleCauldron>() {
        Ok(_) => panic!("module cycle must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE008);
    let cycle = format!(
        "{} -> {} -> {}",
        std::any::type_name::<FirstCycleCauldron>(),
        std::any::type_name::<SecondCycleCauldron>(),
        std::any::type_name::<FirstCycleCauldron>()
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains(&cycle),
        "unexpected diagnostic: {rendered}"
    );
    assert_eq!(error.diagnostics().len(), 2);
    assert!(
        error
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.to_string().contains("cauldron_graph.rs"))
    );
}

#[test]
fn same_namespace_furnaces_do_not_claim_each_others_declarations() {
    use namespace_collision::{root::CollisionRoot, shared_namespace};
    let graph = furnace_rs_core::__private::build_cauldron_graph::<CollisionRoot>().unwrap();
    assert!(graph.cauldrons().iter().any(
        |module| module.type_id() == std::any::TypeId::of::<shared_namespace::FirstCauldron>()
    ));
    assert!(!graph.cauldrons().iter().any(
        |module| module.type_id() == std::any::TypeId::of::<shared_namespace::SecondCauldron>()
    ));
}

#[test]
fn rootless_provider_analysis_ignores_furnace_namespace_overlap() {
    assert!(furnace_rs_core::Furnace::builder().analyze().is_valid());
}
