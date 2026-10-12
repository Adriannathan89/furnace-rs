//! Chainable comparisons against a buffered HTTP response.

use crate::{TestError, TestResult};
use axum::{
    body::{Bytes, to_bytes},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
};

/// A buffered response with chainable assertions.
#[derive(Debug)]
pub struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}
impl TestResponse {
    pub(crate) async fn buffer(response: Response) -> TestResult<Self> {
        let (parts, body) = response.into_parts();
        let body = to_bytes(body, usize::MAX)
            .await
            .map_err(TestError::ResponseBody)?;
        Ok(Self {
            status: parts.status,
            headers: parts.headers,
            body,
        })
    }
    /// Asserts the HTTP status, panicking with expected and actual values.
    pub fn assert_status(self, expected: StatusCode) -> Self {
        assert_eq!(
            self.status, expected,
            "HTTP status: expected {expected}, actual {}",
            self.status
        );
        self
    }
    /// Compares parsed JSON values independently of object key order.
    pub fn assert_json(self, expected: serde_json::Value) -> Self {
        let actual: serde_json::Value = serde_json::from_slice(&self.body).unwrap_or_else(|error| {
            panic!("JSON response: expected {expected}, actual body {:?}; JSON parse failed: {error}", String::from_utf8_lossy(&self.body))
        });
        assert_eq!(
            actual, expected,
            "JSON response: expected {expected}, actual {actual}"
        );
        self
    }
    /// Asserts the UTF-8 body text, panicking on invalid UTF-8 or mismatch.
    pub fn assert_text(self, expected: &str) -> Self {
        let actual = std::str::from_utf8(&self.body).unwrap_or_else(|error| {
            panic!("text response: expected {expected:?}, actual bytes {:?}; UTF-8 decoding failed: {error}", self.body)
        });
        assert_eq!(
            actual, expected,
            "text response: expected {expected:?}, actual {actual:?}"
        );
        self
    }
    /// Asserts a response header, including whether it is present.
    pub fn assert_header(self, name: HeaderName, expected: HeaderValue) -> Self {
        let actual = self.headers.get(&name);
        assert_eq!(
            actual,
            Some(&expected),
            "header {name}: expected {expected:?}, actual {actual:?}"
        );
        self
    }
}
