//! Public cauldron vocabulary and standard startup contract.
#![cfg(all(feature = "http", feature = "runtime-tokio"))]
use furnace_rs::prelude::*;
#[furnace_rs::cauldron]
struct Empty;

#[tokio::test]
async fn burn_rejects_an_application_without_routes_before_binding() {
    fn assert_send<T: Send>(_: &T) {}
    let startup = Furnace::burn::<Empty>();
    assert_send(&startup);
    assert!(startup.await.is_err());
}

#[storage]
struct SharedStorage;
#[cauldron(providers = [SharedStorage], exports = [SharedStorage])]
struct Shared;

#[cauldron(providers = [SharedStorage])]
struct PrivateShared;

#[controller]
struct FirstController {
    #[inject]
    _storage: SharedStorage,
}

impl ::furnace_rs::Sealable for FirstController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl FirstController {
    #[get("/first")]
    async fn first(&self) -> &'static str {
        "first"
    }
}

#[controller]
struct SecondController {
    #[inject]
    _storage: SharedStorage,
}

impl ::furnace_rs::Sealable for SecondController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl SecondController {
    #[get("/second")]
    async fn second(&self) -> &'static str {
        "second"
    }
}

#[cauldron(controllers = [FirstController], imports = [Shared])]
struct FirstFeature;

#[cauldron(controllers = [SecondController], imports = [Shared])]
struct SecondFeature;

#[cauldron(imports = [FirstFeature, SecondFeature])]
struct App;

#[cauldron(controllers = [FirstController], imports = [PrivateShared])]
struct PrivateApp;

#[tokio::test]
async fn imported_features_share_one_exported_storage_and_select_both_routes() {
    use furnace_rs::axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    assert_eq!(
        builder
            .analyze()
            .cauldron_graph()
            .unwrap()
            .cauldrons()
            .len(),
        4
    );
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
    let mut builder = Furnace::builder();
    builder.root::<PrivateApp>().unwrap();
    let analysis = builder.analyze();
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == furnace_rs::core::FURNACE009)
    );
}
