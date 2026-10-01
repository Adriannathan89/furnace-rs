//! Static seal registration records policy types without evaluating factories.
#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]

use furnace_rs_common::core::{SourceLocation, element};
use furnace_rs_common::{
    GuardDescriptor, GuardPolicy, PassportPrincipal, SealRegistration, Sealable, TokenSource, guard,
};
use std::{
    any::TypeId,
    sync::atomic::{AtomicUsize, Ordering},
};

static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
static DESCRIPTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

#[element]
fn sentinel() -> u64 {
    CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
    42
}

#[derive(PassportPrincipal)]
struct Principal {
    subject: String,
}
fn allowed(principal: &Principal) -> bool {
    !principal.subject.is_empty()
}

#[guard(strategy = "custom", principal = Principal, roles(all = ["admin"]), permissions(any = ["read"]), predicate = allowed)]
struct Policy;

struct CountedPolicy;
impl GuardPolicy for CountedPolicy {
    fn descriptor() -> &'static GuardDescriptor {
        DESCRIPTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        static DESCRIPTOR: GuardDescriptor = GuardDescriptor::new(
            "CountedPolicy",
            "seal",
            "custom",
            None,
            None,
            TokenSource::Bearer,
            None,
            None,
            &[],
            SourceLocation::new(file!(), line!(), column!()),
            None,
        );
        &DESCRIPTOR
    }
}

struct Controller;
impl Sealable for Controller {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}

#[test]
fn seal_recording_does_not_construct_values() {
    let empty = Controller::seals().into_definition();
    let one = Controller::seals().seal::<Policy>().into_definition();
    let two = Controller::seals()
        .seal::<Policy>()
        .seal::<CountedPolicy>()
        .into_definition();
    assert_eq!(empty.entries().len(), 0);
    assert!(furnace_rs_common::core::Catalog::provider_for::<Policy>().is_err());
    assert_eq!(one.entries().len(), 1);
    assert_eq!(two.entries().len(), 2);
    assert_eq!(DESCRIPTOR_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 0);
    assert_eq!(one.entries()[0].guard_type_id(), TypeId::of::<Policy>());
    assert!(one.entries()[0].guard_type_name().ends_with("::Policy"));
    assert!(
        one.entries()[0]
            .location()
            .file
            .ends_with("seal_registration.rs")
    );
    assert!(one.entries()[0].location().line > 0);
    assert_eq!(one.entries()[0].descriptor().strategy(), "custom");
    assert_eq!(
        two.entries()[1].guard_type_id(),
        TypeId::of::<CountedPolicy>()
    );
    assert_eq!(two.entries()[1].descriptor().strategy(), "custom");
    assert_eq!(DESCRIPTOR_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 0);
}
