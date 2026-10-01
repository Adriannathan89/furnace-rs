//! Selected HTTP integration validation over virtual application outputs.
use crate::http_scope::HttpApplicationScope;
use furnace_rs_core::__private::{PreflightContext, PreflightDescriptor};
use furnace_rs_core::{CauldronGraph, Diagnostic, Error, Result, SourceLocation};
use std::any::TypeId;

/// One analysis-local selection, including deterministic metadata failures.
#[derive(Clone)]
pub(crate) struct SelectedHttpMetadata(std::result::Result<HttpApplicationScope, Vec<Diagnostic>>);
impl SelectedHttpMetadata {
    pub(crate) fn select(graph: Option<&CauldronGraph>, focus: Option<TypeId>) -> Self {
        let scope = match focus {
            Some(target) => HttpApplicationScope::for_focus(target),
            None => HttpApplicationScope::for_cauldron_graph(graph),
        };
        Self(scope.map_err(|error| error.diagnostics().to_vec()))
    }
    pub(crate) fn scope(self) -> Result<HttpApplicationScope> {
        self.0.map_err(|diagnostics| {
            let mut diagnostics = diagnostics.into_iter();
            Error::from_diagnostics(
                diagnostics
                    .next()
                    .expect("metadata errors have a diagnostic"),
                diagnostics,
            )
        })
    }
}

/// Validates only selected controller occurrences without constructing application state.
pub fn preflight_http(context: &PreflightContext<'_>) -> Vec<Diagnostic> {
    let scope = context
        .memoize(|| SelectedHttpMetadata::select(context.cauldron_graph(), context.focus_type_id()))
        .scope();
    let result = scope.and_then(|scope| {
        crate::route::validate_scoped_descriptors(scope.controllers())?;
        #[cfg(feature = "jwt")]
        {
            let strategies = if context.focus_type_id().is_some() {
                crate::PassportStrategyCatalog::preflight_for_test(scope.guards())?
            } else {
                crate::PassportStrategyCatalog::preflight_scoped(context.cauldron_graph(), scope.guards())?
            };
            for binding in strategies.bindings() {
                if binding.provider_type_id().is_some_and(|type_id| !context.has_output_type_id(type_id)) {
                    return Err(Error::new(Diagnostic::new(crate::FURNACE130, "missing selected strategy output", "the selected custom strategy must be registered in the application scope or supplied by the focused fixture")
                        .with_subject(binding.strategy()).with_location(binding.guard().location())));
                }
            }
            if !scope.guards().is_empty() && !context.has_output::<crate::JwtService>() {
                // Preserve the existing official-default diagnostic instead of emitting a duplicate cause.
                if !context.diagnostics().iter().any(|diagnostic| diagnostic.code() == crate::FURNACE121) {
                    return Err(Error::new(
                        Diagnostic::new(
                            crate::FURNACE130,
                            "missing selected JWT output",
                            "selected controller seals require JwtService before construction",
                        )
                        .with_subject(std::any::type_name::<crate::JwtService>()),
                    ));
                }
            }
        }
        Ok(())
    });
    match result {
        Ok(()) => Vec::new(),
        Err(error) => error.diagnostics().to_vec(),
    }
}
furnace_rs_core::__private::inventory::submit! {
    PreflightDescriptor::new("furnace.common.http.preflight", SourceLocation::new(file!(), line!(), column!()), preflight_http)
}
