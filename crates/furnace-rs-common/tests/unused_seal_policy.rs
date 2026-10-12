//! Linked policies become requirements only through a selected controller seal.
#![cfg(all(feature = "http", feature = "jwt"))]
#![allow(missing_docs)]
use furnace_rs_common::{PassportPrincipal, SealRegistration, Sealable, controller, guard};
#[allow(dead_code)]
#[derive(PassportPrincipal)]
struct Principal {
    subject: String,
}
#[guard(strategy = "never_registered", principal = Principal)]
struct UnusedPolicy;
#[controller]
struct Controller;
impl Sealable for Controller {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}
#[controller(route = "/")]
impl Controller {
    #[get]
    fn index(&self) {}
}
#[test]
fn unused_policy_does_not_require_a_strategy_or_jwt_default() {
    let _ = std::any::TypeId::of::<UnusedPolicy>();
    let analysis = furnace_rs_common::core::Furnace::builder().analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
}
