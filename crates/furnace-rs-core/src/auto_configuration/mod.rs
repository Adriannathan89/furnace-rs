//! Discovery and evaluation of framework-owned default providers.
//!
//! Evaluators inspect a stable catalog and configuration before constructors
//! run; selected appliers contribute provider values and infrastructure hooks
//! only after graph preflight succeeds.

mod analysis;
mod descriptor;
mod report;

use std::cmp::Ordering;

pub(crate) use analysis::analyze_parts_with_memo;
pub(crate) use descriptor::AutoConfigurationInputs;
pub use descriptor::{
    AutoConfigurationApplyContext, AutoConfigurationContext, AutoConfigurationContribution,
    AutoConfigurationDescriptor, AutoConfigurationEvaluation,
};

pub use report::{
    AutoConfigurationConfigEvidence, AutoConfigurationReasonCode, AutoConfigurationReport,
    AutoConfigurationRequirement, AutoConfigurationStatus,
};

inventory::collect!(AutoConfigurationDescriptor);

/// Collects the compiled-in integration descriptors in stable report order.
pub(crate) fn descriptors() -> Vec<&'static AutoConfigurationDescriptor> {
    let mut descriptors: Vec<_> = inventory::iter::<AutoConfigurationDescriptor>
        .into_iter()
        .collect();
    descriptors.sort_by(descriptor_order);

    descriptors
}

/// Orders descriptors by identifier, output type, and source location.
fn descriptor_order(
    left: &&'static AutoConfigurationDescriptor,
    right: &&'static AutoConfigurationDescriptor,
) -> Ordering {
    left.identifier()
        .cmp(right.identifier())
        .then_with(|| left.output_type_name().cmp(right.output_type_name()))
        .then_with(|| left.location().cmp(&right.location()))
}
