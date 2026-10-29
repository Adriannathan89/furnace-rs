//! Metadata-only inspection: no provider resolution, listener, Redis, or TLS I/O.
use furnace_rs_common::__private::{
    DoctorCheck, SocketEndpointReport, SocketHandlerReport, SocketInspectionDescriptor,
    SourceReport,
};

fn inspect(
    graph: Option<&furnace_rs_core::CauldronGraph>,
    config: &furnace_rs_core::Config,
) -> furnace_rs_core::Result<(Vec<SocketEndpointReport>, Vec<DoctorCheck>)> {
    let Some(graph) = graph else {
        return Ok((Vec::new(), Vec::new()));
    };
    let scope = crate::SocketApplicationScope::for_cauldron_graph(Some(graph))?;
    let adapters = furnace_rs_core::__private::inventory::iter::<
        furnace_rs_common::runtime::RuntimeTransportDescriptor,
    >
        .into_iter()
        .map(|d| d.name)
        .collect::<Vec<_>>();
    let selection =
        furnace_rs_common::runtime::RuntimeSelection::parse_registered(config, &adapters)?;

    if selection.socket {
        crate::SocketServerConfig::parse(config)?;
    }

    #[cfg(feature = "jwt")]
    furnace_rs_common::PassportStrategyCatalog::preflight_scoped(
        Some(graph),
        &crate::admission::guards(Some(graph))?,
    )?;
    let endpoints = scope
        .endpoints()
        .iter()
        .map(|e| SocketEndpointReport {
            path: e.path.to_owned(),
            mode: match e.mode {
                crate::EndpointMode::Raw => "raw",
                crate::EndpointMode::Events => "events",
            }
            .into(),
            implementation: e.implementation.into(),
            processing: match e.processing {
                crate::ProcessingMode::Sequential => "sequential".into(),
                crate::ProcessingMode::Concurrent { max_concurrency } => {
                    format!("concurrent({max_concurrency})")
                }
            },
            location: SourceReport {
                file: e.location.file.into(),
                line: e.location.line,
                column: e.location.column,
            },
            #[cfg(feature = "jwt")]
            protection: e.guard.map(|g| protection((g.descriptor)())),
            #[cfg(not(feature = "jwt"))]
            protection: None,
            handlers: e
                .handlers
                .iter()
                .map(|h| SocketHandlerReport {
                    kind: h.kind.into(),
                    name: h.name.into(),
                })
                .collect(),
        })
        .collect();

    Ok((
        endpoints,
        vec![DoctorCheck::pass(
            "WebSocket registration",
            "selected endpoint metadata validated without runtime I/O",
        )],
    ))
}

furnace_rs_core::__private::inventory::submit! {SocketInspectionDescriptor{inspect}}

#[cfg(feature = "jwt")]
/// Converts a selected guard descriptor into the public inspection report.
fn protection(
    guard: &'static furnace_rs_common::GuardDescriptor,
) -> furnace_rs_common::__private::GrpcProtectionReport {
    use furnace_rs_common::__private::{GrpcPolicyReport, GrpcProtectionReport};
    use furnace_rs_common::{PolicyClause, PolicyMode};
    let clause = |clause: PolicyClause| GrpcPolicyReport {
        mode: match clause.mode() {
            PolicyMode::Any => "any",
            PolicyMode::All => "all",
        }
        .to_owned(),
        values: clause
            .values()
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    };

    GrpcProtectionReport {
        strategy: guard.strategy().to_owned(),
        principal: guard.principal_type_name().unwrap_or_default().to_owned(),
        roles: guard.roles().map(clause),
        permissions: guard.permissions().map(clause),
        predicates: guard
            .predicates()
            .iter()
            .map(|p| p.name().to_owned())
            .collect(),
    }
}
