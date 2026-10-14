//! Root-selected protobuf metadata validation without application construction.
use super::{FURNACE150, GrpcGuard, GrpcServiceDescriptor, GrpcServiceRegistration};
use furnace_rs_core::{CauldronGraph, Diagnostic, Error, Furnace, Result};
use prost::Message;
use std::{
    any::TypeId,
    collections::{BTreeMap, BTreeSet},
};

/// One protobuf method with its effective static protection policy.
#[derive(Clone, Debug)]
pub struct GrpcMethodRecord {
    /// Exact protobuf method name.
    pub name: String,
    /// Canonical fully qualified RPC path.
    pub path: String,
    /// Whether the client sends a stream of messages.
    pub client_streaming: bool,
    /// Whether the server returns a stream of messages.
    pub server_streaming: bool,
    /// Effective service/method policy; absence means public.
    #[doc(hidden)]
    pub guard: Option<GrpcGuard>,
}

/// One explicitly selected service and the cauldron owning its implementation.
#[derive(Clone, Debug)]
pub struct ScopedGrpcService {
    descriptor: &'static GrpcServiceDescriptor,
    owner: Option<TypeId>,
    methods: Vec<GrpcMethodRecord>,
}
impl ScopedGrpcService {
    /// Returns the fully qualified protobuf service name.
    pub fn name(&self) -> &'static str {
        self.descriptor.name()
    }
    /// Returns the owning cauldron, if a root selected it.
    #[doc(hidden)]
    pub const fn owner(&self) -> Option<TypeId> {
        self.owner
    }
    /// Returns the static typed registration.
    #[doc(hidden)]
    pub const fn descriptor(&self) -> &'static GrpcServiceDescriptor {
        self.descriptor
    }
    /// Returns methods in protobuf-name order.
    pub fn methods(&self) -> &[GrpcMethodRecord] {
        &self.methods
    }
}

