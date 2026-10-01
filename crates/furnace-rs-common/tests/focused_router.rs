//! Cauldron-free selected controller routing.
#![cfg(feature = "http")]
#![allow(missing_docs)]

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use furnace_rs_common::{
    ControllerRouteDescriptor, controller,
    core::{FURNACE030, Furnace},
    routes,
};
use std::any::TypeId;
use tower::ServiceExt;

#[routes]
trait SelectedRoutes {
    #[get("/selected")]
    async fn selected(&self) -> &'static str;
}
#[controller(routes = [SelectedRoutes])]
struct Selected;
impl SelectedRoutes for Selected {
    async fn selected(&self) -> &'static str {
        "selected"
    }
}
#[routes]
trait OtherRoutes {
    #[get("/other")]
    async fn other(&self) -> &'static str;
}
#[controller(routes = [OtherRoutes])]
struct Other;
impl OtherRoutes for Other {
    async fn other(&self) -> &'static str {
        panic!("unselected route ran")
    }
}
#[furnace_rs_core::burner]
struct NoRoutes;
#[furnace_rs_core::burner]
struct Invalid;
#[furnace_rs_core::burner]
struct Ambiguous;
fn invalid_id() -> TypeId {
    TypeId::of::<Invalid>()
}
fn ambiguous_id() -> TypeId {
    TypeId::of::<Ambiguous>()
}
furnace_rs_core::__private::inventory::submit! {
    ControllerRouteDescriptor::new("Invalid", invalid_id, &[])
}
furnace_rs_core::__private::inventory::submit! {
    ControllerRouteDescriptor::new("AmbiguousOne", ambiguous_id, &[])
}
furnace_rs_core::__private::inventory::submit! {
    ControllerRouteDescriptor::new("AmbiguousTwo", ambiguous_id, &[])
}

#[cfg(feature = "jwt")]
mod unused_guard {
    pub struct UnusedPrincipal;
    impl furnace_rs_common::PassportPrincipal for UnusedPrincipal {
        fn has_role(&self, _: &str) -> bool {
            false
        }
        fn has_permission(&self, _: &str) -> bool {
            false
        }
    }
    #[furnace_rs_common::routes]
    #[furnace_rs_common::guard(strategy = "missing", principal = UnusedPrincipal)]
    pub trait Guarded {
        #[get("/unused-guard")]
        async fn guarded(&self) -> &'static str;
    }
    #[furnace_rs_common::controller(routes = [Guarded])]
    pub struct GuardedController;
    impl Guarded for GuardedController {
        async fn guarded(&self) -> &'static str {
            "unused"
        }
    }
}

#[tokio::test]
async fn builds_only_selected_controller_routes_and_ignores_invalid_unselected_metadata() {
    let mut builder = Furnace::builder();
    builder.__test_focus::<Selected>().unwrap();
    let app = builder.build().await.unwrap();
    let router = furnace_rs_common::__private::build_test_router_for::<Selected>(&app).unwrap();
    let selected = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/selected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(selected.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(selected.into_body(), usize::MAX).await.unwrap(),
        "selected"
    );
    let other = router
        .oneshot(
            Request::builder()
                .uri("/other")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::NOT_FOUND);
}

async fn error_for<T: Send + Sync + 'static>() -> furnace_rs_core::Error {
    let mut builder = Furnace::builder();
    builder.__test_focus::<T>().unwrap();
    match builder.build().await {
        Ok(app) => furnace_rs_common::__private::build_test_router_for::<T>(&app).unwrap_err(),
        Err(error) => error,
    }
}
#[tokio::test]
async fn rejects_absent_controller_metadata() {
    assert_eq!(error_for::<NoRoutes>().await.code(), FURNACE030);
}
#[tokio::test]
async fn rejects_invalid_selected_metadata() {
    assert_eq!(error_for::<Invalid>().await.code(), FURNACE030);
}
#[tokio::test]
async fn rejects_ambiguous_selected_controller_metadata() {
    let error = error_for::<Ambiguous>().await;
    assert_eq!(error.code(), FURNACE030);
    assert!(error.to_string().contains("controller type identifier"));
}

#[cfg(feature = "jwt")]
mod unrelated_strategy {
    #[derive(serde::Deserialize)]
    pub struct Claims;
    pub struct Strategy;
    #[furnace_rs_common::passport_strategy(name = "unmanaged-unrelated")]
    impl furnace_rs_common::PassportStrategy for Strategy {
        type Claims = Claims;
        type Principal = super::unused_guard::UnusedPrincipal;
        const TOKEN_KIND: furnace_rs_common::JwtTokenKind = furnace_rs_common::JwtTokenKind::Access;
        async fn validate(
            &self,
            _: &furnace_rs_common::PassportContext<'_>,
            _: &furnace_rs_common::JwtClaims<Self::Claims>,
        ) -> furnace_rs_common::PassportResult<Self::Principal> {
            Ok(super::unused_guard::UnusedPrincipal)
        }
    }
}

#[cfg(feature = "jwt")]
#[tokio::test]
async fn selected_guard_still_requires_its_strategy() {
    let error = error_for::<unused_guard::GuardedController>().await;
    assert_eq!(error.code(), furnace_rs_common::FURNACE130);
    assert!(error.to_string().contains("missing"));
}

#[cfg(feature = "jwt")]
#[test]
fn ordinary_rootless_preflight_still_rejects_invalid_catalog() {
    let error = furnace_rs_common::PassportStrategyCatalog::preflight(&[]).unwrap_err();
    assert_eq!(error.code(), furnace_rs_common::FURNACE130);
}
