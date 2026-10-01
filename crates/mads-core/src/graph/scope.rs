//! Explicit furnace ownership and dependency access enforcement.
use super::{ModuleGraph, ProviderOwnership, SatisfiedProvider};
use crate::{Diagnostic, MADS009, ProviderDescriptor};
use std::any::TypeId;

pub(crate) struct ScopedProviderCatalog {
    pub(crate) descriptors: Vec<&'static ProviderDescriptor>,
    pub(crate) ownership: Vec<ProviderOwnership>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) covered_missing: Vec<TypeId>,
}

pub(crate) fn select_scoped_providers(
    graph: &ModuleGraph,
    descriptors: &[&'static ProviderDescriptor],
    satisfied: &[SatisfiedProvider],
) -> ScopedProviderCatalog {
    let mut selected = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ownership = Vec::new();
    let mut covered_missing = Vec::new();
    for supplied in satisfied {
        if supplied.type_id != TypeId::of::<crate::Config>()
            && supplied.state != crate::ProviderState::AutoConfigured
            && graph.owner_of(supplied.type_id).is_none()
        {
            diagnostics.push(
                Diagnostic::new(
                    crate::MADS008,
                    "unregistered furnace override",
                    "rooted overrides must target an explicitly registered output",
                )
                .with_subject(supplied.type_name),
            );
        }
    }
    for (owner, member) in graph.members() {
        let matching: Vec<_> = descriptors
            .iter()
            .copied()
            .filter(|descriptor| descriptor.type_id() == member.type_id)
            .collect();
        let supplied = satisfied
            .iter()
            .find(|provider| provider.type_id == member.type_id);
        let overridden = supplied.is_some();
        ownership.push(ProviderOwnership {
            provider_type_name: supplied
                .map(|provider| provider.type_name)
                .or_else(|| matching.first().map(|descriptor| descriptor.type_name()))
                .unwrap_or(member.type_name),
            module_type_name: Some(owner.type_name()),
        });
        if matching.is_empty() && !overridden {
            diagnostics.push(
                Diagnostic::new(
                    crate::MADS003,
                    "missing registered provider",
                    "the explicitly registered output has no constructor metadata",
                )
                .with_subject(member.type_name)
                .with_location(member.location),
            );
        }
        if overridden {
            continue;
        }
        selected.extend(matching.iter().copied());
        for descriptor in matching {
            for dependency in descriptor.dependencies() {
                let target = dependency.type_id();
                let ambient = target == TypeId::of::<crate::Config>()
                    || graph.owner_of(target).is_none()
                        && satisfied.iter().any(|provider| {
                            provider.type_id == target
                                && provider.state == crate::ProviderState::AutoConfigured
                        });
                if ambient {
                    continue;
                }
                if let Some(target_owner) = graph.owner_of(target) {
                    if !graph.can_access(owner.type_id(), target) {
                        diagnostics.push(Diagnostic::new(MADS009, "inaccessible furnace provider",
                            format!("requester `{}` cannot access `{}` owned by `{}`; use an explicit export and direct import or a reachable global export",
                                owner.type_name(), dependency.type_name(), target_owner.type_name()))
                            .with_subject(dependency.type_name()).with_location(descriptor.location())
                            .with_suggestion(format!("dependency path: {} -> {}", descriptor.type_name(), dependency.type_name())));
                        covered_missing.push(target);
                    }
                }
            }
        }
    }
    ownership.sort_by_key(|record| record.provider_type_name);
    ScopedProviderCatalog {
        descriptors: selected,
        ownership,
        diagnostics,
        covered_missing,
    }
}
