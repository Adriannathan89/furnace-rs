//! Selection of one registered provider and its dependency closure.

use std::any::TypeId;

use super::{GraphAnalysis, SatisfiedProvider, analyze_descriptors};
use crate::{Diagnostic, MADS003, ProviderDescriptor};

pub(crate) fn select_focused_providers(
    target: TypeId,
    target_name: &'static str,
    providers: &[&'static ProviderDescriptor],
    supplied: &[SatisfiedProvider],
    required: &[TypeId],
) -> GraphAnalysis {
    let mut pending = vec![target];
    let mut visited = Vec::new();
    let mut selected = Vec::new();
    while let Some(type_id) = pending.pop() {
        if visited.contains(&type_id) {
            continue;
        }
        visited.push(type_id);
        let supplied = supplied.iter().any(|value| value.type_id == type_id);
        if required.contains(&type_id) {
            continue;
        }
        for descriptor in providers.iter().filter(|p| p.type_id() == type_id) {
            selected.push(*descriptor);
            if !supplied {
                pending.extend(descriptor.dependencies().iter().map(|dep| dep.type_id()));
            }
        }
    }
    let supplied = supplied
        .iter()
        .filter(|value| visited.contains(&value.type_id))
        .cloned()
        .collect::<Vec<_>>();
    let mut analysis = analyze_descriptors(&selected, &supplied, &[]);
    // A supplied subject still has to name a registered declaration.
    if !selected.iter().any(|p| p.type_id() == target) {
        analysis.append_diagnostics(vec![
            Diagnostic::new(
                MADS003,
                "unresolved test subject",
                "the selected subject has no registered provider",
            )
            .with_subject(target_name),
        ]);
    }
    analysis
}
