//! Integration tests for deterministic rooted module graph construction.

use mads_core::{MADS008, ModuleNode, furnace};

struct UndeclaredRoot;

impl mads_core::Furnace for UndeclaredRoot {
    fn register(self) -> mads_core::FurnaceRegistration<Self> {
        mads_core::FurnaceRegistration::new(self)
    }
}

mod diamond {
    use super::furnace;

    pub mod shared {
        use super::furnace;

        #[furnace]
        pub struct SharedModule;

        impl mads_core::Furnace for SharedModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                mads_core::FurnaceRegistration::new(self)
            }
        }
    }

    pub mod left {
        use super::{furnace, shared::SharedModule};

        #[furnace]
        pub struct LeftModule;

        impl mads_core::Furnace for LeftModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(SharedModule)
            }
        }
    }

    pub mod right {
        use super::{furnace, shared::SharedModule};

        #[furnace]
        pub struct RightModule;

        impl mads_core::Furnace for RightModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(SharedModule)
            }
        }
    }

    pub mod root {
        use super::{furnace, left::LeftModule, right::RightModule};

        #[furnace]
        pub struct DiamondRoot;

        impl mads_core::Furnace for DiamondRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(LeftModule).import(RightModule)
            }
        }
    }
}

mod duplicate_import {
    use super::furnace;

    pub mod leaf {
        use super::furnace;

        #[furnace]
        pub struct LeafModule;

        impl mads_core::Furnace for LeafModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                mads_core::FurnaceRegistration::new(self)
            }
        }
    }

    pub mod root {
        use super::{furnace, leaf::LeafModule};

        #[furnace]
        pub struct DuplicateImportRoot;

        impl mads_core::Furnace for DuplicateImportRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(LeafModule).import(LeafModule)
            }
        }
    }
}

mod self_import {
    use super::furnace;

    #[furnace]
    pub struct SelfImportModule;

    impl mads_core::Furnace for SelfImportModule {
        fn register(self) -> mads_core::FurnaceRegistration<Self> {
            self.import(SelfImportModule)
        }
    }
}

mod cycle {
    use super::furnace;

    pub mod first {
        use super::{furnace, second::SecondCycleModule};

        #[furnace]
        pub struct FirstCycleModule;

        impl mads_core::Furnace for FirstCycleModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(SecondCycleModule)
            }
        }
    }

    pub mod second {
        use super::{first::FirstCycleModule, furnace};

        #[furnace]
        pub struct SecondCycleModule;

        impl mads_core::Furnace for SecondCycleModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(FirstCycleModule)
            }
        }
    }
}

mod namespace_collision {
    use super::furnace;

    pub mod shared_namespace {
        use super::furnace;

        #[furnace]
        pub struct FirstModule;

        impl mads_core::Furnace for FirstModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                mads_core::FurnaceRegistration::new(self)
            }
        }

        #[furnace]
        pub struct SecondModule;

        impl mads_core::Furnace for SecondModule {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                mads_core::FurnaceRegistration::new(self)
            }
        }
    }

    pub mod root {
        use super::{furnace, shared_namespace::FirstModule};

        #[furnace]
        pub struct CollisionRoot;

        impl mads_core::Furnace for CollisionRoot {
            fn register(self) -> mads_core::FurnaceRegistration<Self> {
                self.import(FirstModule)
            }
        }
    }
}

#[test]
fn diamond_graph_is_deterministic_and_deduplicated() {
    use diamond::{left::LeftModule, right::RightModule, root::DiamondRoot, shared::SharedModule};

    let graph = mads_core::__private::build_module_graph::<DiamondRoot>()
        .expect("diamond graph should be valid");
    assert_eq!(
        graph
            .modules()
            .iter()
            .map(ModuleNode::type_name)
            .collect::<Vec<_>>(),
        [
            std::any::type_name::<DiamondRoot>(),
            std::any::type_name::<LeftModule>(),
            std::any::type_name::<SharedModule>(),
            std::any::type_name::<RightModule>(),
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
        std::any::type_name::<LeftModule>()
    );
}

#[test]
fn missing_root_metadata_uses_a_stable_subject() {
    let error = match mads_core::__private::build_module_graph::<UndeclaredRoot>() {
        Ok(_) => panic!("undeclared root must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), MADS008);
    assert!(error.to_string().contains("subject: requested module"));
}

#[test]
fn duplicate_direct_import_reports_stable_subject_and_location() {
    use duplicate_import::root::DuplicateImportRoot;

    let error = match mads_core::__private::build_module_graph::<DuplicateImportRoot>() {
        Ok(_) => panic!("duplicate direct imports must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), MADS008);
    let rendered = error.to_string();
    assert!(rendered.contains(std::any::type_name::<DuplicateImportRoot>()));
    assert!(rendered.contains("module_graph.rs"));
}

#[test]
fn self_import_reports_the_cycle_and_source_location() {
    use self_import::SelfImportModule;

    let error = match mads_core::__private::build_module_graph::<SelfImportModule>() {
        Ok(_) => panic!("self import must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), MADS008);
    let cycle = format!(
        "{} -> {}",
        std::any::type_name::<SelfImportModule>(),
        std::any::type_name::<SelfImportModule>()
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains(&cycle),
        "unexpected diagnostic: {rendered}"
    );
    assert!(rendered.contains("module_graph.rs"));
}

#[test]
fn multi_module_cycle_reports_stable_chain_and_locations() {
    use cycle::{first::FirstCycleModule, second::SecondCycleModule};

    let error = match mads_core::__private::build_module_graph::<FirstCycleModule>() {
        Ok(_) => panic!("module cycle must fail"),
        Err(error) => error,
    };
    assert_eq!(error.code(), MADS008);
    let cycle = format!(
        "{} -> {} -> {}",
        std::any::type_name::<FirstCycleModule>(),
        std::any::type_name::<SecondCycleModule>(),
        std::any::type_name::<FirstCycleModule>()
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
            .all(|diagnostic| diagnostic.to_string().contains("module_graph.rs"))
    );
}

#[test]
fn same_namespace_furnaces_do_not_claim_each_others_declarations() {
    use namespace_collision::{root::CollisionRoot, shared_namespace};
    let graph = mads_core::__private::build_module_graph::<CollisionRoot>().unwrap();
    assert!(
        graph
            .modules()
            .iter()
            .any(|module| module.type_id()
                == std::any::TypeId::of::<shared_namespace::FirstModule>())
    );
    assert!(
        !graph
            .modules()
            .iter()
            .any(|module| module.type_id()
                == std::any::TypeId::of::<shared_namespace::SecondModule>())
    );
}

#[test]
fn rootless_provider_analysis_ignores_furnace_namespace_overlap() {
    assert!(mads_core::Mads::builder().analyze().is_valid());
}
