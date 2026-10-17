//! Backward-compatible ownership boundary for Passport guard APIs.
pub use super::policy::{
    BuiltinGuardAdapter, GuardCatalog, GuardDescriptor, GuardPolicy, GuardPredicate,
    GuardPredicateAdapter, PolicyClause, PolicyMode, TokenSource,
};
