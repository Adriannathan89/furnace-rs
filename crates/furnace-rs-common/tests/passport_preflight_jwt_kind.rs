//! The reserved `jwt` strategy name accepts access tokens only.

#![cfg(all(feature = "http", feature = "jwt"))]

use furnace_rs_common::{
    FURNACE130, GuardCatalog, JwtClaims, JwtTokenKind, PassportContext, PassportPrincipal,
    PassportResult, PassportStrategy, PassportStrategyCatalog,
};

#[derive(serde::Deserialize)]
struct AccessClaims;

struct AccessPrincipal;

impl PassportPrincipal for AccessPrincipal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

#[furnace_rs_core::burner]
struct IncorrectAccessStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for IncorrectAccessStrategy {
    type Claims = AccessClaims;
    type Principal = AccessPrincipal;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Refresh;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        _claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(AccessPrincipal)
    }
}

#[furnace_rs_common::routes]
#[furnace_rs_common::guard(strategy = "jwt", principal = AccessPrincipal)]
#[allow(dead_code)]
trait AccessRoutes {
    #[furnace_rs_common::get("/")]
    async fn profile(&self);
}

#[test]
fn preflight_rejects_a_refresh_strategy_registered_as_jwt() {
    let guards = GuardCatalog::guards();
    let error = PassportStrategyCatalog::preflight(&guards).unwrap_err();

    assert_eq!(error.code(), FURNACE130);
    assert!(error.to_string().contains("reserved_strategy_token_kind"));
    assert!(error.to_string().contains("jwt"));
}
