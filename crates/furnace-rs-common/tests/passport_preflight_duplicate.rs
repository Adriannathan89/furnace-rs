//! Duplicate managed Passport strategies fail during analysis.

#![cfg(all(feature = "http", feature = "jwt"))]

use std::sync::atomic::{AtomicUsize, Ordering};

use furnace_rs_common::{
    FURNACE130, JwtClaims, JwtTokenKind, PassportContext, PassportPrincipal, PassportResult,
    PassportStrategy,
    core::{Furnace, LifecycleFuture, LifecycleHook},
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

#[furnace_rs_core::burner]
struct FirstStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for FirstStrategy {
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

#[furnace_rs_core::burner]
struct SecondStrategy;

#[furnace_rs_common::passport_strategy(name = "jwt")]
impl PassportStrategy for SecondStrategy {
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

static ORDINARY_CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
static LIFECYCLE_STARTS: AtomicUsize = AtomicUsize::new(0);

struct OrdinaryProvider;

#[furnace_rs_core::element]
fn ordinary_provider() -> OrdinaryProvider {
    ORDINARY_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
    OrdinaryProvider
}

struct CountingHook;

impl LifecycleHook for CountingHook {
    fn name(&self) -> &str {
        "duplicate-preflight-counter"
    }

    fn start<'a>(&'a self, _: &'a furnace_rs_core::ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async move {
            LIFECYCLE_STARTS.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }

    fn stop<'a>(&'a self, _: &'a furnace_rs_core::ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

#[tokio::test]
async fn duplicate_strategy_fails_before_construction_or_lifecycle() {
    ORDINARY_CONSTRUCTIONS.store(0, Ordering::SeqCst);
    LIFECYCLE_STARTS.store(0, Ordering::SeqCst);
    let mut builder = Furnace::builder();
    builder.lifecycle_hook(CountingHook);

    let analysis = builder.analyze();
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE130);
    assert!(
        analysis.diagnostics()[0]
            .to_string()
            .contains("duplicate_strategy")
    );

    let error = match builder.build().await {
        Ok(_) => panic!("duplicate strategies must fail before construction"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE130);
    assert_eq!(ORDINARY_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
    assert_eq!(LIFECYCLE_STARTS.load(Ordering::SeqCst), 0);
}
