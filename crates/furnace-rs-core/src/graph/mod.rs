//! Immutable provider graph inspection types.

use crate::Catalog;

mod analysis;
mod cauldron;
mod cycle;
mod focus;
mod inspection;
mod model;
mod plan;
mod scope;

pub use cauldron::{CauldronGraph, CauldronImportEdge, CauldronNode, ProviderOwnership};
#[doc(hidden)]
pub use inspection::{
    AutoConfigurationInspectionSnapshot, CauldronImportInspectionSnapshot,
    CauldronInspectionSnapshot, DependencyInspectionSnapshot, GraphInspectionSnapshot,
    OwnedSourceLocation, ProviderInspectionSnapshot,
};
pub use model::{
    ApplicationGraph, ConstructionPlan, ConstructionStep, DependencyEdge, GraphAnalysis,
    ProviderNode, ProviderOrigin, ProviderState,
};

pub(crate) use cauldron::build_cauldron_graph;
pub(crate) use focus::select_focused_providers;
pub(crate) use model::SatisfiedProvider;
pub(crate) use scope::select_scoped_providers;

pub(crate) fn analyze_descriptors(
    descriptors: &[&'static crate::ProviderDescriptor],
    satisfied: &[SatisfiedProvider],
    covered_missing: &[std::any::TypeId],
) -> GraphAnalysis {
    analysis::analyze_parts(descriptors, satisfied, covered_missing)
}

pub(crate) fn analyze_catalog(
    satisfied: &[SatisfiedProvider],
    covered_missing: &[std::any::TypeId],
) -> GraphAnalysis {
    let descriptors = Catalog::providers();
    analyze_descriptors(&descriptors, satisfied, covered_missing)
}

pub(crate) fn analyze_descriptors_with_locations(
    descriptors: &[&'static crate::ProviderDescriptor],
    satisfied: &[SatisfiedProvider],
    covered_missing: &[std::any::TypeId],
    locations: &[(std::any::TypeId, crate::SourceLocation)],
) -> GraphAnalysis {
    analysis::analyze_parts_with_locations(descriptors, satisfied, covered_missing, locations)
}
