//! Explicit cauldron composition without constructing application dependencies.
use crate::SourceLocation;
use std::any::TypeId;

/// Framework-neutral membership implemented by optional transport integrations.
#[doc(hidden)]
pub trait TransportProvider: crate::Injector {
    /// Stable identifier of the integration owning this service.
    const TRANSPORT: &'static str;
}

/// Metadata builder used only by declarative macro expansions.
#[doc(hidden)]
#[derive(Default)]
pub struct CauldronDefinitionBuilder {
    definition: CauldronDefinition,
}

/// Erased registration metadata consumed by graph analysis.
#[doc(hidden)]
#[derive(Default)]
pub struct CauldronDefinition {
    pub(crate) members: Vec<CauldronMember>,
    pub(crate) imports: Vec<CauldronImport>,
    pub(crate) exports: Vec<CauldronMember>,
    pub(crate) global: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct CauldronMember {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) controller: bool,
    pub(crate) transport: Option<&'static str>,
    pub(crate) location: SourceLocation,
    pub(crate) descriptor: Option<&'static crate::ProviderDescriptor>,
}

pub(crate) struct CauldronImport {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) location: SourceLocation,
}

#[track_caller]
/// Captures the declaration call site for stable registration diagnostics.
fn location() -> SourceLocation {
    let caller = std::panic::Location::caller();
    SourceLocation::new(caller.file(), caller.line(), caller.column())
}

impl CauldronDefinitionBuilder {
    /// Starts an empty metadata definition.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a managed provider or a plain service implementing `Injector`.
    #[track_caller]
    pub fn provide<T: crate::Injector>(mut self) -> Self {
        self.definition.members.push(CauldronMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: false,
            transport: None,
            location: location(),
            descriptor: Some(T::descriptor()),
        });
        self
    }

    /// Registers an explicitly selected transport service as a managed provider.
    #[doc(hidden)]
    #[track_caller]
    pub fn transport_provider<T: TransportProvider>(self) -> Self {
        let mut registration = self.provide::<T>();
        registration
            .definition
            .members
            .last_mut()
            .expect("a provider was just registered")
            .transport = Some(T::TRANSPORT);
        registration
    }

    /// Registers an output constructed by the explicitly selected injector.
    ///
    /// Use this for trait bindings or third-party native types. The constructor
    /// receives its declared dependencies during startup, never during registration.
    #[track_caller]
    pub fn provide_with<T, I>(mut self) -> Self
    where
        T: Send + Sync + 'static,
        I: crate::Injector<T>,
    {
        self.definition.members.push(CauldronMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: false,
            transport: None,
            location: location(),
            descriptor: Some(I::descriptor()),
        });
        self
    }

    /// Registers a controller and its associated route metadata.
    #[track_caller]
    pub fn controller<T: Send + Sync + 'static>(mut self) -> Self {
        self.definition.members.push(CauldronMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: true,
            transport: None,
            location: location(),
            descriptor: None,
        });
        self
    }

    /// Connects another cauldron without evaluating its registration here.
    #[track_caller]
    pub fn import<I: 'static>(mut self) -> Self {
        self.definition.imports.push(CauldronImport {
            type_id: TypeId::of::<I>(),
            type_name: std::any::type_name::<I>(),
            location: location(),
        });
        self
    }

    /// Publishes a local provider to direct importers or global consumers.
    #[track_caller]
    pub fn export<T: Send + Sync + 'static>(mut self) -> Self {
        self.definition.exports.push(CauldronMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: false,
            transport: None,
            location: location(),
            descriptor: None,
        });
        self
    }

    /// Makes explicit exports available throughout the reachable application.
    pub fn global(mut self) -> Self {
        self.definition.global = true;
        self
    }

    /// Finishes the metadata consumed by the declaration catalog.
    #[doc(hidden)]
    pub fn into_definition(self) -> CauldronDefinition {
        self.definition
    }
}
