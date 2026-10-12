//! Reserved Passport strategy names enforce their JWT token profiles.

#![cfg(all(feature = "http", feature = "jwt"))]

use furnace_rs_common::{
    FURNACE130, JwtClaims, JwtTokenKind, PassportContext, PassportPrincipal, PassportResult,
    PassportStrategy, PassportStrategyCatalog,
};

#[derive(serde::Deserialize)]
struct RefreshClaims;

struct RefreshPrincipal;

impl PassportPrincipal for RefreshPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

#[furnace_rs_core::burner]
struct IncorrectRefreshStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt-refresh")]
impl PassportStrategy for IncorrectRefreshStrategy {
    type Claims = RefreshClaims;
    type Principal = RefreshPrincipal;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        _claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(RefreshPrincipal)
    }
}

#[furnace_rs_common::controller]
struct RefreshRoutesController;

#[furnace_rs_common::guard(principal = RefreshPrincipal, strategy = "jwt-refresh")]
struct RefreshRoutesControllerGuard;

impl ::furnace_rs_common::Sealable for RefreshRoutesController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<RefreshRoutesControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl RefreshRoutesController {
    #[furnace_rs_common::post("/")]

    async fn refresh(&self) {
        unreachable!("metadata-only endpoint")
    }
}

#[test]
fn preflight_rejects_an_access_strategy_registered_as_jwt_refresh() {
    let error = PassportStrategyCatalog::preflight(&[
        <RefreshRoutesControllerGuard as furnace_rs_common::GuardPolicy>::descriptor(),
    ])
    .unwrap_err();

    assert_eq!(error.code(), FURNACE130);
    assert!(error.to_string().contains("reserved_strategy_token_kind"));
    assert!(error.to_string().contains("jwt-refresh"));
}
