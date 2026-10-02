//! Controllers are public unless they explicitly implement Sealable.
#![cfg(feature = "http")]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use furnace_rs_common::core::{Cauldron, CauldronRegistration, Furnace, cauldron};
use furnace_rs_common::{build_router, controller};
use tower::ServiceExt;

#[controller]
struct PublicController;
#[controller(route = "/public")]
impl PublicController {
    #[get]
    fn index(&self) -> &'static str {
        "public"
    }
    #[get("/redundant")]
    #[seal(skip)]
    fn redundant_skip(&self) -> &'static str {
        "public"
    }
}
#[cauldron]
struct Root;
impl Cauldron for Root {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<PublicController>()
    }
}
#[tokio::test]
async fn controller_without_sealable_is_public_without_jwt_configuration() {
    let mut builder = Furnace::builder();
    builder.root::<Root>().unwrap();
    let app = builder.build().await.unwrap();
    let response = build_router(&app)
        .unwrap()
        .oneshot(
            Request::builder()
                .uri("/public")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