/// A validated gRPC service catalog confined to the selected application root.
#[derive(Clone, Debug)]
pub struct GrpcApplicationScope {
    services: Vec<ScopedGrpcService>,
    #[cfg(feature = "jwt")]
    root: Option<TypeId>,
    files: BTreeMap<String, prost_types::FileDescriptorProto>,
}
impl GrpcApplicationScope {
    /// Selects and validates registrations without constructing providers or reading files.
    #[allow(clippy::result_large_err)]
    pub fn for_cauldron_graph(graph: Option<&CauldronGraph>) -> Result<Self> {
        Self::select(graph, None)
    }
    /// Selects one managed implementation for focused fixtures.
    #[doc(hidden)]
    #[allow(clippy::result_large_err)]
    pub fn for_focus(target: TypeId) -> Result<Self> {
        Self::select(None, Some(target))
    }
    #[allow(clippy::result_large_err)]
    fn select(graph: Option<&CauldronGraph>, focus: Option<TypeId>) -> Result<Self> {
        let mut descriptors: Vec<_> =
            furnace_rs_core::__private::inventory::iter::<GrpcServiceRegistration>
                .into_iter()
                .map(|entry| entry.descriptor)
                .collect();
        if let Some(target) = focus {
            descriptors.retain(|d| d.provider_type_id() == target);
        }
        if let Some(graph) = graph {
            let selected: BTreeSet<_> = graph.registered_transport_providers("grpc").collect();
            descriptors.retain(|d| selected.contains(&d.provider_type_id()));
            if selected
                .iter()
                .any(|id| !descriptors.iter().any(|d| d.provider_type_id() == *id))
            {
                return Err(metadata_error(
                    None,
                    "registered gRPC provider is missing its service descriptor",
                ));
            }
        }
        descriptors.sort_by_key(|descriptor| (descriptor.name(), descriptor.implementation()));
        Self::validate(&descriptors, graph)
    }
    /// Returns validated service metadata selected for a constructed application.
    #[allow(clippy::result_large_err)]
    pub fn for_application(application: &Furnace) -> Result<Self> {
        Self::for_cauldron_graph(application.cauldron_graph())
    }
    #[cfg(feature = "jwt")]
    pub(crate) const fn root_type_id(&self) -> Option<TypeId> {
        self.root
    }
    /// Returns selected services in fully qualified protobuf-name order.
    pub fn services(&self) -> &[ScopedGrpcService] {
        &self.services
    }
    #[cfg(feature = "jwt")]
    pub(crate) fn guards(&self) -> Vec<crate::common::passport::__private::ScopedGuard> {
        self.services
            .iter()
            .flat_map(|service| {
                service.methods.iter().filter_map(move |method| {
                    method
                        .guard
                        .map(|guard| crate::common::passport::__private::ScopedGuard {
                            guard: guard.descriptor(),
                            occurrence: Some(crate::common::passport::__private::GuardOccurrence {
                                provider: service.descriptor.provider_type_id(),
                                operation: method.path.clone(),
                            }),
                            context_cauldron: service.owner,
                        })
                })
            })
            .collect()
    }
    /// Returns merged descriptor files; service filtering is applied by reflection.
    #[doc(hidden)]
    pub fn files(&self) -> &BTreeMap<String, prost_types::FileDescriptorProto> {
        &self.files
    }
    #[allow(clippy::result_large_err)]
    fn validate(
        descriptors: &[&'static GrpcServiceDescriptor],
        graph: Option<&CauldronGraph>,
    ) -> Result<Self> {
        let mut files = BTreeMap::new();
        let mut names = BTreeSet::new();
        let mut services = Vec::new();
        for descriptor in descriptors {
            if descriptor.name() == super::reflection::HEALTH
                || super::reflection::NAMES.contains(&descriptor.name())
            {
                return Err(metadata_error(
                    Some(descriptor),
                    "application service conflicts with a reserved infrastructure service",
                ));
            }
            if !names.insert(descriptor.name()) {
                return Err(metadata_error(
                    Some(descriptor),
                    "duplicate selected protobuf service",
                ));
            }
            let fds = prost_types::FileDescriptorSet::decode(descriptor.descriptor_set())
                .map_err(|_| metadata_error(Some(descriptor), "invalid protobuf descriptor set"))?;
            for mut file in fds.file {
                let file_name = file
                    .name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        metadata_error(Some(descriptor), "descriptor file name is missing")
                    })?;
                file.source_code_info = None;
                if let Some(existing) = files.get(&file_name) {
                    if existing != &file {
                        return Err(metadata_error(
                            Some(descriptor),
                            "conflicting protobuf file definitions",
                        ));
                    }
                } else {
                    files.insert(file_name, file);
                }
            }
        }
        for descriptor in descriptors {
            let matching: Vec<_> = files
                .values()
                .flat_map(|file| file.service.iter().map(move |service| (file, service)))
                .filter(|(file, service)| {
                    let name = service.name.as_deref().unwrap_or("");
                    let package = file.package.as_deref().unwrap_or("");
                    if package.is_empty() {
                        name == descriptor.name()
                    } else {
                        format!("{package}.{name}") == descriptor.name()
                    }
                })
                .collect();
            if matching.len() != 1 {
                return Err(metadata_error(
                    Some(descriptor),
                    "generated service identity must match exactly one descriptor declaration",
                ));
            }
            let (_, service) = matching[0];
            if service.method.is_empty() {
                return Err(metadata_error(
                    Some(descriptor),
                    "selected service has no protobuf methods",
                ));
            }
            let mut methods = Vec::new();
            let mut method_names = BTreeSet::new();
            for method in &service.method {
                let name = method
                    .name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        metadata_error(Some(descriptor), "protobuf method name is missing")
                    })?;
                if !method_names.insert(name.clone())
                    || method.input_type.as_deref().is_none_or(str::is_empty)
                    || method.output_type.as_deref().is_none_or(str::is_empty)
                {
                    return Err(metadata_error(
                        Some(descriptor),
                        "invalid or duplicate protobuf method declaration",
                    ));
                }
                let guard = descriptor
                    .overrides()
                    .iter()
                    .find(|entry| entry.wire_name == name)
                    .map_or(descriptor.guard(), |entry| entry.guard);
                #[cfg(feature = "jwt")]
                if let Some(guard) = guard {
                    crate::GuardCatalog::validate_descriptors(&[guard.descriptor()])?;
                    if guard.descriptor().source() != crate::TokenSource::Bearer {
                        return Err(metadata_error(
                            Some(descriptor),
                            "gRPC guards support authorization bearer metadata only",
                        ));
                    }
                }
                methods.push(GrpcMethodRecord {
                    path: format!("/{}/{name}", descriptor.name()),
                    name,
                    client_streaming: method.client_streaming.unwrap_or(false),
                    server_streaming: method.server_streaming.unwrap_or(false),
                    guard,
                });
            }
            if let Some(authored) = descriptor.rust_methods() {
                let actual: BTreeSet<_> = authored.iter().copied().collect();
                let expected: BTreeSet<_> = method_names
                    .iter()
                    .map(|name| rust_method_name(name))
                    .collect();
                if actual.len() != authored.len()
                    || actual != expected.iter().map(String::as_str).collect()
                {
                    return Err(metadata_error(
                        Some(descriptor),
                        "descriptor methods do not match the generated trait implementation",
                    ));
                }
            }
            let mut overrides = BTreeSet::new();
            for entry in descriptor.overrides() {
                let rust_name = rust_method_name(entry.wire_name);
                if rust_name != entry.rust_name {
                    return Err(metadata_error(
                        Some(descriptor),
                        "protobuf method override does not match its Rust implementation method",
                    ));
                }
                if !method_names.contains(entry.wire_name) || !overrides.insert(entry.wire_name) {
                    return Err(metadata_error(
                        Some(descriptor),
                        "unknown or duplicate protobuf method override",
                    ));
                }
            }
            methods.sort_by(|a, b| a.name.cmp(&b.name));
            services.push(ScopedGrpcService {
                descriptor,
                owner: graph.and_then(|g| {
                    g.owner_of(descriptor.provider_type_id())
                        .map(|owner| owner.type_id())
                }),
                methods,
            });
        }
        Ok(Self {
            services,
            files,
            #[cfg(feature = "jwt")]
            root: graph.map(|graph| graph.root().type_id()),
        })
    }
}
fn metadata_error(descriptor: Option<&GrpcServiceDescriptor>, message: &'static str) -> Error {
    let mut diagnostic = Diagnostic::new(FURNACE150, "invalid gRPC registration", message);
    if let Some(descriptor) = descriptor {
        diagnostic = diagnostic
            .with_subject(descriptor.name())
            .with_location(descriptor.location());
    }
    Error::new(diagnostic)
}

