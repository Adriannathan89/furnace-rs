//! Transport-independent guard selection and occurrence identities.
use super::GuardDescriptor;
use std::any::TypeId;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Transport-neutral identity of one protected operation.
#[doc(hidden)]
pub struct GuardOccurrence {
    /// Owning implementation identity.
    pub provider: TypeId,
    /// Canonical operation identity.
    pub operation: String,
}

#[derive(Clone)]
/// Selected policy and its application visibility context.
#[doc(hidden)]
pub struct ScopedGuard {
    /// Static policy.
    pub guard: &'static GuardDescriptor,
    /// Selected operation.
    pub occurrence: Option<GuardOccurrence>,
    /// Cauldron determining strategy visibility.
    pub context_cauldron: Option<TypeId>,
}

impl ScopedGuard {
    /// Returns this operation identity.
    pub fn occurrence(&self) -> Option<GuardOccurrence> {
        self.occurrence.clone()
    }
    /// Returns the selected policy.
    pub const fn guard(&self) -> &'static GuardDescriptor {
        self.guard
    }
    /// Returns the selected visibility context.
    pub const fn context_cauldron(&self) -> Option<TypeId> {
        self.context_cauldron
    }
}

/// Neutral integration callback contributing selected policies to shared JWT analysis.
#[doc(hidden)]
pub struct PassportScopeDescriptor {
    /// Selects policies using analysis metadata only, without constructors or I/O.
    pub select: for<'a> fn(
        &furnace_rs_core::__private::AutoConfigurationContext<'a>,
    ) -> furnace_rs_core::Result<Vec<ScopedGuard>>,
}
furnace_rs_core::__private::inventory::collect!(PassportScopeDescriptor);
