//! Static metadata contracts emitted by provider and module declarations.

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::lifecycle::LifecycleRegistration;
use crate::{ConstructionContext, ErasedProvider, LifecycleResource, Result, SourceLocation};

/// Static import metadata retained for low-level descriptor consumers.
pub struct CauldronImportDescriptor {
    type_name: &'static str,
    type_id: fn() -> TypeId,
}

impl CauldronImportDescriptor {
    /// Creates an import descriptor from a stable type name and identifier factory.
    pub const fn new(type_name: &'static str, type_id: fn() -> TypeId) -> Self {
        Self { type_name, type_id }
    }

    /// Returns the imported module's authored type name.
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns the imported module's runtime type identifier.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }
}

/// Categorizes the role a provider plays in an application.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProviderKind {
    /// A provider implementing application service behavior.
    Service,
    /// A provider implementing persistence access behavior.
    Repository,
    /// A general-purpose provider.
    Provider,
}

/// Describes whether a provider declaration is public outside its Rust module.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProviderVisibility {
    /// The provider is public outside its Rust module.
    Public,
    /// The provider is private to its Rust module.
    Private,
}

/// The asynchronous result of constructing an erased provider.
pub type ProviderFuture<'a> = Pin<Box<dyn Future<Output = Result<ErasedProvider>> + Send + 'a>>;

/// Constructs a provider using the dependencies and configuration available at startup.
pub type ProviderConstructor = for<'a> fn(&'a ConstructionContext<'a>) -> ProviderFuture<'a>;

/// A type-erased provider and lifecycle registrations produced together.
#[doc(hidden)]
pub struct ProviderContribution {
    provider: ErasedProvider,
    registrations: Vec<LifecycleRegistration>,
}

impl ProviderContribution {
    /// Creates a contribution containing an ordinary provider and no hooks.
    #[doc(hidden)]
    pub fn from_provider(provider: ErasedProvider) -> Self {
        Self {
            provider,
            registrations: Vec::new(),
        }
    }

    /// Erases the native value while retaining its authored lifecycle hooks.
    #[doc(hidden)]
    pub fn from_resource<T>(resource: LifecycleResource<T>) -> Self
    where
        T: Send + Sync + 'static,
    {
        let (value, registrations) = resource.into_parts();
        Self {
            provider: Arc::new(value),
            registrations,
        }
    }

    pub(crate) fn into_parts(self) -> (ErasedProvider, Vec<LifecycleRegistration>) {
        (self.provider, self.registrations)
    }

    /// Returns only the native provider, discarding unregistered hook metadata.
    #[doc(hidden)]
    pub fn into_provider(self) -> ErasedProvider {
        self.into_parts().0
    }
}

/// The asynchronous result of constructing a provider with lifecycle metadata.
#[doc(hidden)]
pub type LifecycleProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderContribution>> + Send + 'a>>;

/// Constructs a provider and its lifecycle registrations together.
#[doc(hidden)]
pub type LifecycleProviderConstructor =
    for<'a> fn(&'a ConstructionContext<'a>) -> LifecycleProviderFuture<'a>;

/// Describes a provider dependency by its stable type metadata.
pub struct DependencyDescriptor {
    type_name: &'static str,
    type_id: fn() -> TypeId,
    runtime_type_name: Option<fn() -> &'static str>,
}

impl DependencyDescriptor {
    /// Creates a dependency descriptor from a stable type name and identifier factory.
    pub const fn new(type_name: &'static str, type_id: fn() -> TypeId) -> Self {
        Self {
            type_name,
            type_id,
            runtime_type_name: None,
        }
    }

    /// Attaches a concrete name resolver for generic dependency metadata.
    #[doc(hidden)]
    pub const fn with_runtime_type_name(mut self, name: fn() -> &'static str) -> Self {
        self.runtime_type_name = Some(name);
        self
    }

    /// Returns the dependency's stable type name.
    pub fn type_name(&self) -> &'static str {
        self.runtime_type_name.map_or(self.type_name, |name| name())
    }

    /// Returns the dependency's runtime type identifier.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }
}

/// Describes a statically declared provider and its constructor.
pub struct ProviderDescriptor {
    controller: bool,
    kind: ProviderKind,
    type_name: &'static str,
    type_id: fn() -> TypeId,
    runtime_type_name: Option<fn() -> &'static str>,
    resolved_type_name: Option<fn() -> &'static str>,
    namespace: Option<&'static str>,
    dependencies: &'static [DependencyDescriptor],
    visibility: ProviderVisibility,
    location: SourceLocation,
    constructor: ProviderConstructor,
    lifecycle_constructor: Option<LifecycleProviderConstructor>,
}

impl ProviderDescriptor {
    /// Creates a provider descriptor from static declaration metadata.
    pub const fn new(
        kind: ProviderKind,
        type_name: &'static str,
        type_id: fn() -> TypeId,
        dependencies: &'static [DependencyDescriptor],
        visibility: ProviderVisibility,
        location: SourceLocation,
        constructor: ProviderConstructor,
    ) -> Self {
        Self {
            controller: false,
            kind,
            type_name,
            type_id,
            runtime_type_name: None,
            resolved_type_name: None,
            namespace: None,
            dependencies,
            visibility,
            location,
            constructor,
            lifecycle_constructor: None,
        }
    }

