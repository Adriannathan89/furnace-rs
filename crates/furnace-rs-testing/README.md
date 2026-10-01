# furnace-rs-testing

Focused in-process fixtures for FURNACE unit tests and TDD. Select one registered
service, repository, or controller; FURNACE builds its registered dependency chain,
including private providers, without requiring module declarations or imports.
Rust privacy still applies. Supplied values replace matching constructors.

## Setup

```toml
[dependencies]
furnace-rs = "=0.9.2"

[dev-dependencies]
furnace-rs-testing = "=0.9.2"
```

Apply `#[furnace-rs::test]` to a zero-argument, nongeneric `async fn`. Cargo discovers
it as a test, and the macro creates `test_fixture()` inside that function.
No separate Tokio dependency or `#[tokio::test]` is needed. The supported fixture
entry point is available only inside annotated tests; hidden macro internals
are not a security boundary against deliberate use.

## Service or repository

```rust
use furnace_rs_testing::sea_orm::{DatabaseConnection, DbBackend, MockDatabase};

#[furnace-rs::storage]
struct UserRepository {
    database: DatabaseConnection,
}

#[furnace-rs::burner]
struct UserService {
    repository: UserRepository,
}

#[furnace-rs::test]
async fn service_uses_mock_database() {
    test_fixture()
        .mock_database(MockDatabase::new(DbBackend::Sqlite))
        .subject::<UserService>()
        .run(|context| async move {
            assert!(context.resolve::<UserService>().is_ok());
        })
        .await
        .unwrap();
}
```

Queue results with SeaORM's `append_query_results` or `append_exec_results`.
This is a scripted in-memory mock, not a live SQLite engine. Only
`DbBackend::Sqlite` is accepted. A chain that requires `DatabaseConnection`
fails setup until `mock_database` supplies it; production connectors are never
used. `provide(DatabaseConnection)` is rejected. A type may be supplied once.

## Controller HTTP assertions

```rust
use furnace_rs_testing::http_types::{HeaderValue, StatusCode, header::CONTENT_TYPE};

#[furnace-rs::routes]
trait UserRoutes {
    #[get("/users")]
    async fn users(&self) -> furnace-rs::Json<serde_json::Value>;
}

#[furnace-rs::controller(routes = [UserRoutes])]
struct UserController;

impl UserRoutes for UserController {
    async fn users(&self) -> furnace-rs::Json<serde_json::Value> {
        furnace-rs::Json(serde_json::json!({ "name": "Ada" }))
    }
}

#[furnace-rs::test]
async fn controller_returns_users() {
    test_fixture().controller::<UserController>()
        .run(|client| async move {
            client.get("/users").send().await.unwrap()
                .assert_status(StatusCode::OK)
                .assert_json(serde_json::json!({ "name": "Ada" }))
                .assert_header(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        })
        .await
        .unwrap();
}
```

Only the selected controller's routes and guards are installed. Requests use
Tower in process, without a listener. Use `get`, `post`, `put`, `patch`, `delete`,
or `request(Method, uri)`. Add headers with `header` and encode a request with
`json(&value)`. Compare buffered responses with `assert_status`, `assert_json`,
`assert_text`, and `assert_header`. JSON object key order does not matter.
Transport and setup failures return `TestError`; comparison failures panic with
expected and actual values.

## Lifecycle

`run` builds and starts the fixture, executes the body, then awaits shutdown
before returning. An unwinding body panic resumes after shutdown. If shutdown
also fails, its diagnostic and source chain are reported to stderr while the
original panic is preserved. Startup failures use FURNACE rollback and skip the
body. Normal shutdown failures return `TestError::Furnace` with the original source.
Process abort and external cancellation cannot guarantee asynchronous cleanup.
