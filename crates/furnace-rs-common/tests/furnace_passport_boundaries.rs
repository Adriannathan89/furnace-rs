//! Rooted Passport preflight respects explicit declaration and JWT ownership.
#![cfg(all(feature = "http", feature = "jwt"))]

use furnace_rs_common::{
    ClaimsPrincipal, JwtClaims, JwtService, JwtTokenKind, PassportContext, PassportPrincipal,
    PassportResult, PassportStrategy,
};
use furnace_rs_core::{Cauldron, CauldronRegistration, Config, FURNACE009, Furnace};

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

#[furnace_rs_common::routes]
#[furnace_rs_common::guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>)]
trait Routes {
    #[furnace_rs_common::get("/")]
    async fn index(&self) -> &'static str;
}
#[furnace_rs_common::controller(routes = [Routes])]
struct Controller;
impl Routes for Controller {
    async fn index(&self) -> &'static str {
        "protected"
    }
}

#[furnace_rs_core::cauldron]
struct LocalJwt;
impl Cauldron for LocalJwt {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<JwtService>().controller::<Controller>()
    }
}
#[furnace_rs_core::cauldron]
struct PrivateJwt;
impl Cauldron for PrivateJwt {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<JwtService>()
    }
}
#[furnace_rs_core::cauldron]
struct Http;
impl Cauldron for Http {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<Controller>()
    }
}
#[furnace_rs_core::cauldron]
struct PrivateApp;
impl Cauldron for PrivateApp {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(PrivateJwt).import(Http)
    }
}
#[furnace_rs_core::cauldron]
struct InvalidRegistered;
impl Cauldron for InvalidRegistered {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<IncorrectStrategy>()
    }
}

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
