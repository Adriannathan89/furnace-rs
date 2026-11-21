//! `#[cache]` and `#[rate_limit]` on controller endpoints, through the facade.
#![allow(missing_docs)]
#![cfg(feature = "extra")]

use furnace_rs::prelude::{MemoryBackend, RedisExtra};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

static CACHED_CALLS: AtomicUsize = AtomicUsize::new(0);

#[furnace_rs::controller]
struct ExtraController;

impl ::furnace_rs::Sealable for ExtraController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller(route = "/x")]
impl ExtraController {
    #[get("/c")]
    #[cache(ttl = 60)]
    async fn cached(&self) -> &'static str {
        CACHED_CALLS.fetch_add(1, SeqCst);
        "cached"
    }

    #[get("/r")]
    #[rate_limit(limit = 1, per = 60)]
    async fn limited(&self) -> &'static str {
        "limited"
    }
}

#[furnace_rs::test]
async fn attributes_apply_cache_and_rate_limit() -> furnace_rs_testing::TestResult<()> {
    test_fixture()
        .provide(RedisExtra::with_backend(MemoryBackend::default()))
        .controller::<ExtraController>()
        .run(|client| async move {
            client
                .get("/x/c")
                .send()
                .await
                .unwrap()
                .assert_text("cached");
            client
                .get("/x/c")
                .send()
                .await
                .unwrap()
                .assert_text("cached");
            assert_eq!(CACHED_CALLS.load(SeqCst), 1);

            client
                .get("/x/r")
                .send()
                .await
                .unwrap()
                .assert_text("limited");
            client
                .get("/x/r")
                .send()
                .await
                .unwrap()
                .assert_status(furnace_rs_testing::http_types::StatusCode::TOO_MANY_REQUESTS);
        })
        .await
}

#[furnace_rs::test]
async fn missing_service_fails_router_build() {
    let result = test_fixture()
        .controller::<ExtraController>()
        .run(|_| async {})
        .await;
    let error = result
        .expect_err("router build must fail without RedisExtra")
        .to_string();
    assert!(error.contains("RedisExtra"), "unexpected error: {error}");
}
