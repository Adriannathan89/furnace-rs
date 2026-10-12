//! Effective Passport guard metadata emitted by route contracts.

#![cfg(all(feature = "http", feature = "jwt"))]

use std::any::TypeId;

use furnace_rs_common::{
    FURNACE131, GuardCatalog, GuardDescriptor, PassportPrincipal, PolicyMode, TokenSource,
    core::SourceLocation,
};

struct UserPrincipal {
    user_id: u64,
}

impl PassportPrincipal for UserPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

fn owns_profile(principal: &UserPrincipal) -> bool {
    principal.user_id == 7
}

#[furnace_rs_common::controller]
struct UserRoutesController;

#[furnace_rs_common::guard(permissions (all = ["profile:read"]), predicate = owns_profile, principal = UserPrincipal, roles (any = ["user" , "admin"]), source = bearer, strategy = "jwt")]
struct UserRoutesControllerGuard;

impl ::furnace_rs_common::Sealable for UserRoutesController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<UserRoutesControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl UserRoutesController {
    #[furnace_rs_common::get("/users/profile")]

    async fn profile(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[furnace_rs_common::controller]
struct UserRoutesController1;

impl ::furnace_rs_common::Sealable for UserRoutesController1 {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        ::furnace_rs_common::SealRegistration::new()
    }
}

#[furnace_rs_common::controller]
impl UserRoutesController1 {
    #[furnace_rs_common::post("/users/login")]

    async fn login(&self) {
        unreachable!("metadata-only endpoint")
    }
}

const MISSING_PRINCIPAL: GuardDescriptor = GuardDescriptor::new(
    "ManualRoutes",
    "profile",
    "jwt",
    None,
    None,
    TokenSource::Bearer,
    None,
    None,
    &[],
    SourceLocation::new("tests/passport_guard_catalog.rs", 1, 1),
    None,
);

#[test]
fn static_policy_records_all_rules_and_public_controller_has_no_seal() {
    let profile = <UserRoutesControllerGuard as furnace_rs_common::GuardPolicy>::descriptor();
    assert_eq!(profile.strategy(), "jwt");
    assert_eq!(
        profile.principal_type_id(),
        Some(TypeId::of::<UserPrincipal>())
    );
    assert_eq!(
        profile.principal_type_name(),
        Some(std::any::type_name::<UserPrincipal>())
    );
    assert_eq!(profile.source(), TokenSource::Bearer);

    let roles = profile.roles().expect("inherited roles");
    assert_eq!(roles.mode(), PolicyMode::Any);
    assert_eq!(roles.values(), ["user", "admin"]);

    let permissions = profile.permissions().expect("method permissions");
    assert_eq!(permissions.mode(), PolicyMode::All);
    assert_eq!(permissions.values(), ["profile:read"]);
    assert_eq!(profile.predicates().len(), 1);
    assert!(GuardCatalog::validate_descriptors(&[profile]).is_ok());
    use furnace_rs_common::Sealable;
    let definition = UserRoutesController::seals().into_definition();
    assert!(std::ptr::eq(definition.entries()[0].descriptor(), profile));
    assert!(
        UserRoutesController1::seals()
            .into_definition()
            .entries()
            .is_empty()
    );
}

#[test]
fn manual_guard_metadata_without_a_principal_fails_closed() {
    let error = GuardCatalog::validate_descriptors(&[&MISSING_PRINCIPAL])
        .expect_err("manual descriptors must be fully typed");
    assert_eq!(error.code(), FURNACE131);
}
