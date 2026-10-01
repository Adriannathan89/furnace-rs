//! Passport strategies must be registered managed providers.

#![cfg(all(feature = "http", feature = "jwt"))]

use furnace_rs_common::{
    FURNACE130, JwtClaims, JwtTokenKind, PassportContext, PassportPrincipal, PassportResult,
    PassportStrategy, core::Furnace,
};

#[derive(serde::Deserialize)]
struct Claims;

struct Principal;

impl PassportPrincipal for Principal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

struct UnmanagedStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for UnmanagedStrategy {
    type Claims = Claims;
    type Principal = Principal;

    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        _claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(Principal)
    }
}

#[furnace_rs_common::routes]
#[furnace_rs_common::guard(strategy = "jwt", principal = Principal)]
#[allow(dead_code)]
trait ProtectedRoutes {
    #[furnace_rs_common::get("/")]
    async fn profile(&self);
}

#[test]
fn unmanaged_strategy_is_rejected_during_analysis() {
    let analysis = Furnace::builder().analyze();

    assert!(!analysis.is_valid());
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE130);
    assert!(
        analysis.diagnostics()[0]
            .to_string()
            .contains("unmanaged_strategy")
    );
    assert!(
        analysis.diagnostics()[0]
            .to_string()
            .contains("UnmanagedStrategy")
    );
}