#[cfg(test)]
mod tests {
    use super::super::GrpcMethodOverride;
    use super::*;
    use furnace_grpc_test_support::FILE_DESCRIPTOR_SET;
    use furnace_rs_core::SourceLocation;

    fn descriptor(
        name: fn() -> &'static str,
        bytes: &'static [u8],
        overrides: &'static [GrpcMethodOverride],
    ) -> &'static GrpcServiceDescriptor {
        Box::leak(Box::new(GrpcServiceDescriptor::new(
            TypeId::of::<u64>,
            std::any::type_name::<u64>,
            name,
            bytes,
            None,
            overrides,
            SourceLocation::new(file!(), line!(), column!()),
        )))
    }

    #[test]
    fn matching_descriptor_files_are_deduplicated() {
        let first = descriptor(|| "furnace.test.v1.Transport", FILE_DESCRIPTOR_SET, &[]);
        let second = descriptor(|| "furnace.test.v1.Hidden", FILE_DESCRIPTOR_SET, &[]);
        let scope = GrpcApplicationScope::validate(&[first, second], None).unwrap();
        assert_eq!(scope.files().len(), 2);
        assert_eq!(scope.services().len(), 2);
    }

    #[test]
    fn descriptor_conflicts_fail_preconstruction() {
        let first = descriptor(|| "furnace.test.v1.Hidden", FILE_DESCRIPTOR_SET, &[]);
        let mut fds = prost_types::FileDescriptorSet::decode(FILE_DESCRIPTOR_SET).unwrap();
        fds.file[0].package = Some("other.package".into());
        let bytes = Box::leak(fds.encode_to_vec().into_boxed_slice());
        let second = descriptor(|| "other.package.Transport", bytes, &[]);
        assert_eq!(
            GrpcApplicationScope::validate(&[first, second], None)
                .unwrap_err()
                .code(),
            FURNACE150
        );
    }

    #[test]
    fn method_override_must_target_its_rust_implementation_method() {
        static OVERRIDES: [GrpcMethodOverride; 1] = [GrpcMethodOverride {
            wire_name: "Secret",
            rust_name: "other",
            guard: None,
        }];
        let d = descriptor(|| "furnace.test.v1.Hidden", FILE_DESCRIPTOR_SET, &OVERRIDES);
        assert_eq!(
            GrpcApplicationScope::validate(&[d], None)
                .unwrap_err()
                .code(),
            FURNACE150
        );
    }
}

fn rust_method_name(wire_name: &str) -> String {
    use heck::ToSnakeCase;
    let mut name = wire_name.to_snake_case();
    if matches!(name.as_str(), "self" | "super" | "crate" | "extern" | "_") {
        name.push('_');
    }
    name
}
