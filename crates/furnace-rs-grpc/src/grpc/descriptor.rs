//! Static protobuf service and guard declarations emitted by registration macros.
use furnace_rs_core::SourceLocation;
use std::{any::TypeId, fmt};

/// A typed Passport guard reference carried without constructing a policy.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct GrpcGuard {
    #[cfg(not(feature = "jwt"))]
    _private: (),
    #[cfg(feature = "jwt")]
    descriptor: fn() -> &'static crate::GuardDescriptor,
}

impl GrpcGuard {
    /// Captures the existing typed guard contract.
    #[cfg(feature = "jwt")]
    pub const fn of<G: crate::GuardPolicy>() -> Self {
        Self {
            descriptor: G::descriptor,
        }
    }

    /// Returns the underlying static Passport policy.
    #[cfg(feature = "jwt")]
    pub fn descriptor(self) -> &'static crate::GuardDescriptor {
        (self.descriptor)()
    }
}

impl fmt::Debug for GrpcGuard {
    /// Formats the registration identity without exposing function pointers.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GrpcGuard(..)")
    }
}

/// One explicitly public or guarded protobuf method override.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct GrpcMethodOverride {
    /// Exact protobuf method name.
    pub wire_name: &'static str,
    /// Rust method receiving the override, checked against protobuf generation.
    pub rust_name: &'static str,
    /// A replacement guard; absence is an explicitly public override.
    pub guard: Option<GrpcGuard>,
}

/// Static registration for one generated Tonic service implementation.
#[doc(hidden)]
pub struct GrpcServiceDescriptor {
    provider: fn() -> TypeId,
    implementation: fn() -> &'static str,
    wire_name: fn() -> &'static str,
    descriptor_set: &'static [u8],
    guard: Option<GrpcGuard>,
    overrides: &'static [GrpcMethodOverride],
    rust_methods: Option<&'static [&'static str]>,
    location: SourceLocation,
    registrar: Option<super::GrpcServiceRegistrar>,
}

impl GrpcServiceDescriptor {
    /// Creates immutable registration data without resolving a service.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        provider: fn() -> TypeId,
        implementation: fn() -> &'static str,
        wire_name: fn() -> &'static str,
        descriptor_set: &'static [u8],
        guard: Option<GrpcGuard>,
        overrides: &'static [GrpcMethodOverride],
        location: SourceLocation,
    ) -> Self {
        Self {
            provider,
            implementation,
            wire_name,
            descriptor_set,
            guard,
            overrides,
            location,
            registrar: None,
            rust_methods: None,
        }
    }

    /// Captures all authored generated-trait methods for descriptor coverage validation.
    #[doc(hidden)]
    pub const fn with_rust_methods(mut self, methods: &'static [&'static str]) -> Self {
        self.rust_methods = Some(methods);

        self
    }

    /// Returns the authored Rust trait methods used for descriptor coverage checks.
    pub(crate) const fn rust_methods(&self) -> Option<&'static [&'static str]> {
        self.rust_methods
    }

    /// Returns the selected DI output identity.
    pub fn provider_type_id(&self) -> TypeId {
        (self.provider)()
    }

    /// Returns the Rust implementation's identity.
    pub fn implementation(&self) -> &'static str {
        (self.implementation)()
    }

    /// Returns the generated fully qualified protobuf service name.
    pub fn name(&self) -> &'static str {
        (self.wire_name)()
    }

    /// Returns encoded protobuf file descriptors for this registration.
    pub const fn descriptor_set(&self) -> &'static [u8] {
        self.descriptor_set
    }

    /// Returns the inherited service-wide protection.
    pub const fn guard(&self) -> Option<GrpcGuard> {
        self.guard
    }

    /// Returns explicit method-level policy overrides.
    pub const fn overrides(&self) -> &'static [GrpcMethodOverride] {
        self.overrides
    }

    /// Returns the authored declaration location.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// Attaches a typed generated server adapter.
    pub const fn with_registrar(mut self, registrar: super::GrpcServiceRegistrar) -> Self {
        self.registrar = Some(registrar);

        self
    }

    /// Returns the typed generated server adapter.
    pub const fn registrar(&self) -> Option<super::GrpcServiceRegistrar> {
        self.registrar
    }
}

impl fmt::Debug for GrpcServiceDescriptor {
    /// Formats the registration identity without exposing function pointers.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GrpcServiceDescriptor")
            .field("service", &self.name())
            .field("implementation", &self.implementation())
            .finish_non_exhaustive()
    }
}

/// Link-time inventory wrapper for a generated service descriptor.
#[doc(hidden)]
pub struct GrpcServiceRegistration {
    /// Registered descriptor, with no application instance attached.
    pub descriptor: &'static GrpcServiceDescriptor,
}

furnace_rs_core::__private::inventory::collect!(GrpcServiceRegistration);

/// Static identity of one DI-managed named remote client.
#[doc(hidden)]
pub struct GrpcClientDescriptor {
    name: &'static str,
    output_type_id: fn() -> TypeId,
    output_type_name: fn() -> &'static str,
    location: SourceLocation,
}

impl GrpcClientDescriptor {
    /// Creates named-client metadata without constructing the wrapper.
    pub const fn new(
        name: &'static str,
        output_type_id: fn() -> TypeId,
        output_type_name: fn() -> &'static str,
        location: SourceLocation,
    ) -> Self {
        Self {
            name,
            output_type_id,
            output_type_name,
            location,
        }
    }

    /// Returns the configuration name.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the wrapper's exact native output identity.
    pub fn output_type_id(&self) -> TypeId {
        (self.output_type_id)()
    }

    /// Returns the wrapper's Rust type name.
    pub fn output_type_name(&self) -> &'static str {
        (self.output_type_name)()
    }

    /// Returns the declaration source.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }
}

/// Links one named client for side-effect-free preflight and inspection.
#[doc(hidden)]
pub struct GrpcClientRegistration {
    /// Named client metadata.
    pub descriptor: &'static GrpcClientDescriptor,
}

furnace_rs_core::__private::inventory::collect!(GrpcClientRegistration);

/// One typed policy belonging to a cauldron root's optional reflection service.
#[doc(hidden)]
pub struct GrpcReflectionDescriptor {
    root: fn() -> TypeId,
    guard: GrpcGuard,
    location: SourceLocation,
}

impl GrpcReflectionDescriptor {
    /// Creates a root policy without constructing a provider.
    pub const fn new(root: fn() -> TypeId, guard: GrpcGuard, location: SourceLocation) -> Self {
        Self {
            root,
            guard,
            location,
        }
    }

    /// Returns the root identity.
    pub fn root(&self) -> TypeId {
        (self.root)()
    }

    /// Returns the typed policy.
    pub const fn guard(&self) -> GrpcGuard {
        self.guard
    }

    /// Returns the declaration source.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }
}

/// Linked root reflection metadata.
#[doc(hidden)]
pub struct GrpcReflectionRegistration {
    /// Static policy declaration.
    pub descriptor: &'static GrpcReflectionDescriptor,
}

furnace_rs_core::__private::inventory::collect!(GrpcReflectionRegistration);
