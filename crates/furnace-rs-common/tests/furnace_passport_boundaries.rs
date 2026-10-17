//! Rooted Passport preflight respects explicit declaration and JWT ownership.
#![cfg(all(feature = "http", feature = "jwt"))]

use furnace_rs_common::{
    ClaimsPrincipal, JwtClaims, JwtService, JwtTokenKind, PassportContext, PassportPrincipal,
    PassportResult, PassportStrategy,
};
use furnace_rs_core::{Config, FURNACE009, Furnace};

#[derive(serde::Deserialize)]
struct Claims;
impl PassportPrincipal for Claims {
    fn has_role(&self, _: &str) -> bool {
        false
    }
    fn has_permission(&self, _: &str) -> bool {
        false
    }
}

#[furnace_rs_core::burner]
struct IncorrectStrategy;
#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for IncorrectStrategy {
    type Claims = Claims;
    type Principal = ClaimsPrincipal<Claims>;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Refresh;
    async fn validate(
        &self,
        _: &PassportContext<'_>,
        _: &JwtClaims<Claims>,
    ) -> PassportResult<Self::Principal> {
        unreachable!()
    }
}

#[furnace_rs_common::controller]
struct Controller;

#[furnace_rs_common::guard(principal = ClaimsPrincipal < Claims >, strategy = "jwt")]
struct ControllerGuard;

impl ::furnace_rs_common::Sealable for Controller {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<ControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl Controller {
    #[furnace_rs_common::get("/")]
    async fn index(&self) -> &'static str {
        "protected"
    }
}

#[furnace_rs_core::cauldron(providers = [JwtService], controllers = [Controller])]
struct LocalJwt;

#[furnace_rs_core::cauldron(providers = [JwtService])]
struct PrivateJwt;

#[furnace_rs_core::cauldron(controllers = [Controller])]
struct Http;

#[furnace_rs_core::cauldron(imports = [PrivateJwt, Http])]
struct PrivateApp;

#[furnace_rs_core::cauldron(providers = [IncorrectStrategy])]
struct InvalidRegistered;

fn config() -> Config {
    furnace_rs_core::ConfigBuilder::new()
        .source(furnace_rs_core::MapSource::new(
            "test",
            [("passport.secret", "a-long-enough-test-secret-with-32-bytes")],
        ))
        .build()
        .unwrap()
}

#[tokio::test]
async fn unregistered_strategy_metadata_does_not_poison_a_root() {
    let config = config();
    let jwt = JwtService::from_config(&config).unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<LocalJwt>().unwrap();
    builder.provide(jwt).unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    let app = builder.build().await.unwrap();
    let _router = furnace_rs_common::build_router(&app).unwrap();
}

#[test]
fn guards_cannot_consume_a_private_foreign_jwt_override() {
    let config = config();
    let jwt = JwtService::from_config(&config).unwrap();
    let mut builder = Furnace::builder_with_config(config);
    builder.root::<PrivateApp>().unwrap();
    builder.provide(jwt).unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == FURNACE009),
        "{:?}",
        analysis.diagnostics()
    );
}

#[test]
fn registered_invalid_strategy_metadata_is_still_rejected() {
    let mut builder = Furnace::builder_with_config(config());
    builder.root::<InvalidRegistered>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert!(
        analysis.diagnostics().iter().any(|diagnostic| diagnostic
            .message()
            .contains("reserved_strategy_token_kind")),
        "{:?}",
        analysis.diagnostics()
    );
}
