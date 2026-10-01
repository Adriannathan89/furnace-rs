//! Rooted Passport preflight respects explicit declaration and JWT ownership.
#![cfg(all(feature = "http", feature = "jwt"))]

use mads_common::{
    ClaimsPrincipal, JwtClaims, JwtService, JwtTokenKind, PassportContext, PassportPrincipal,
    PassportResult, PassportStrategy,
};
use mads_core::{Config, Furnace, FurnaceRegistration, MADS009, Mads};

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

#[mads_core::burner]
struct IncorrectStrategy;
#[mads_common::passport_strategy(name = "jwt")]
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

#[mads_common::routes]
#[mads_common::guard(strategy = "jwt", principal = ClaimsPrincipal<Claims>)]
trait Routes {
    #[mads_common::get("/")]
    async fn index(&self) -> &'static str;
}
#[mads_common::controller(routes = [Routes])]
struct Controller;
impl Routes for Controller {
    async fn index(&self) -> &'static str {
        "protected"
    }
}

#[mads_core::furnace]
struct LocalJwt;
impl Furnace for LocalJwt {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<JwtService>().controller::<Controller>()
    }
}
#[mads_core::furnace]
struct PrivateJwt;
impl Furnace for PrivateJwt {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<JwtService>()
    }
}
#[mads_core::furnace]
struct Http;
impl Furnace for Http {
    fn register(self) -> FurnaceRegistration<Self> {
        self.controller::<Controller>()
    }
}
#[mads_core::furnace]
struct PrivateApp;
impl Furnace for PrivateApp {
    fn register(self) -> FurnaceRegistration<Self> {
        self.import(PrivateJwt).import(Http)
    }
}
#[mads_core::furnace]
struct InvalidRegistered;
impl Furnace for InvalidRegistered {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<IncorrectStrategy>()
    }
}

fn config() -> Config {
    mads_core::ConfigBuilder::new()
        .source(mads_core::MapSource::new(
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
    let mut builder = Mads::builder_with_config(config);
    builder.root::<LocalJwt>().unwrap();
    builder.provide(jwt).unwrap();
    let analysis = builder.analyze();
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics());
    let app = builder.build().await.unwrap();
    let _router = mads_common::build_router(&app).unwrap();
}

#[test]
fn guards_cannot_consume_a_private_foreign_jwt_override() {
    let config = config();
    let jwt = JwtService::from_config(&config).unwrap();
    let mut builder = Mads::builder_with_config(config);
    builder.root::<PrivateApp>().unwrap();
    builder.provide(jwt).unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == MADS009),
        "{:?}",
        analysis.diagnostics()
    );
}

#[test]
fn registered_invalid_strategy_metadata_is_still_rejected() {
    let mut builder = Mads::builder_with_config(config());
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