    /// Marks constructor metadata emitted by a managed controller declaration.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_controller(mut self) -> Self {
        self.controller = true;
        self
    }

    /// Returns whether this declaration describes a managed controller.
    #[doc(hidden)]
    pub const fn is_controller(&self) -> bool {
        self.controller
    }

    /// Attaches the constructor that retains lifecycle registrations.
    #[doc(hidden)]
    #[must_use]
    pub const fn with_lifecycle_constructor(
        mut self,
        constructor: LifecycleProviderConstructor,
    ) -> Self {
        self.lifecycle_constructor = Some(constructor);
        self
    }

    /// Attaches the resolved Rust type name emitted by a provider macro.
    ///
    /// This document-hidden metadata lets catalog consumers compare types
    /// across separately compiled crate instances while retaining `TypeId` as
    /// the primary identity mechanism.
    #[doc(hidden)]
    pub const fn with_runtime_type_name(mut self, runtime_type_name: fn() -> &'static str) -> Self {
        self.runtime_type_name = Some(runtime_type_name);
        self
    }

    /// Attaches the Rust namespace containing this provider declaration.
    #[must_use]
    pub const fn with_namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Returns the provider's role.
    pub const fn kind(&self) -> ProviderKind {
        self.kind
    }

    /// Returns the provider's stable output type name.
    pub fn type_name(&self) -> &'static str {
        self.resolved_type_name
            .map_or(self.type_name, |name| name())
    }

    /// Resolves generic output names without changing authored macro names.
    #[doc(hidden)]
    pub const fn with_resolved_type_name(mut self, name: fn() -> &'static str) -> Self {
        self.resolved_type_name = Some(name);
        self
    }

    /// Retains an authored integration name instead of a generic resolved name.
    #[doc(hidden)]
    pub const fn with_authored_type_name(mut self, name: &'static str) -> Self {
        self.type_name = name;
        self.resolved_type_name = None;
        self
    }

    /// Records the declaration that supplies generic integration metadata.
    #[doc(hidden)]
    pub const fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }

    /// Retains an integration's authored visibility for catalog inspection.
    #[doc(hidden)]
    pub const fn with_visibility(mut self, visibility: ProviderVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Returns the provider's runtime output type identifier.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the resolved Rust output type name emitted by a provider macro.
    #[doc(hidden)]
    pub fn runtime_type_name(&self) -> Option<&'static str> {
        self.runtime_type_name
            .map(|runtime_type_name| runtime_type_name())
    }

    /// Returns the Rust namespace containing this provider declaration, when available.
    pub const fn namespace(&self) -> Option<&'static str> {
        self.namespace
    }

    /// Returns the static dependency descriptors required by this provider.
    pub const fn dependencies(&self) -> &'static [DependencyDescriptor] {
        self.dependencies
    }

    /// Returns the provider's declaration visibility metadata.
    pub const fn visibility(&self) -> ProviderVisibility {
        self.visibility
    }

    /// Returns the provider declaration's source location.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// Returns the provider constructor.
    pub const fn constructor(&self) -> ProviderConstructor {
        self.constructor
    }

    /// Returns the lifecycle-aware constructor, when this provider has one.
    #[doc(hidden)]
    pub const fn lifecycle_constructor(&self) -> Option<LifecycleProviderConstructor> {
        self.lifecycle_constructor
    }
}

/// Describes a statically declared application module.
pub struct CauldronDescriptor {
    registration: Option<fn() -> crate::CauldronDefinition>,
    type_name: &'static str,
    type_id: fn() -> TypeId,
    namespace: Option<&'static str>,
    imports: &'static [CauldronImportDescriptor],
    global: bool,
    location: SourceLocation,
}

impl CauldronDescriptor {
    /// Creates a module descriptor from static declaration metadata.
    pub const fn new(
        type_name: &'static str,
        type_id: fn() -> TypeId,
        location: SourceLocation,
    ) -> Self {
        Self {
            type_name,
            type_id,
            namespace: None,
            imports: &[],
            registration: None,
            global: false,
            location,
        }
    }

    /// Attaches an explicit cauldron registration callback.
    #[must_use]
    pub const fn with_registration(mut self, callback: fn() -> crate::CauldronDefinition) -> Self {
        self.registration = Some(callback);
        self
    }

    /// Returns the explicit registration callback.
    pub const fn registration(&self) -> Option<fn() -> crate::CauldronDefinition> {
        self.registration
    }

    /// Attaches the Rust namespace containing this module declaration.
    #[must_use]
    pub const fn with_namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Attaches the module's direct imports in authored declaration order.
    #[must_use]
    pub const fn with_imports(mut self, imports: &'static [CauldronImportDescriptor]) -> Self {
        self.imports = imports;
        self
    }

    /// Marks the module as global, allowing its providers to be accessed from any module.
    #[must_use]
    pub const fn with_global(mut self) -> Self {
        self.global = true;
        self
    }

    /// Returns the module's global declaration status.
    pub const fn is_global(&self) -> bool {
        self.global
    }

    /// Returns the module's stable type name.
    pub const fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Returns the module's runtime type identifier.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the Rust namespace containing this module declaration, when available.
    pub const fn namespace(&self) -> Option<&'static str> {
        self.namespace
    }

    /// Returns the module's direct imports in authored declaration order.
    pub const fn imports(&self) -> &'static [CauldronImportDescriptor] {
        self.imports
    }

    /// Returns the module declaration's source location.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }
}
