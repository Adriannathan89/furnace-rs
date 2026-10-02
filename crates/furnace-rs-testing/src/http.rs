//! In-process HTTP requests against one selected controller.

use crate::{TestContext, TestError, TestResponse, TestResult};
use axum::{
    body::Body,
    http::{self, HeaderName, HeaderValue, Method, Request},
};
use std::sync::Arc;
use tower::ServiceExt;

/// An in-process client for the fixture's selected controller routes.
#[derive(Clone)]
pub struct TestClient {
    context: TestContext,
    router: axum::Router,
}
impl TestClient {
    pub(crate) fn new(context: TestContext, router: axum::Router) -> Self {
        Self { context, router }
    }

    /// Resolves a selected or explicitly supplied concrete provider.
    #[allow(clippy::result_large_err)]
    pub fn resolve<T: Send + Sync + 'static>(&self) -> furnace_rs_core::Result<Arc<T>> {
        self.context.resolve::<T>()
    }

    /// Creates a request using any native HTTP method.
    pub fn request(&self, method: Method, uri: &str) -> TestRequest {
        TestRequest {
            router: self.router.clone(),
            builder: Request::builder().method(method).uri(uri),
            body: Body::empty(),
        }
    }
    /// Creates a GET request.
    pub fn get(&self, uri: &str) -> TestRequest {
        self.request(Method::GET, uri)
    }
    /// Creates a POST request.
    pub fn post(&self, uri: &str) -> TestRequest {
        self.request(Method::POST, uri)
    }
    /// Creates a PUT request.
    pub fn put(&self, uri: &str) -> TestRequest {
        self.request(Method::PUT, uri)
    }
    /// Creates a PATCH request.
    pub fn patch(&self, uri: &str) -> TestRequest {
        self.request(Method::PATCH, uri)
    }
    /// Creates a DELETE request.
    pub fn delete(&self, uri: &str) -> TestRequest {
        self.request(Method::DELETE, uri)
    }
}

/// A pending request, dispatched without opening a listener.
#[must_use]
pub struct TestRequest {
    router: axum::Router,
    builder: http::request::Builder,
    body: Body,
}
impl TestRequest {
    /// Adds a native HTTP header to the request.
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.builder = self.builder.header(name, value);
        self
    }
    /// Serializes a JSON body and sets the JSON content type.
    pub fn json<S: serde::Serialize>(mut self, body: &S) -> TestResult<Self> {
        let bytes = serde_json::to_vec(body).map_err(TestError::Serialization)?;
        if let Some(headers) = self.builder.headers_mut() {
            headers.insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
        }
        self.body = Body::from(bytes);
        Ok(self)
    }
    /// Dispatches the request and buffers the response body once.
    pub async fn send(self) -> TestResult<TestResponse> {
        let request = self.builder.body(self.body).map_err(TestError::Request)?;
        let response = self
            .router
            .oneshot(request)
            .await
            .map_err(|error| TestError::Service(Box::new(error)))?;
        TestResponse::buffer(response).await
    }
}
