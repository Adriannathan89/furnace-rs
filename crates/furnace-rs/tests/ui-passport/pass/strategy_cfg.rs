use furnace_rs::common::*;

// These types deliberately do not exist when the implementation is disabled.
#[passport_strategy(name = "cfg-disabled")]
#[cfg(any())]
impl PassportStrategy for DisabledStrategy {
    type Claims = DisabledClaims;
    type Principal = DisabledPrincipal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _: &PassportContext<'_>,
        _: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(DisabledPrincipal)
    }
}

#[passport_strategy(name = "cfg-attr-disabled")]
#[cfg_attr(all(), cfg(any()))]
impl PassportStrategy for AttributeDisabledStrategy {
    type Claims = AttributeDisabledClaims;
    type Principal = AttributeDisabledPrincipal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _: &PassportContext<'_>,
        _: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(AttributeDisabledPrincipal)
    }
}

#[derive(serde::Deserialize)]
struct EnabledClaims;
struct EnabledPrincipal;
impl PassportPrincipal for EnabledPrincipal {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}
struct EnabledStrategy;
#[passport_strategy(name = "cfg-enabled")]
#[cfg(all())]
#[cfg_attr(any(), cfg(any()))]
impl PassportStrategy for EnabledStrategy {
    type Claims = EnabledClaims;
    type Principal = EnabledPrincipal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _: &PassportContext<'_>,
        _: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        Ok(EnabledPrincipal)
    }
}

fn main() {
    let strategies = PassportStrategyCatalog::strategies();
    assert_eq!(strategies.len(), 1);
    assert_eq!(strategies[0].name(), "cfg-enabled");
}
