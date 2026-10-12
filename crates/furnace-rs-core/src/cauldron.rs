//! Explicit cauldron composition without constructing application dependencies.
use crate::SourceLocation;
use std::{any::TypeId, marker::PhantomData};

/// Describes the members and imports of one application cauldron.
pub trait Cauldron: Send + Sync + Sized + 'static {
    /// Records static topology without constructing providers or performing I/O.
    fn register(self) -> CauldronRegistration<Self>;
}

/// A fluent declaration of a cauldron's local dependencies and public interface.
pub struct CauldronRegistration<M: Cauldron> {
    definition: CauldronDefinition,
    marker: PhantomData<fn() -> M>,
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
    pub(crate) location: SourceLocation,
    pub(crate) descriptor: Option<&'static crate::ProviderDescriptor>,
}

pub(crate) struct CauldronImport {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) location: SourceLocation,
}

#[track_caller]
fn location() -> SourceLocation {
    let caller = std::panic::Location::caller();
    SourceLocation::new(caller.file(), caller.line(), caller.column())
}

impl<M: Cauldron> CauldronRegistration<M> {
    /// Starts an empty declaration. The unit value is not retained.
    pub fn new(_module: M) -> Self {
        Self {
            definition: CauldronDefinition::default(),
            marker: PhantomData,
        }
    }

    /// Registers a managed provider or a plain service implementing `Injector`.
    #[track_caller]
    pub fn provide<T: crate::Injector>(mut self) -> Self {
        self.definition.members.push(CauldronMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: false,
            location: location(),
            descriptor: Some(T::descriptor()),
        });
        self
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
            location: location(),
            descriptor: None,
        });
        self
    }

    /// Connects another cauldron without evaluating its registration here.
    #[track_caller]
    pub fn import<I: Cauldron>(mut self, _module: I) -> Self {
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

    /// Erases the authored cauldron type for the declaration catalog.
    #[doc(hidden)]
    pub fn into_definition(self) -> CauldronDefinition {
        self.definition
    }
}
