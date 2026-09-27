# MADS Testing API Design

**Status:** Approved conversational design; written contract awaiting review

**Target:** `mads-testing` in the Rust 1.94, edition 2024 workspace

## Intent

Give MADS application authors a small, in-process test fixture for unit tests
and TDD. A test chooses one registered controller, service, or repository,
supplies test values, and lets MADS construct that subject's registered
dependency chain without declaring a root module. Controller tests send HTTP
requests to that controller's generated routes and compare the result with
short assertions. SeaORM tests use a scripted, in-memory `MockDatabase` with
the SQLite backend. The fixture starts and shuts down lifecycle hooks around
the test body, including when an assertion unwinds.

Success means a test can exercise a controller through HTTP or a service
directly, with its real registered dependency chain and a supplied mock
database, while unrelated providers and routes do not run.

## Scope and constraints

- Add `mads-testing` as a workspace crate. The existing untracked, empty
  `crates/mads-testing` directory is user work and is the intended crate
  location; implementation must inspect it before editing.
- Add `#[mads::test]` for inline Rust modules. It provides the supported
  module-free fixture entry point inside the annotated module and compiles
  that module only for tests.
- Support registered controllers, services, and repositories as subjects.
- Select dependencies by concrete type, recursively, using existing MADS
  provider metadata and construction rules. A supplied value satisfies its
  matching dependency type and prevents its registered constructor from
  running.
- Bypass MADS module imports and provider visibility only for this focused
  test selection. Rust language privacy still applies to paths written by
  the test. Existing rooted and complete-catalog application behavior stays
  unchanged.
- Use SeaORM 2.0 `MockDatabase` with `DbBackend::Sqlite` only. This is a
  scripted mock, not a live SQLite database or SQLite driver. Do not open a
  socket or file.
- Require an explicit mock when the selected chain needs
  `DatabaseConnection`. Never fall through to a production database provider
  in that case. A non-SQLite mock is a setup error.
- Support in-process HTTP requests and chainable status, JSON, text, and
  header assertions. No listener is bound.
- Rust source remains documented under the workspace `missing_docs = deny`
  lint and contains no unsafe code. Avoid a dependency cycle among `mads`,
  `mads-core`, `mads-common`, `mads-persistence`, and `mads-testing`.

This version does not include real SQLite, migrations, database transaction
helpers, fixture factories, snapshots, property testing, or helpers beyond
the database, focused construction, and HTTP boundary agreed here. File-backed
`#[mads::test] mod tests;` is not supported.

## Approaches considered

1. **Chosen: focused selection in core and HTTP.** Extend the existing core
   graph path to select one provider and its dependency closure. Extend HTTP
   route construction to register only one selected controller. Wrap those
   paths in `mads-testing`. This reuses graph diagnostics, construction order,
   typed dispatch, and route validation.
2. **Synthetic test module.** Generate module metadata from the test macro
   and use rooted construction. This makes cross-module private dependencies
   awkward and ties test behavior to normal import rules.
3. **Separate test graph.** Walk provider descriptors in `mads-testing`.
   This would duplicate core validation and construction semantics.

## Public API contract

`#[mads::test]` is a module attribute re-exported by `mads`. It accepts an
inline module, applies `#[cfg(test)]`, and generates a module-local
`test_fixture()` function. That
function is the supported entry to the module-free fixture API. It must be
available to test functions in that module and unavailable through the
supported API in an unannotated module. Expansion rejects an external module
or a non-module item with a compile-time diagnostic. The macro does not
replace `#[tokio::test]` or execute the test body.

The fixture API has these operations and type relationships:

