//! Integration tests for deterministic rooted module graph construction.

#[macro_use]
#[path = "support/cauldron_metadata.rs"]
mod cauldron_metadata;
use furnace_rs_core::{CauldronNode, FURNACE008};

struct UndeclaredRoot;

mod diamond {

    pub mod shared {

        cauldron_metadata! {
            pub struct SharedCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        }
    }

    pub mod left {
        use super::shared::SharedCauldron;

        cauldron_metadata! {
            pub struct LeftCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<SharedCauldron>()
        }
    }

    pub mod right {
        use super::shared::SharedCauldron;

        cauldron_metadata! {
            pub struct RightCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<SharedCauldron>()
        }
    }

    pub mod root {
        use super::{left::LeftCauldron, right::RightCauldron};

        cauldron_metadata! {
            pub struct DiamondRoot;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<LeftCauldron>()
                .import::<RightCauldron>()
        }
    }
}

mod duplicate_import {

    pub mod leaf {

        cauldron_metadata! {
            pub struct LeafCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        }
    }

    pub mod root {
        use super::leaf::LeafCauldron;

        cauldron_metadata! {
            pub struct DuplicateImportRoot;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<LeafCauldron>()
                .import::<LeafCauldron>()
        }
    }
}

mod self_import {

    cauldron_metadata! {
        pub struct SelfImportCauldron;
        furnace_rs_core::__private::CauldronDefinitionBuilder::new()
            .import::<SelfImportCauldron>()
    }
}

mod cycle {

    pub mod first {
        use super::second::SecondCycleCauldron;

        cauldron_metadata! {
            pub struct FirstCycleCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<SecondCycleCauldron>()
        }
    }

    pub mod second {
        use super::first::FirstCycleCauldron;

        cauldron_metadata! {
            pub struct SecondCycleCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<FirstCycleCauldron>()
        }
    }
}

mod namespace_collision {

    pub mod shared_namespace {

        cauldron_metadata! {
            pub struct FirstCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        }

        cauldron_metadata! {
            pub struct SecondCauldron;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
        }
    }

    pub mod root {
        use super::shared_namespace::FirstCauldron;

        cauldron_metadata! {
            pub struct CollisionRoot;
            furnace_rs_core::__private::CauldronDefinitionBuilder::new()
                .import::<FirstCauldron>()
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
