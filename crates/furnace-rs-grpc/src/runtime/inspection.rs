//! Static selected protobuf metadata, with no provider or TLS-file access.
use furnace_rs_common::__private::{
    DoctorCheck, GrpcMethodReport, GrpcServiceReport, InspectionTransportDescriptor, SourceReport,
};
use furnace_rs_core::{CauldronGraph, Config, Result};
#[allow(clippy::result_large_err)]
/// Reports selected gRPC services and listener checks without opening a socket.
fn inspect(
    graph: Option<&CauldronGraph>,
    config: &Config,
) -> Result<(Vec<GrpcServiceReport>, Vec<DoctorCheck>)> {
    let Some(graph) = graph else {
        return Ok((Vec::new(), Vec::new()));
    };
    let scope = crate::GrpcApplicationScope::for_cauldron_graph(Some(graph))?;
    #[cfg(feature = "jwt")]
    crate::PassportStrategyCatalog::preflight_scoped(
        Some(graph),
        &crate::grpc::reflection::guards(&scope, config)?,
    )?;
    let mut checks = Vec::new();
    checks.push(DoctorCheck::pass(
        "gRPC registration",
        "selected protobuf service and method metadata validated",
    ));
    let adapters: Vec<_> = furnace_rs_core::__private::inventory::iter::<
        furnace_rs_common::runtime::RuntimeTransportDescriptor,
    >
        .into_iter()
        .collect();
    let selection = furnace_rs_common::runtime::RuntimeSelection::parse(
        config,
        adapters.iter().any(|a| a.name == "http"),
        true,
    )?;
    if selection.grpc {
        crate::GrpcServerConfig::parse(config)?;
        checks.push(DoctorCheck::pass(
            "gRPC configuration",
            "selected listener configuration validated without startup I/O",
        ));
    } else {
        checks.push(DoctorCheck::skipped(
            "gRPC configuration",
            "gRPC listener is not selected",
        ));
    }
    let mut services = Vec::new();
    for service in scope.services() {
        let descriptor = service.descriptor();
        let location = descriptor.location();
        services.push(GrpcServiceReport {
            name: service.name().to_owned(),
            implementation: descriptor.implementation().to_owned(),
            location: SourceReport {
                file: location.file.to_owned(),
                line: location.line,
                column: location.column,
            },
            methods: service
                .methods()
                .iter()
                .map(|method| GrpcMethodReport {
                    name: method.name.clone(),
                    path: method.path.clone(),
                    client_streaming: method.client_streaming,
                    server_streaming: method.server_streaming,
                    #[cfg(feature = "jwt")]
                    protection: method.guard.map(|guard| protection(guard.descriptor())),
                    #[cfg(not(feature = "jwt"))]
                    protection: None,
                })
                .collect(),
        });
    }
    #[cfg(feature = "jwt")]
    checks.push(DoctorCheck::pass(
        "gRPC protection",
        "effective admission policies included in shared Passport preflight",
    ));
    Ok((services, checks))
}
#[cfg(feature = "jwt")]
/// Converts a selected guard descriptor into the public inspection report.
fn protection(
    guard: &'static crate::GuardDescriptor,
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
furnace_rs_core::__private::inventory::submit! { InspectionTransportDescriptor { name: "gRPC", inspect } }
