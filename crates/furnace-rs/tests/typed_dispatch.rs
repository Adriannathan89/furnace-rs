//! Facade-level proof of same-named typed route dispatch.

use furnace_rs::common::axum::body::{Body, to_bytes};
use furnace_rs::common::axum::http::{Request, StatusCode};
use furnace_rs::common::build_router;
use furnace_rs::core::Furnace;
use tower::ServiceExt;

#[furnace_rs::routes]
trait AlphaRoutes {
    #[furnace_rs::get("/alpha")]
    async fn lookup(&self) -> &'static str;
}

#[furnace_rs::routes]
trait BetaRoutes {
    #[furnace_rs::get("/beta")]
    async fn lookup(&self) -> &'static str;
}

#[furnace_rs::controller(routes = [AlphaRoutes, BetaRoutes])]
struct LookupController;

impl AlphaRoutes for LookupController {
    async fn lookup(&self) -> &'static str {
        "alpha"
    }
}

impl BetaRoutes for LookupController {
    async fn lookup(&self) -> &'static str {
        "beta"
    }
}

#[tokio::test]
async fn facade_builds_a_router_with_typed_trait_dispatch() {
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
