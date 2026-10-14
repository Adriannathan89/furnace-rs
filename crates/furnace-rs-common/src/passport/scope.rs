//! Transport-independent guard selection and occurrence identities.
use super::GuardDescriptor;
use std::any::TypeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GuardOccurrence {
    pub(crate) provider: TypeId,
    pub(crate) operation: String,
}

#[derive(Clone)]
pub(crate) struct ScopedGuard {
    pub(crate) guard: &'static GuardDescriptor,
    pub(crate) occurrence: Option<GuardOccurrence>,
    pub(crate) context_cauldron: Option<TypeId>,
}

impl ScopedGuard {
    pub(crate) fn occurrence(&self) -> Option<GuardOccurrence> {
        self.occurrence.clone()
    }
    pub(crate) const fn guard(&self) -> &'static GuardDescriptor {
        self.guard
    }
    pub(crate) const fn context_cauldron(&self) -> Option<TypeId> {
        self.context_cauldron
    }
}
