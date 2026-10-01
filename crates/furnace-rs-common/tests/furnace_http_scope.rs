//! Cauldron membership determines routes independently of declaration namespaces.
#![cfg(feature = "http")]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use furnace_rs_common::core::{Cauldron, CauldronRegistration, Furnace};
use furnace_rs_common::{build_router, controller};
use tower::ServiceExt;
mod contract {}
#[controller]
struct Selected;

impl ::furnace_rs_common::Sealable for Selected {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        ::furnace_rs_common::SealRegistration::new()
    }
}

#[furnace_rs_common::controller]
impl Selected {
    #[get("/selected")]
    async fn selected(&self) -> &'static str {
        "selected"
    }
}

#[controller]
struct Stray;

impl ::furnace_rs_common::Sealable for Stray {
    fn seals() -> ::furnace_rs_common::SealRegistration<Self> {
        ::furnace_rs_common::SealRegistration::new()
    }
}

#[furnace_rs_common::controller]
impl Stray {
    #[get("/stray")]
    async fn stray(&self) -> &'static str {
        "stray"
    }
}

#[furnace_rs_core::cauldron]
struct Root;
impl Cauldron for Root {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<Selected>()
    }
}
#[tokio::test]
async fn only_registered_controllers_install_routes() {
    let mut builder = Furnace::builder();
    builder.root::<Root>().unwrap();
    let app = builder.build().await.unwrap();
    let router = build_router(&app).unwrap();
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/selected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/stray")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[derive(Clone)]
struct Ordinary;
#[furnace_rs_core::element]
fn ordinary() -> Ordinary {
    Ordinary
}
#[furnace_rs_core::cauldron]
struct WrongController;
impl Cauldron for WrongController {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<Ordinary>()
    }
}
#[furnace_rs_core::cauldron]
struct ExportedController;
impl Cauldron for ExportedController {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Selected>().export::<Selected>()
    }
}
#[test]
fn controller_roles_and_exports_are_validated_before_construction() {
    for analysis in [
        {
            let mut b = Furnace::builder();
            b.root::<WrongController>().unwrap();
            b.analyze()
        },
        {
            let mut b = Furnace::builder();
            b.root::<ExportedController>().unwrap();
            b.analyze()
        },
    ] {
        assert!(!analysis.is_valid());
        assert!(
            analysis
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code() == furnace_rs_core::FURNACE008)
        );
    }
}
