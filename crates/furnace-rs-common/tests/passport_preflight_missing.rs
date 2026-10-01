//! Guards require a matching managed Passport strategy.

#![cfg(all(feature = "http", feature = "jwt"))]

use std::sync::atomic::{AtomicUsize, Ordering};

use furnace_rs_common::{
    FURNACE130, PassportPrincipal,
    core::{Furnace, LifecycleFuture, LifecycleHook},
};

struct Principal;

impl PassportPrincipal for Principal {
    fn has_role(&self, _role: &str) -> bool {
        false
    }

    fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}

#[furnace_rs_common::controller]
struct ProtectedRoutesController;

#[furnace_rs_common::guard(principal = Principal, strategy = "missing")]
struct ProtectedRoutesControllerGuard;

impl ::furnace_rs_common::Sealable for ProtectedRoutesController {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        Self::seal::<ProtectedRoutesControllerGuard>()
    }
}

#[furnace_rs_common::controller]
impl ProtectedRoutesController {
    #[furnace_rs_common::get("/")]

    async fn profile(&self) {
        unreachable!("metadata-only endpoint")
    }
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
        "missing-preflight-counter"
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
async fn missing_strategy_fails_before_construction_or_lifecycle() {
    ORDINARY_CONSTRUCTIONS.store(0, Ordering::SeqCst);
    LIFECYCLE_STARTS.store(0, Ordering::SeqCst);
    let mut builder = Furnace::builder();
    builder.lifecycle_hook(CountingHook);

    let analysis = builder.analyze();
    assert_eq!(analysis.diagnostics()[0].code(), FURNACE130);
    assert!(
        analysis.diagnostics()[0]
            .to_string()
            .contains("missing_strategy")
    );

    let error = match builder.build().await {
        Ok(_) => panic!("missing strategies must fail before construction"),
        Err(error) => error,
    };
    assert_eq!(error.code(), FURNACE130);
    assert_eq!(ORDINARY_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
    assert_eq!(LIFECYCLE_STARTS.load(Ordering::SeqCst), 0);
}
