//! In-process HTTP dispatch, comparisons, and teardown.
#![allow(missing_docs)]
use axum::{
    Json,
    body::Body,
    http::{HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::{FutureExt, stream};
use mads_core::{ApplicationContext, LifecycleFuture, LifecycleHook, LifecycleResource};
use mads_testing::{__private::test_fixture, TestError};
use serde_json::{Value, json};
use std::{
    panic::AssertUnwindSafe,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[mads_core::service]
struct Service {
    message: String,
}
#[mads_common::routes]
trait Routes {
    #[get("/users")]
    async fn users(&self) -> Json<Value>;
    #[get("/text")]
    async fn text(&self) -> Response;
    #[post("/echo")]
    async fn echo(&self, body: Json<Value>) -> (StatusCode, Json<Value>);
    #[put("/echo")]
    async fn put(&self) -> &'static str;
    #[patch("/echo")]
    async fn patch(&self) -> &'static str;
    #[delete("/echo")]
    async fn delete(&self) -> &'static str;
    #[get("/broken")]
    async fn broken(&self) -> Response;
}
#[mads_common::controller(routes = [Routes])]
struct Controller {
    service: Service,
    resource: Resource,
}
impl Routes for Controller {
    async fn users(&self) -> Json<Value> {
        Json(json!({"name":self.service.message,"id":1}))
    }
    async fn text(&self) -> Response {
        let _ = &self.resource;
        (
            [(
                HeaderName::from_static("x-test"),
                HeaderValue::from_static("yes"),
            )],
            self.service.message.clone(),
        )
            .into_response()
    }
    async fn echo(&self, Json(body): Json<Value>) -> (StatusCode, Json<Value>) {
        (StatusCode::CREATED, Json(body))
    }
    async fn put(&self) -> &'static str {
        "put"
    }
    async fn patch(&self) -> &'static str {
        "patch"
    }
    async fn delete(&self) -> &'static str {
        "delete"
    }
    async fn broken(&self) -> Response {
        Response::new(Body::from_stream(stream::once(async {
            Err::<String, _>(std::io::Error::other("broken body"))
        })))
    }
}
#[mads_common::routes]
trait OtherRoutes {
    #[get("/other")]
    async fn other(&self) -> &'static str;
}
#[mads_common::controller(routes = [OtherRoutes])]
struct Other;
impl OtherRoutes for Other {
    async fn other(&self) -> &'static str {
        panic!("unselected")
    }
}
#[derive(Clone, Default)]
struct Stops(Arc<AtomicUsize>);
struct StopHook(Stops);
impl LifecycleHook for StopHook {
    fn name(&self) -> &str {
        "http"
    }
    fn start<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn stop<'a>(&'a self, _: &'a ApplicationContext) -> LifecycleFuture<'a> {
        Box::pin(async {
            tokio::task::yield_now().await;
            self.0.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}
#[derive(Clone)]
struct Resource;
#[mads_core::provider(lifecycle)]
async fn resource(stops: Stops) -> LifecycleResource<Resource> {
    LifecycleResource::new(Resource).with_application_hook(StopHook(stops))
}
fn fixture(stops: Stops) -> mads_testing::ControllerFixture<Controller> {
    test_fixture()
        .provide(String::from("Ada"))
        .provide(stops)
        .controller::<Controller>()
}
#[tokio::test]
async fn controller_get_uses_selected_chain_and_compares_json() {
    fixture(Stops::default())
        .run(|app| async move {
            assert_eq!(app.resolve::<Service>().unwrap().message, "Ada");
            app.get("/users")
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::OK)
                .assert_json(json!({"id":1,"name":"Ada"}));
            app.get("/other")
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::NOT_FOUND);
            app.get("/text")
                .send()
                .await
                .unwrap()
                .assert_text("Ada")
                .assert_header(
                    HeaderName::from_static("x-test"),
                    HeaderValue::from_static("yes"),
                );
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn all_methods_and_json_dispatch() {
    fixture(Stops::default())
        .run(|app| async move {
            app.post("/echo")
                .header(
                    HeaderName::from_static("x-input"),
                    HeaderValue::from_static("value"),
                )
                .json(&json!({"hello":"world"}))
                .unwrap()
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::CREATED)
                .assert_json(json!({"hello":"world"}));
            app.put("/echo").send().await.unwrap().assert_text("put");
            app.patch("/echo")
                .send()
                .await
                .unwrap()
                .assert_text("patch");
            app.delete("/echo")
                .send()
                .await
                .unwrap()
                .assert_text("delete");
            app.request(Method::HEAD, "/text")
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::OK)
                .assert_text("");
        })
        .await
        .unwrap();
}
struct CannotSerialize;
impl serde::Serialize for CannotSerialize {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("cannot serialize"))
    }
}
#[tokio::test]
async fn request_serialization_and_body_errors_are_results() {
    fixture(Stops::default())
        .run(|app| async move {
            assert!(matches!(
                app.get("\n").send().await,
                Err(TestError::Request(_))
            ));
            assert!(matches!(
                app.post("/echo").json(&CannotSerialize),
                Err(TestError::Serialization(_))
            ));
            assert!(matches!(
                app.get("/broken").send().await,
                Err(TestError::ResponseBody(_))
            ));
        })
        .await
        .unwrap();
}
fn panic_text(panic: Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap()
}
#[tokio::test]
async fn mismatch_reports_expected_actual_and_still_shuts_down() {
    for kind in 0..5 {
        let stops = Stops::default();
        let result = AssertUnwindSafe(fixture(stops.clone()).run(|app| async move {
            let response = app.get("/text").send().await.unwrap();
            match kind {
                0 => {
                    response.assert_status(StatusCode::CREATED);
                }
                1 => {
                    response.assert_text("Grace");
                }
                2 => {
                    response.assert_header(
                        HeaderName::from_static("x-test"),
                        HeaderValue::from_static("no"),
                    );
                }
                3 => {
                    response.assert_json(json!({"name":"Ada"}));
                }
                _ => {
                    app.get("/users")
                        .send()
                        .await
                        .unwrap()
                        .assert_json(json!({"id":2}));
                }
            }
        }))
        .catch_unwind()
        .await;
        let message = panic_text(result.unwrap_err());
        assert!(message.contains("expected"), "{message}");
        assert!(message.contains("actual"), "{message}");
        if kind == 3 {
            assert!(message.contains("Ada"));
            assert!(message.contains("JSON"));
        }
        assert_eq!(stops.0.load(Ordering::SeqCst), 1);
    }
}
