//! Static controller protection declarations without constructing values.

use std::marker::PhantomData;

#[cfg(feature = "jwt")]
use crate::{GuardDescriptor, GuardPolicy};
#[cfg(feature = "jwt")]
use furnace_rs_core::SourceLocation;
#[cfg(feature = "jwt")]
use std::any::{TypeId, type_name};

/// Declares one controller's protection independently of its constructed state.
///
/// Implement this trait to protect a controller. Controllers without it are
/// public. Endpoints marked `#[seal(skip)]` bypass the declared policy.
pub trait Sealable: Send + Sync + Sized + 'static {
    /// Returns an empty declaration for public endpoints or one typed policy.
    fn seals() -> SealRegistration<Self>;
}

/// Records a controller's ordered policy declarations without evaluating them.
pub struct SealRegistration<C> {
    definition: SealDefinition,
    marker: PhantomData<fn() -> C>,
}

impl<C> SealRegistration<C> {
    /// Starts an empty declaration, making the controller public.
    pub fn new() -> Self {
        Self {
            definition: SealDefinition::default(),
            marker: PhantomData,
        }
    }

    /// Records one static guard policy. Startup rejects multiple declarations.
    #[cfg(feature = "jwt")]
    #[track_caller]
    pub fn seal<G: GuardPolicy>(mut self) -> Self {
        let caller = std::panic::Location::caller();
        self.definition.entries.push(SealEntry {
            guard_type_id: TypeId::of::<G>(),
            guard_type_name: type_name::<G>(),
            location: SourceLocation::new(caller.file(), caller.line(), caller.column()),
            descriptor: G::descriptor,
        });
        self
    }

    /// Erases the controller marker for static startup analysis.
    #[doc(hidden)]
    pub fn into_definition(self) -> SealDefinition {
        self.definition
    }
}

impl<C> Default for SealRegistration<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Concrete-controller probe used by generated metadata callbacks.
#[doc(hidden)]
pub struct SealProbe<C>(PhantomData<fn() -> C>);

impl<C> SealProbe<C> {
    /// Creates a probe without constructing the controller.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<C> Default for SealProbe<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves an explicit seal or falls back to a public declaration.
#[doc(hidden)]
pub trait OptionalSeal {
    /// Returns static protection metadata for the concrete controller.
    fn optional_seal(self) -> SealDefinition;
}

impl<C> OptionalSeal for &SealProbe<C> {
    fn optional_seal(self) -> SealDefinition {
        SealDefinition::default()
    }
}

impl<C: Sealable> OptionalSeal for &&SealProbe<C> {
    fn optional_seal(self) -> SealDefinition {
        C::seals().into_definition()
    }
}

/// Erased protection metadata consumed by selected-controller analysis.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct SealDefinition {
    #[cfg(feature = "jwt")]
    entries: Vec<SealEntry>,
}

impl SealDefinition {
    /// Returns policy declarations in their authored order.
    #[cfg(feature = "jwt")]
    pub fn entries(&self) -> &[SealEntry] {
        &self.entries
    }
}

/// A policy type and its declaration location, with a deferred descriptor lookup.
#[cfg(feature = "jwt")]
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct SealEntry {
    guard_type_id: TypeId,
    guard_type_name: &'static str,
    location: SourceLocation,
    descriptor: fn() -> &'static GuardDescriptor,
}

#[cfg(feature = "jwt")]
impl SealEntry {
    /// Returns the declared guard policy's runtime identity.
    pub const fn guard_type_id(&self) -> TypeId {
        self.guard_type_id
    }
    /// Returns the declared guard policy's stable Rust name.
    pub const fn guard_type_name(&self) -> &'static str {
        self.guard_type_name
    }
    /// Returns the location of the authored seal call.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }
    /// Looks up the static descriptor when selected analysis needs the policy.
    pub fn descriptor(&self) -> &'static GuardDescriptor {
        (self.descriptor)()
    }
}
