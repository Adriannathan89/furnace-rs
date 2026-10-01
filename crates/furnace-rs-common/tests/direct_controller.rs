//! Direct controller endpoints preserve native extraction and explicit ownership.
#![cfg(feature = "http")]
#![allow(missing_docs)]
use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use furnace_rs_common::core::{Cauldron, CauldronRegistration, Furnace, cauldron, element};
use furnace_rs_common::{
    Json, Path, RouteCatalog, SealRegistration, Sealable, build_router, controller,
};
use std::sync::Mutex;
use tower::ServiceExt;

static ORDER: Mutex<Vec<&str>> = Mutex::new(Vec::new());
#[derive(Clone)]
struct Repository;
#[element]
fn repository() -> Repository {
    ORDER.lock().unwrap().push("repository");
    Repository
}
#[derive(Clone)]
struct Service {
    _repository: Repository,
}
#[element]
fn service(repository: Repository) -> Service {
    ORDER.lock().unwrap().push("service");
    Service {
        _repository: repository,
    }
}

#[controller]
struct UserController {
    service: Service,
}
impl Sealable for UserController {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}
#[controller(route = "/user/")]
impl UserController {
    #[get("/:id")]
    async fn find(&self, Path(id): Path<u64>) -> String {
        self.helper();
        format!("user:{id}")
    }
    #[get]
    fn list(&self) -> &'static str {
        self.helper();
        "users"
    }
    #[post]
    async fn create(&self, Json(value): Json<String>) -> String {
        value
    }
    #[get("/status")]
    fn status() -> &'static str {
        "ready"
    }
    #[cfg(any())]
    #[get("/{id}")]
    async fn disabled(&self) -> &'static str {
        "disabled"
    }
    #[get("/self")]
    fn same(
        &self,
        axum::extract::Extension(other): axum::extract::Extension<Self>,
    ) -> &'static str {
        if std::ptr::eq(&self.service, &other.service) {
            "same"
        } else {
            "different"
        }
    }
    #[get("/files/{*path}")]
    fn files(&self, Path(path): Path<String>) -> String {
        path
    }
    fn helper(&self) {
        let _ = &self.service;
    }
}
#[cauldron]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<Repository>()
            .provide::<Service>()
            .controller::<UserController>()
    }
}

#[tokio::test]
async fn inherent_handlers_use_registered_dependencies_and_native_extractors() {
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    assert_eq!(*ORDER.lock().unwrap(), ["repository", "service"]);
    let router = build_router(&app).unwrap().layer(axum::extract::Extension(
        app.context()
            .resolve::<UserController>()
            .unwrap()
            .as_ref()
            .clone(),
    ));
    for (method, path, body, expected) in [
        (Method::GET, "/user/42", "", "user:42"),
        (Method::GET, "/user", "", "users"),
        (Method::POST, "/user", "\"created\"", "created"),
        (Method::GET, "/user/status", "", "ready"),
        (Method::GET, "/user/self", "", "same"),
        (Method::GET, "/user/files/a/b", "", "a/b"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            expected
        );
    }
    let routes = RouteCatalog::routes_for::<UserController>();
    assert_eq!(routes.len(), 6);
    assert_eq!(routes[0].full_path(), "/user/{id}");
}

mod conflict {
    use super::*;
    #[controller]
    pub struct Controller;
    impl Sealable for Controller {
        fn seals() -> SealRegistration<Self> {
            SealRegistration::new()
        }
    }
    #[controller(route = "/conflict")]
    impl Controller {
        #[get("/:id")]
        fn first(&self) {}
        #[get("/{name}")]
        fn second(&self) {}
    }
}
#[test]
fn canonical_capture_conflicts_are_rejected_before_router_registration() {
    let error = RouteCatalog::validate_controller::<conflict::Controller>().unwrap_err();
    assert_eq!(error.code(), furnace_rs_common::core::FURNACE030);
}

mod cross_verb {
    use super::*;
    #[controller]
    pub struct Controller;
    impl Sealable for Controller {
        fn seals() -> SealRegistration<Self> {
            SealRegistration::new()
        }
    }
    #[controller(route = "/cross")]
    impl Controller {
        #[get("/{id}")]
        fn get(&self) {}
        #[post("/{name}")]
        fn post(&self) {}
    }
}
#[test]
fn capture_names_must_agree_across_verbs() {
    let error = RouteCatalog::validate_controller::<cross_verb::Controller>().unwrap_err();
    assert_eq!(error.code(), furnace_rs_common::core::FURNACE030);
}
mod wildcard_conflict {
    use super::*;
    #[controller]
    pub struct Controller;
    impl Sealable for Controller {
        fn seals() -> SealRegistration<Self> {
            SealRegistration::new()
        }
    }
    #[controller(route = "/files")]
    impl Controller {
        #[get("/{*path}")]
        fn first(&self) {}
        #[get("/*other")]
        fn second(&self) {}
    }
}
#[test]
fn wildcard_conflicts_are_rejected_before_registration() {
    assert_eq!(
        RouteCatalog::validate_controller::<wildcard_conflict::Controller>()
            .unwrap_err()
            .code(),
        furnace_rs_common::core::FURNACE030
    );
}

mod wildcard_parameter_conflict {
    use super::*;
    #[controller]
    pub struct Controller;
    impl Sealable for Controller {
        fn seals() -> SealRegistration<Self> {
            SealRegistration::new()
        }
    }
    #[controller(route = "/overlap")]
    impl Controller {
        #[get("/{*path}")]
        fn wildcard(&self) {}
        #[post("/{id}/detail")]
        fn parameter(&self) {}
    }
}
#[test]
fn wildcard_and_parameter_tree_conflicts_are_rejected() {
    assert_eq!(
        RouteCatalog::validate_controller::<wildcard_parameter_conflict::Controller>()
            .unwrap_err()
            .code(),
        furnace_rs_common::core::FURNACE030
    );
}

mod divergent_captures {
    use super::*;
    #[controller]
    pub struct Controller;
    impl Sealable for Controller {
        fn seals() -> SealRegistration<Self> {
            SealRegistration::new()
        }
    }
    #[controller(route = "/branches")]
    impl Controller {
        #[get("/:id/foo")]
        fn foo(&self, Path(id): Path<String>) -> String {
            format!("id:{id}")
        }
        #[get("/:name/bar")]
        fn bar(&self, Path(name): Path<String>) -> String {
            format!("name:{name}")
        }
    }
    #[cauldron]
    pub struct Root;
    impl Cauldron for Root {
        fn register(self) -> CauldronRegistration<Self> {
            self.controller::<Controller>()
        }
    }
}

#[tokio::test]
async fn divergent_capture_branches_preserve_native_parameter_names() {
    RouteCatalog::validate_controller::<divergent_captures::Controller>().unwrap();
    let mut builder = Furnace::builder();
    builder.root::<divergent_captures::Root>().unwrap();
    let app = builder.build().await.unwrap();
    let router = build_router(&app).unwrap();
    for (uri, expected) in [
        ("/branches/42/foo", "id:42"),
        ("/branches/alice/bar", "name:alice"),
    ] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(body.as_ref(), expected.as_bytes());
    }
}
