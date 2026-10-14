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
    pub(crate) fn select(graph: Option<&furnace_rs_core::CauldronGraph>) -> Self {
        Self(
            GrpcApplicationScope::for_cauldron_graph(graph)
                .map_err(|error| error.diagnostics().to_vec()),
        )
    }
}
/// Validates selected protobuf metadata before provider construction.
#[doc(hidden)]
pub fn preflight_grpc(context: &PreflightContext<'_>) -> Vec<Diagnostic> {
    let selected = context.memoize(|| SelectedGrpcMetadata::select(context.cauldron_graph()));
    selected.0.err().unwrap_or_default()
}
furnace_rs_core::__private::inventory::submit! {
    PreflightDescriptor::new("furnace.common.grpc.preflight", SourceLocation::new(file!(), line!(), column!()), preflight_grpc)
}
