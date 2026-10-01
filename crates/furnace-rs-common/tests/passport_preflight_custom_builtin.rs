//! Custom `jwt` strategies override otherwise eligible built-in adapters.

#![cfg(all(feature = "http", feature = "jwt"))]

use furnace_rs_common::{
    ClaimsPrincipal, JwtClaims, JwtTokenKind, PassportContext, PassportPrincipal, PassportResult,
    PassportStrategy, PassportStrategyCatalog,
};

#[derive(serde::Deserialize)]
struct UserClaims;

impl PassportPrincipal for UserClaims {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

#[furnace_rs_core::burner]
struct CustomJwtStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for CustomJwtStrategy {
    type Claims = UserClaims;
    type Principal = ClaimsPrincipal<UserClaims>;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        _claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        unreachable!("preflight never executes a Passport strategy")
    }
}

#[furnace_rs_common::controller]
struct UserRoutesController;

#[furnace_rs_common::guard(principal = ClaimsPrincipal < UserClaims >, strategy = "jwt")]
struct UserRoutesControllerGuard;

impl ::furnace_rs_common::Sealable for UserRoutesController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<UserRoutesControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl UserRoutesController {
    #[furnace_rs_common::get("/profile")]

    async fn profile(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[test]
fn custom_jwt_strategy_overrides_an_eligible_claims_principal_adapter() {
    let guards = [<UserRoutesControllerGuard as furnace_rs_common::GuardPolicy>::descriptor()];
    let preflight = PassportStrategyCatalog::preflight(&guards).unwrap();
    let binding = preflight.bindings().first().unwrap();

    assert!(binding.guard().builtin_adapter().is_some());
    assert_eq!(binding.strategy(), "jwt");
    assert_eq!(binding.token_kind(), JwtTokenKind::Access);
    assert!(!binding.is_builtin());
}