```rust
// Generated inside each #[mads::test] inline module:
fn test_fixture() -> mads_testing::TestFixtureBuilder;

impl TestFixtureBuilder {
    pub fn mock_database(self, mock: sea_orm::MockDatabase) -> Self;
    pub fn provide<T: Send + Sync + 'static>(self, value: T) -> Self;
    pub fn subject<T: Send + Sync + 'static>(self) -> SubjectFixture<T>;
    pub fn controller<T: Send + Sync + 'static>(self) -> ControllerFixture<T>;
}

impl<T> SubjectFixture<T> {
    pub async fn run<F, Fut>(self, body: F) -> TestResult<()>;
    // F: FnOnce(TestContext) -> Fut, Fut: Future<Output = ()>
}

impl<T> ControllerFixture<T> {
    pub async fn run<F, Fut>(self, body: F) -> TestResult<()>;
    // F: FnOnce(TestClient) -> Fut, Fut: Future<Output = ()>
}

impl TestContext {
    pub fn resolve<T: Send + Sync + 'static>(&self) -> mads_core::Result<Arc<T>>;
}

impl TestClient {
    pub fn resolve<T: Send + Sync + 'static>(&self) -> mads_core::Result<Arc<T>>;
    pub fn request(&self, method: http::Method, uri: &str) -> TestRequest;
    pub fn get(&self, uri: &str) -> TestRequest;
    pub fn post(&self, uri: &str) -> TestRequest;
    pub fn put(&self, uri: &str) -> TestRequest;
    pub fn patch(&self, uri: &str) -> TestRequest;
    pub fn delete(&self, uri: &str) -> TestRequest;
}

impl TestRequest {
    pub fn header(self, name: http::header::HeaderName,
                  value: http::header::HeaderValue) -> Self;
    pub fn json<S: serde::Serialize>(self, body: &S) -> TestResult<Self>;
    pub async fn send(self) -> TestResult<TestResponse>;
}

impl TestResponse {
    pub fn assert_status(self, expected: http::StatusCode) -> Self;
    pub fn assert_json(self, expected: serde_json::Value) -> Self;
    pub fn assert_text(self, expected: &str) -> Self;
    pub fn assert_header(self, name: http::header::HeaderName,
                         expected: http::header::HeaderValue) -> Self;
}
```

The public error alias is `TestResult<T> = Result<T, TestError>`. `TestError`
preserves a MADS diagnostic as its source for graph, construction, route,
startup, and shutdown failures. It separately identifies missing mock
database, unsupported mock backend, request construction, JSON serialization,
HTTP service, and response-body failures. A fixture may supply each concrete
type once; a repeated type is a setup error. `provide::<DatabaseConnection>`
is rejected so every database dependency uses the checked mock path.
`mads_testing::sea_orm` re-exports the SeaORM mock types and native
`DatabaseConnection`, so a test does not need a separate direct SeaORM
dependency merely to prepare a fixture.

Request methods use native HTTP types; the crate may re-export them for
ergonomics. `send` buffers one response body so multiple assertions can
inspect it. `assert_json` parses the actual body as JSON and compares
`serde_json::Value` values, so JSON object member order is irrelevant.
Assertions panic with expected and actual values, including a readable body
when parsing fails. The response stays available for chained assertions.

### Typical controller test

```rust
#[mads::test]
mod tests {
    use super::*;
    use mads_testing::sea_orm::{DbBackend, MockDatabase};

    #[tokio::test]
    async fn get_user() {
        let mock = MockDatabase::new(DbBackend::Sqlite)
            .append_query_results([[user_model()]]);

        test_fixture()
            .mock_database(mock)
            .controller::<UserController>()
            .run(|app| async move {
                app.get("/users/1")
                    .send().await.unwrap()
                    .assert_status(StatusCode::OK)
                    .assert_json(json!({ "id": 1 }));
            })
            .await
            .unwrap();
    }
}
```

`user_model()` represents the application's SeaORM model fixture. Tests
queue SeaORM query and execution results through SeaORM's native
`MockDatabase` methods before handing the mock to MADS.

### Typical service test

