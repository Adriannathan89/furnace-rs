//! Module-free selected controller routing.
#![cfg(feature = "http")]
#![allow(missing_docs)]

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use mads_common::{
    ControllerRouteDescriptor, controller,
    core::{MADS030, Mads},
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
#[mads_core::service]
struct NoRoutes;
#[mads_core::service]
struct Invalid;
#[mads_core::service]
struct Ambiguous;
fn invalid_id() -> TypeId {
    TypeId::of::<Invalid>()
}
fn ambiguous_id() -> TypeId {
    TypeId::of::<Ambiguous>()
}
mads_core::__private::inventory::submit! {
    ControllerRouteDescriptor::new("Invalid", invalid_id, &[])
}
mads_core::__private::inventory::submit! {
    ControllerRouteDescriptor::new("AmbiguousOne", ambiguous_id, &[])
}
mads_core::__private::inventory::submit! {
    ControllerRouteDescriptor::new("AmbiguousTwo", ambiguous_id, &[])
}

#[cfg(feature = "jwt")]
mod unused_guard {
    pub struct UnusedPrincipal;
    impl mads_common::PassportPrincipal for UnusedPrincipal {
        fn has_role(&self, _: &str) -> bool {
            false
        }
        fn has_permission(&self, _: &str) -> bool {
            false
        }
    }
    #[mads_common::routes]
    #[mads_common::guard(strategy = "missing", principal = UnusedPrincipal)]
    pub trait Guarded {
        #[get("/unused-guard")]
        async fn guarded(&self) -> &'static str;
    }
    #[mads_common::controller(routes = [Guarded])]
    pub struct GuardedController;
    impl Guarded for GuardedController {
        async fn guarded(&self) -> &'static str {
            "unused"
        }
    }
}

#[tokio::test]
async fn builds_only_selected_controller_routes_and_ignores_invalid_unselected_metadata() {
    let mut builder = Mads::builder();
    builder.__test_focus::<Selected>().unwrap();
    let app = builder.build().await.unwrap();
    let router = mads_common::__private::build_test_router_for::<Selected>(&app).unwrap();
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

async fn error_for<T: Send + Sync + 'static>() -> mads_core::Error {
    let mut builder = Mads::builder();
    builder.__test_focus::<T>().unwrap();
    let app = builder.build().await.unwrap();
    mads_common::__private::build_test_router_for::<T>(&app).unwrap_err()
}
#[tokio::test]
async fn rejects_absent_controller_metadata() {
    assert_eq!(error_for::<NoRoutes>().await.code(), MADS030);
}
#[tokio::test]
async fn rejects_invalid_selected_metadata() {
    assert_eq!(error_for::<Invalid>().await.code(), MADS030);
}
#[tokio::test]
async fn rejects_ambiguous_selected_controller_metadata() {
    let error = error_for::<Ambiguous>().await;
    assert_eq!(error.code(), MADS030);
    assert!(error.to_string().contains("ambiguous"));
}

#[cfg(feature = "jwt")]
mod unrelated_strategy {
    #[derive(serde::Deserialize)]
    pub struct Claims;
    pub struct Strategy;
    #[mads_common::passport_strategy(name = "unmanaged-unrelated")]
    impl mads_common::PassportStrategy for Strategy {
        type Claims = Claims;
        type Principal = super::unused_guard::UnusedPrincipal;
        const TOKEN_KIND: mads_common::JwtTokenKind = mads_common::JwtTokenKind::Access;
        async fn validate(
            &self,
            _: &mads_common::PassportContext<'_>,
            _: &mads_common::JwtClaims<Self::Claims>,
        ) -> mads_common::PassportResult<Self::Principal> {
            Ok(super::unused_guard::UnusedPrincipal)
        }
    }
}

#[cfg(feature = "jwt")]
#[tokio::test]
async fn selected_guard_still_requires_its_strategy() {
    let error = error_for::<unused_guard::GuardedController>().await;
    assert_eq!(error.code(), mads_common::MADS130);
    assert!(error.to_string().contains("missing"));
}

#[cfg(feature = "jwt")]
#[test]
fn ordinary_rootless_preflight_still_rejects_invalid_catalog() {
    let error = mads_common::PassportStrategyCatalog::preflight(&[]).unwrap_err();
    assert_eq!(error.code(), mads_common::MADS130);
}
