//! Analysis-only gRPC preflight using the core's existing validator catalog.
use super::GrpcApplicationScope;
use furnace_rs_core::{
    __private::{PreflightContext, PreflightDescriptor},
    Diagnostic, SourceLocation,
};

/// Analysis-local scope shared with other integration evaluators.
#[derive(Clone)]
pub(crate) struct SelectedGrpcMetadata(pub(crate) Result<GrpcApplicationScope, Vec<Diagnostic>>);
impl SelectedGrpcMetadata {
    pub(crate) fn select(
        graph: Option<&furnace_rs_core::CauldronGraph>,
        focus: Option<std::any::TypeId>,
    ) -> Self {
        Self(
            focus
                .map_or_else(
                    || GrpcApplicationScope::for_cauldron_graph(graph),
                    GrpcApplicationScope::for_focus,
                )
                .map_err(|error| error.diagnostics().to_vec()),
        )
    }
    pub(crate) fn scope(self) -> furnace_rs_core::Result<GrpcApplicationScope> {
        self.0.map_err(|diagnostics| {
            let mut diagnostics = diagnostics.into_iter();
            furnace_rs_core::Error::from_diagnostics(
                diagnostics.next().expect("metadata error diagnostic"),
                diagnostics,
            )
        })
    }
}
/// Validates selected protobuf metadata before provider construction.
#[doc(hidden)]
pub fn preflight_grpc(context: &PreflightContext<'_>) -> Vec<Diagnostic> {
    let result = context
        .memoize(|| SelectedGrpcMetadata::select(context.cauldron_graph(), context.focus_type_id()))
        .scope()
        .and_then(|scope| {
            #[cfg(feature = "jwt")]
            {
                let guards = scope.guards();
                let strategies = if context.focus_type_id().is_some() {
                    crate::PassportStrategyCatalog::preflight_for_test(&guards, |id| {
                        context.has_output_type_id(id)
                    })?
                } else {
                    crate::PassportStrategyCatalog::preflight_scoped(
                        context.cauldron_graph(),
                        &guards,
                    )?
                };
                for binding in strategies.bindings() {
                    if binding
                        .provider_type_id()
                        .is_some_and(|id| !context.has_output_type_id(id))
                    {
                        return Err(furnace_rs_core::Error::new(
                            Diagnostic::new(
                                crate::FURNACE130,
                                "missing selected strategy output",
                                "selected gRPC strategy must be available in the application scope",
                            )
                            .with_subject(binding.strategy())
                            .with_location(binding.guard().location()),
                        ));
                    }
                }
                if !guards.is_empty()
                    && !context.has_output::<crate::JwtService>()
                    && !context
                        .diagnostics()
                        .iter()
                        .any(|d| d.code() == crate::FURNACE121)
                {
                    return Err(furnace_rs_core::Error::new(Diagnostic::new(
                        crate::FURNACE130,
                        "missing selected JWT output",
                        "selected gRPC policies require JwtService before construction",
                    )));
                }
            }
            #[cfg(not(feature = "jwt"))]
            let _ = scope;
            Ok(())
        });
    result
        .err()
        .map(|error| error.diagnostics().to_vec())
        .unwrap_or_default()
}
furnace_rs_core::__private::inventory::submit! {
    PreflightDescriptor::new("furnace.common.grpc.preflight", SourceLocation::new(file!(), line!(), column!()), preflight_grpc)
}

fn preflight_clients(context: &PreflightContext<'_>) -> Vec<Diagnostic> {
    let mut names = std::collections::BTreeSet::new();
    let mut diagnostics = Vec::new();
    let mut clients: Vec<_> =
        furnace_rs_core::__private::inventory::iter::<super::GrpcClientRegistration>
            .into_iter()
            .map(|registration| registration.descriptor)
            .collect();
    clients.sort_by_key(|client| (client.name(), client.output_type_name()));
    for client in clients {
        if !context.has_output_type_id(client.output_type_id()) {
            continue;
        }
        if !super::config::valid_client_name(client.name()) || !names.insert(client.name()) {
            diagnostics.push(
                Diagnostic::new(
                    super::FURNACE153,
                    "invalid named gRPC client",
                    "selected client names must be valid and unique",
                )
                .with_subject(client.name())
                .with_location(client.location()),
            );
        }
        if let Err(error) = super::GrpcClientConfig::parse(context.config(), client.name()) {
            diagnostics.extend(error.diagnostics().iter().cloned());
        }
    }
    diagnostics
}
furnace_rs_core::__private::inventory::submit! {
    PreflightDescriptor::new("furnace.common.grpc.clients.preflight", SourceLocation::new(file!(), line!(), column!()), preflight_clients)
}