```rust
test_fixture()
    .mock_database(MockDatabase::new(DbBackend::Sqlite))
    .subject::<UserService>()
    .run(|context| async move {
        let service = context.resolve::<UserService>().unwrap();
        // Call and assert the service behavior here.
    })
    .await
    .unwrap();
```

The service example assumes `UserService` and its registered dependency chain
can start with an empty scripted mock; tests that execute queries enqueue
matching results first.

## Selection and routing behavior

The test fixture selects a single registered subject by concrete `TypeId`.
It traverses only registered dependencies reachable from that subject,
stopping at supplied values. It validates missing providers, duplicate or
ambiguous bindings, and cycles with existing core diagnostic semantics.
Unrelated catalog entries must not be validated, constructed, or started by
this fixture. A selected private provider can be constructed irrespective of
MADS module ownership or imports; it remains subject to Rust privacy when
named in test source.

For a controller subject, HTTP selection validates only that controller's
route metadata and installs only its generated routes. It uses the existing
typed registrar, extractors, response conversion, and Axum service path.
An unknown route returns the ordinary router response. Conflicts among the
selected controller's own routes still fail setup. Controllers elsewhere in
the binary do not affect this router. A service or repository subject receives
`TestContext`, which has resolution but no HTTP methods.

The fixture's SQLite mock is converted to SeaORM's native
`DatabaseConnection` and inserted as the provided value for that type. A
dependency on `DatabaseConnection` with no `.mock_database(...)` produces a
missing-mock setup error even if the linked catalog has a production provider
for the same type. The backend is checked before conversion. Generic
`provide` values may replace other registered providers in the chain.

## Lifecycle and failure behavior

`run` builds the focused application, constructs its selected chain, starts
its lifecycle hooks, and then invokes the asynchronous test body. It awaits
shutdown after the body completes normally. It catches an unwinding panic
from the body, awaits shutdown, then resumes the original panic. The original
assertion failure remains primary if shutdown also fails; the shutdown error
is reported alongside it. On a startup error, existing MADS lifecycle
rollback runs and the body is not invoked. On a normal shutdown error, `run`
returns that error.

This guarantee covers normal completion and Rust unwinding panics while the
`run` future is polled to completion. Process abort and external cancellation
cannot execute asynchronous cleanup. The fixture does not expose an API that
requires users to remember a separate shutdown call.

The macro gate applies to the supported public fixture entry point. Rust
procedural-macro expansion requires callable implementation internals; those
are hidden from documentation and are outside the supported contract. This is
a compile-time usage guard for normal consumers, not a security boundary
against deliberate invocation of hidden internals.

## Acceptance and documentation

Implementation must demonstrate:

1. A module-free service test constructs service → repository from registered
   metadata using a supplied SQLite `MockDatabase` and can resolve the
   service.
2. A module-free controller test constructs its chain, serves only its routes
   in process, and compares status, JSON, text, and headers.
3. A selected private provider in another MADS module can be used through
   the test path when its Rust type is legally nameable.
4. An unrelated provider that would fail construction is never invoked.
5. Missing mock, non-SQLite mock, repeated supplies, and a direct
   `DatabaseConnection` supply fail setup without a live connection attempt.
6. Missing and ambiguous selected providers, selected dependency cycles,
   invalid selected routes, and absent controller metadata yield useful
   diagnostics. Unselected invalid routes do not fail the fixture.
7. A mock database connection reaches the repository unchanged as SeaORM's
   native type.
8. Lifecycle start and shutdown run once in order after a successful body
   and after an assertion panic. Startup failure uses core rollback.
9. An unannotated module cannot use the supported `test_fixture()` entry
   point; the macro rejects external modules and non-module items.
10. Existing rooted and complete-catalog tests keep their behavior.

Add focused tests in the owning crates, a consumer example that compiles
through the public `mads` and `mads-testing` paths, and user documentation
showing both controller and service workflows. Relevant crate tests, workspace
format/lint/test/doc gates, and the repository coverage gate are the
implementation verification targets.
