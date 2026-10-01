//! Public furnace vocabulary and standard startup contract.
#![cfg(all(feature = "http", feature = "runtime-tokio"))]
use mads::prelude::*;
#[mads::furnace]
struct Empty;
impl Furnace for Empty {
    fn register(self) -> FurnaceRegistration<Self> {
        FurnaceRegistration::new(self)
    }
}
#[tokio::test]
async fn burn_rejects_an_application_without_routes_before_binding() {
    fn assert_send<T: Send>(_: &T) {}
    let startup = Mads::burn::<Empty>();
    assert_send(&startup);
    assert!(startup.await.is_err());
}

#[storage]
struct SharedStorage;
#[furnace]
struct Shared;
impl Furnace for Shared {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<SharedStorage>().export::<SharedStorage>()
    }
}
#[furnace]
struct PrivateShared;
impl Furnace for PrivateShared {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<SharedStorage>()
    }
}
#[routes]
trait FirstRoutes {
    #[get("/first")]
    async fn first(&self) -> &'static str;
}
#[controller(routes = [FirstRoutes])]
struct FirstController {
    _storage: SharedStorage,
}
impl FirstRoutes for FirstController {
    async fn first(&self) -> &'static str {
        "first"
    }
}
#[routes]
trait SecondRoutes {
    #[get("/second")]
    async fn second(&self) -> &'static str;
}
#[controller(routes = [SecondRoutes])]
struct SecondController {
    _storage: SharedStorage,
}
impl SecondRoutes for SecondController {
    async fn second(&self) -> &'static str {
        "second"
    }
}
#[furnace]
struct FirstFeature;
impl Furnace for FirstFeature {
    fn register(self) -> FurnaceRegistration<Self> {
        self.controller::<FirstController>().import(Shared)
    }
}
#[furnace]
struct SecondFeature;
impl Furnace for SecondFeature {
    fn register(self) -> FurnaceRegistration<Self> {
        self.controller::<SecondController>().import(Shared)
    }
}
#[furnace]
struct App;
impl Furnace for App {
    fn register(self) -> FurnaceRegistration<Self> {
        self.import(FirstFeature).import(SecondFeature)
    }
}
#[furnace]
struct PrivateApp;
impl Furnace for PrivateApp {
    fn register(self) -> FurnaceRegistration<Self> {
        self.controller::<FirstController>().import(PrivateShared)
    }
}
#[tokio::test]
async fn imported_features_share_one_exported_storage_and_select_both_routes() {
    use mads::axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut builder = Mads::builder();
    builder.root::<App>().unwrap();
    assert_eq!(builder.analyze().module_graph().unwrap().modules().len(), 4);
    let app = builder.build().await.unwrap();
    let router = build_router(&app).unwrap();
    for path in ["/first", "/second"] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let mut builder = Mads::builder();
    builder.root::<PrivateApp>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == mads::core::MADS009)
    );
}
