//! Direct controller handlers consume a manually bound trait service.
#![cfg(feature = "http")]
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use furnace_rs_common::core::{
    Cauldron, CauldronRegistration, Furnace, Injector, Result, cauldron,
};
use furnace_rs_common::{build_router, controller};
use std::sync::Arc;
use tower::ServiceExt;

trait Greeting: Send + Sync {
    fn greet(&self) -> &'static str;
}
struct Implementation;
impl Greeting for Implementation {
    fn greet(&self) -> &'static str {
        "injected"
    }
}
impl Injector<Arc<dyn Greeting>> for Implementation {
    type Dependencies = ();
    async fn inject((): ()) -> Result<Arc<dyn Greeting>> {
        Ok(Arc::new(Self))
    }
}
#[controller]
struct Controller {
    greeting: Arc<dyn Greeting>,
}
#[controller(route = "/injector")]
impl Controller {
    #[get]
    fn index(&self) -> &'static str {
        self.greeting.greet()
    }
}
#[cauldron]
struct App;
impl Cauldron for App {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide_with::<Arc<dyn Greeting>, Implementation>()
            .controller::<Controller>()
    }
}
#[tokio::test]
async fn controller_injector_retains_route_metadata_and_trait_binding() {
    assert!(Controller::descriptor().is_controller());
    let mut builder = Furnace::builder();
    builder.root::<App>().unwrap();
    let app = builder.build().await.unwrap();
    let original = app.context().resolve::<Controller>().unwrap();
    let cloned = original.as_ref().clone();
    assert!(Arc::ptr_eq(&original.greeting, &cloned.greeting));
    let response = build_router(&app)
        .unwrap()
        .oneshot(
            Request::builder()
                .uri("/injector")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        "injected"
    );
}
