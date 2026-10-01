//! Explicit furnace composition without constructing application dependencies.
use crate::SourceLocation;
use std::{any::TypeId, marker::PhantomData};

/// Describes the members and imports of one application furnace.
pub trait Furnace: Send + Sync + Sized + 'static {
    /// Records static topology without constructing providers or performing I/O.
    fn register(self) -> FurnaceRegistration<Self>;
}

/// A fluent declaration of a furnace's local dependencies and public interface.
pub struct FurnaceRegistration<M: Furnace> {
    definition: FurnaceDefinition,
    marker: PhantomData<fn() -> M>,
}

/// Erased registration metadata consumed by graph analysis.
#[doc(hidden)]
#[derive(Default)]
pub struct FurnaceDefinition {
    pub(crate) members: Vec<FurnaceMember>,
    pub(crate) imports: Vec<FurnaceImport>,
    pub(crate) exports: Vec<FurnaceMember>,
    pub(crate) global: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct FurnaceMember {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) controller: bool,
    pub(crate) location: SourceLocation,
}

pub(crate) struct FurnaceImport {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) location: SourceLocation,
}

#[track_caller]
fn location() -> SourceLocation {
    let caller = std::panic::Location::caller();
    SourceLocation::new(caller.file(), caller.line(), caller.column())
}

impl<M: Furnace> FurnaceRegistration<M> {
    /// Starts an empty declaration. The unit value is not retained.
    pub fn new(_module: M) -> Self {
        Self {
            definition: FurnaceDefinition::default(),
            marker: PhantomData,
        }
    }

    /// Registers the output type of a burner, storage, or element factory.
    #[track_caller]
    pub fn provide<T: Send + Sync + 'static>(mut self) -> Self {
        self.definition.members.push(FurnaceMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: false,
            location: location(),
        });
        self
    }

    /// Registers a controller and its associated route metadata.
    #[track_caller]
    pub fn controller<T: Send + Sync + 'static>(mut self) -> Self {
        self.definition.members.push(FurnaceMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: true,
            location: location(),
        });
        self
    }

    /// Connects another furnace without evaluating its registration here.
    #[track_caller]
    pub fn import<I: Furnace>(mut self, _module: I) -> Self {
        self.definition.imports.push(FurnaceImport {
            type_id: TypeId::of::<I>(),
            type_name: std::any::type_name::<I>(),
            location: location(),
        });
        self
    }

    /// Publishes a local provider to direct importers or global consumers.
    #[track_caller]
    pub fn export<T: Send + Sync + 'static>(mut self) -> Self {
        self.definition.exports.push(FurnaceMember {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            controller: false,
            location: location(),
        });
        self
    }

    /// Makes explicit exports available throughout the reachable application.
    pub fn global(mut self) -> Self {
        self.definition.global = true;
        self
    }

    /// Erases the authored furnace type for the declaration catalog.
    #[doc(hidden)]
    pub fn into_definition(self) -> FurnaceDefinition {
        self.definition
    }
}
