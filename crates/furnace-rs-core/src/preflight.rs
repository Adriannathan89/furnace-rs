//! Metadata-only integration validation before any application construction.
use crate::{CauldronGraph, Config, Diagnostic, GraphAnalysis, SourceLocation};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;

/// An official integration's metadata-only validation callback.
#[doc(hidden)]
pub type PreflightValidator = for<'a> fn(&PreflightContext<'a>) -> Vec<Diagnostic>;

/// Static registration for an integration validator.
#[doc(hidden)]
pub struct PreflightDescriptor {
    identifier: &'static str,
    location: SourceLocation,
    validator: PreflightValidator,
}
impl PreflightDescriptor {
    /// Registers a validator under a stable integration identifier.
    pub const fn new(
        identifier: &'static str,
        location: SourceLocation,
        validator: PreflightValidator,
    ) -> Self {
        Self {
            identifier,
            location,
            validator,
        }
    }
}
inventory::collect!(PreflightDescriptor);

/// Selected virtual state available before default applications and constructors.
#[doc(hidden)]
pub struct PreflightContext<'a> {
    config: &'a Config,
    analysis: &'a GraphAnalysis,
    focus: Option<TypeId>,
    memo: &'a AnalysisMemo,
    supplied: &'a [crate::graph::SatisfiedProvider],
}
impl<'a> PreflightContext<'a> {
    pub(crate) const fn new(
        config: &'a Config,
        analysis: &'a GraphAnalysis,
        focus: Option<TypeId>,
        memo: &'a AnalysisMemo,
        supplied: &'a [crate::graph::SatisfiedProvider],
    ) -> Self {
        Self {
            config,
            analysis,
            focus,
            memo,
            supplied,
        }
    }
    /// Returns immutable builder configuration.
    pub const fn config(&self) -> &Config {
        self.config
    }
    /// Returns rooted ownership and visibility, when a root was selected.
    pub const fn cauldron_graph(&self) -> Option<&CauldronGraph> {
        self.analysis.cauldron_graph()
    }
    /// Returns the exact provider selected by an official focused fixture.
    pub const fn focus_type_id(&self) -> Option<TypeId> {
        self.focus
    }
    /// Reports whether a selected provider, supplied value, or virtual default owns an output.
    pub fn has_output<T: Send + Sync + 'static>(&self) -> bool {
        self.has_output_type_id(TypeId::of::<T>())
    }
    /// Reports availability for an integration's dynamically selected provider identity.
    pub fn has_output_type_id(&self, type_id: TypeId) -> bool {
        self.supplied.iter().any(|value| value.type_id == type_id)
            || self
                .analysis
                .graph()
                .providers
                .iter()
                .any(|node| node.type_id == type_id)
    }
    /// Returns diagnostics already emitted by graph and default-condition analysis.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.analysis.diagnostics()
    }
    /// Shares immutable integration metadata within this analysis only.
    pub fn memoize<T: Clone + 'static>(&self, build: impl FnOnce() -> T) -> T {
        self.memo.get_or_insert(build)
    }
}

#[derive(Default)]
pub(crate) struct AnalysisMemo {
    values: RefCell<HashMap<TypeId, Box<dyn Any>>>,
}
impl AnalysisMemo {
    pub(crate) fn get_or_insert<T: Clone + 'static>(&self, build: impl FnOnce() -> T) -> T {
        if let Some(value) = self.values.borrow().get(&TypeId::of::<T>()) {
            return value
                .downcast_ref::<T>()
                .expect("memo type identity matches its key")
                .clone();
        }
        let value = build();
        self.values
            .borrow_mut()
            .insert(TypeId::of::<T>(), Box::new(value.clone()));
        value
    }
}

pub(crate) fn validate(context: &PreflightContext<'_>) -> Vec<Diagnostic> {
    let mut descriptors: Vec<_> = inventory::iter::<PreflightDescriptor>.into_iter().collect();
    descriptors.sort_by_key(|descriptor| {
        (
            descriptor.identifier,
            descriptor.location.file,
            descriptor.location.line,
            descriptor.location.column,
        )
    });
    let mut diagnostics = Vec::new();
    for descriptor in descriptors {
        for diagnostic in (descriptor.validator)(context) {
            if !context.analysis.diagnostics().contains(&diagnostic)
                && !diagnostics.contains(&diagnostic)
            {
                diagnostics.push(diagnostic);
            }
        }
    }
    diagnostics
}
