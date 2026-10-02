//! Facade-level proof of same-named typed route dispatch.

use furnace_rs::common::axum::body::{Body, to_bytes};
use furnace_rs::common::axum::http::{Request, StatusCode};
use furnace_rs::common::build_router;
use furnace_rs::core::Furnace;
use tower::ServiceExt;

#[furnace_rs::controller]
struct LookupController;

impl ::furnace_rs::Sealable for LookupController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl LookupController {
    #[furnace_rs::get("/alpha")]
    async fn alpha(&self) -> &'static str {
        "alpha"
    }
    #[furnace_rs::get("/beta")]
    async fn beta(&self) -> &'static str {
        "beta"
    }
}

#[tokio::test]
async fn facade_builds_a_router_with_typed_inherent_dispatch() {
    let application = Furnace::builder().build().await.unwrap();
    let router = build_router(&application).unwrap();

    let alpha = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/alpha")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(alpha.status(), StatusCode::OK);
    let alpha_body = to_bytes(alpha.into_body(), usize::MAX).await.unwrap();
    assert_eq!(alpha_body.as_ref(), b"alpha");

    let beta = router
        .oneshot(Request::builder().uri("/beta").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(beta.status(), StatusCode::OK);
    let beta_body = to_bytes(beta.into_body(), usize::MAX).await.unwrap();
    assert_eq!(beta_body.as_ref(), b"beta");
}
