//! Selection of one registered provider and its dependency closure.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};

use super::{GraphAnalysis, SatisfiedProvider, analyze_descriptors};
use crate::{Diagnostic, FURNACE003, ProviderDescriptor};

/// Selects the target provider and reachable dependencies for an isolated fixture build.
pub(crate) fn select_focused_providers(
    target: TypeId,
    target_name: &'static str,
    providers: &[&'static ProviderDescriptor],
    supplied: &[SatisfiedProvider],
    required: &[TypeId],
    covered_missing: &[TypeId],
) -> GraphAnalysis {
    let mut by_type: HashMap<TypeId, Vec<&'static ProviderDescriptor>> = HashMap::new();

    for &descriptor in providers {
        by_type
            .entry(descriptor.type_id())
            .or_default()
            .push(descriptor);
    }

    let supplied_ids: HashSet<_> = supplied.iter().map(|value| value.type_id).collect();
    let required: HashSet<_> = required.iter().copied().collect();
    let mut pending = vec![target];
    let mut visited = HashSet::new();
    let mut selected = Vec::new();

    while let Some(type_id) = pending.pop() {
        if !visited.insert(type_id) {
            continue;
        }

        let supplied = supplied_ids.contains(&type_id);

        if required.contains(&type_id) {
            continue;
        }

        for &descriptor in by_type.get(&type_id).into_iter().flatten() {
            selected.push(descriptor);

            if !supplied {
                pending.extend(descriptor.dependencies().iter().map(|dep| dep.type_id()));
            }
        }
    }

    let supplied = supplied
        .iter()
        .filter(|value| {
            visited.contains(&value.type_id) || value.state == crate::ProviderState::AutoConfigured
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut analysis = analyze_descriptors(&selected, &supplied, covered_missing);

    // A supplied subject still has to name a registered declaration.
    if !selected.iter().any(|p| p.type_id() == target) {
        analysis.append_diagnostics(vec![
            Diagnostic::new(
                FURNACE003,
                "unresolved test subject",
                "the selected subject has no registered provider",
            )
            .with_subject(target_name),
        ]);
    }

    analysis
}
