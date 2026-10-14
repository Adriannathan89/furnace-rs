//! Static root policy selection and descriptor filtering for both reflection protocols.
use super::{
    FURNACE150, FURNACE152, GrpcApplicationScope, GrpcReflectionDescriptor, GrpcServerConfig,
};
use furnace_rs_core::{Config, Diagnostic, Error, Result};
use prost::Message;
use std::collections::{BTreeMap, BTreeSet};
pub(crate) const NAMES: [&str; 2] = [
    "grpc.reflection.v1.ServerReflection",
    "grpc.reflection.v1alpha.ServerReflection",
];
pub(crate) const HEALTH: &str = "grpc.health.v1.Health";
#[cfg(feature = "jwt")]
pub(crate) fn path(name: &str) -> String {
    format!("/{name}/ServerReflectionInfo")
}
/// Builds the descriptor set used by enabled reflection for selected services and infrastructure.
#[allow(clippy::result_large_err)]
pub fn reflection_descriptors(
    scope: &GrpcApplicationScope,
) -> Result<prost_types::FileDescriptorSet> {
    let mut files = scope.files().clone();
    let mut selected: BTreeSet<_> = scope
        .services()
        .iter()
        .map(|service| service.name())
        .collect();
    selected.insert(HEALTH);
    selected.extend(NAMES);
    for bytes in [
        tonic_health::pb::FILE_DESCRIPTOR_SET,
        tonic_reflection::pb::v1::FILE_DESCRIPTOR_SET,
        tonic_reflection::pb::v1alpha::FILE_DESCRIPTOR_SET,
    ] {
        merge_files(&mut files, bytes)?;
    }
    for file in files.values_mut() {
        let package = file.package.as_deref().unwrap_or("");
        file.service.retain(|service| {
            let name = service.name.as_deref().unwrap_or("");
            let qualified = if package.is_empty() {
                name.to_owned()
            } else {
                format!("{package}.{name}")
            };
            selected.contains(qualified.as_str())
        });
        file.source_code_info = None;
    }
    Ok(prost_types::FileDescriptorSet {
        file: files.into_values().collect(),
    })
}
fn merge_files(
    files: &mut BTreeMap<String, prost_types::FileDescriptorProto>,
    bytes: &[u8],
) -> Result<()> {
    let fds = prost_types::FileDescriptorSet::decode(bytes).map_err(|_| metadata_error())?;
    for mut file in fds.file {
        file.source_code_info = None;
        let name = file.name.clone().ok_or_else(metadata_error)?;
        if let Some(existing) = files.get(&name) {
            if existing != &file {
                return Err(metadata_error());
            }
        } else {
            files.insert(name, file);
        }
    }
    Ok(())
}
fn metadata_error() -> Error {
    Error::new(Diagnostic::new(
        FURNACE150,
        "conflicting infrastructure protobuf descriptors",
        "descriptor files must have matching definitions for every shared file name",
    ))
}
pub(crate) fn selected_policy(
    scope: &GrpcApplicationScope,
    config: &Config,
) -> Result<Option<&'static GrpcReflectionDescriptor>> {
    if !GrpcServerConfig::parse(config)?.reflection_enabled() {
        return Ok(None);
    }
    #[cfg(not(feature = "jwt"))]
    {
        let _ = scope;
        Err(policy_error("enabled reflection requires the jwt feature"))
    }
    #[cfg(feature = "jwt")]
    {
        let root = scope
            .root_type_id()
            .ok_or_else(|| policy_error("enabled reflection requires a selected cauldron root"))?;
        let selected: Vec<_> =
            furnace_rs_core::__private::inventory::iter::<super::GrpcReflectionRegistration>
                .into_iter()
                .map(|entry| entry.descriptor)
                .filter(|descriptor| descriptor.root() == root)
                .collect();
        let [policy] = selected.as_slice() else {
            return Err(policy_error(
                "enabled reflection requires exactly one typed policy for the selected root",
            ));
        };
        crate::GuardCatalog::validate_descriptors(&[policy.guard().descriptor()])?;
        if policy.guard().descriptor().source() != crate::TokenSource::Bearer {
            return Err(policy_error(
                "gRPC reflection supports authorization bearer metadata only",
            ));
        }
        reflection_descriptors(scope)?;
        Ok(Some(*policy))
    }
}
fn policy_error(message: &'static str) -> Error {
    Error::new(Diagnostic::new(
        FURNACE152,
        "invalid gRPC reflection protection",
        message,
    ))
}
#[cfg(feature = "jwt")]
pub(crate) fn guards(
    scope: &GrpcApplicationScope,
    config: &Config,
) -> Result<Vec<crate::common::passport::__private::ScopedGuard>> {
    let mut guards = scope.guards();
    if let Some(policy) = selected_policy(scope, config)? {
        for name in NAMES {
            guards.push(crate::common::passport::__private::ScopedGuard {
                guard: policy.guard().descriptor(),
                occurrence: Some(crate::common::passport::__private::GuardOccurrence {
                    provider: policy.root(),
                    operation: path(name),
                }),
                context_cauldron: Some(policy.root()),
            });
        }
    }
    Ok(guards)
}
#[cfg(feature = "jwt")]
pub(crate) fn bindings(
    scope: &GrpcApplicationScope,
    app: &furnace_rs_core::Furnace,
    name: &str,
) -> Result<BTreeMap<String, Option<crate::PassportStrategyBinding<'static>>>> {
    let guards = guards(scope, app.context().config())?;
    let preflight =
        crate::PassportStrategyCatalog::preflight_scoped(app.cauldron_graph(), &guards)?;
    let policy = selected_policy(scope, app.context().config())?
        .expect("reflection installation requires selected policy");
    let path = path(name);
    let occurrence = crate::common::passport::__private::GuardOccurrence {
        provider: policy.root(),
        operation: path.clone(),
    };
    let binding = preflight
        .binding_for_occurrence(&occurrence, Some(policy.root()))
        .expect("reflection guard was preflighted")
        .clone();
    Ok(BTreeMap::from([(path, Some(binding))]))
}
