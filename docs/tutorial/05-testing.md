# 5. Testing

Previous: [Accounts and login](04-accounts-and-login.md) · [Learning path](README.md)

Tests run the real dependency graph in process: no listener, no `.env`, no
ports. You pick one subject, replace the providers you want to control, and
assert.

Add the testing crate:

```toml
[dev-dependencies]
furnace-rs-testing = "=2.0.0"
serde_json = "1"
```

The listings come from
[`example/user-accounts/src/accounts/tests.rs`](../../example/user-accounts/src/accounts/tests.rs).
Run them with `cargo test` in that directory.

## Supplies replace real providers

```rust
fn jwt() -> JwtService {
    let config = ConfigBuilder::new()
        .source(furnace_rs::core::MapSource::new(
            "test",
            [("passport.secret", "test-only-signing-key-with-32-bytes!")],
        ))
        .build()
        .unwrap();

    JwtService::from_config(&config).unwrap()
}

fn fast_hasher() -> PasswordHasher {
    PasswordHasher::new(DefaultPasswordHasher::with_costs(32, 1, 1).unwrap())
}
```

- `jwt()` builds the service from in-memory configuration, so tests do not
  depend on a `.env` file.
- `fast_hasher()` uses the smallest Argon2 costs. Production hashing is slow on
  purpose; tests should not pay that on every call.

A value passed to `provide` is used wherever that type is injected.

## Test a service

```rust
#[furnace_rs::test]
async fn stores_a_hash_and_authenticates_only_the_right_password() {
    test_fixture()
        .provide(jwt())
        .provide(fast_hasher())
        .provide(AccountRepository::default())
        .subject::<AccountService>()
        .run(|context| async move {
            let accounts = context.resolve::<AccountService>().unwrap();

            let created = accounts.register("ada", PASSWORD).await.unwrap().unwrap();
            assert!(created.password_hash.starts_with("$argon2id$"));
            assert!(accounts.register("ada", PASSWORD).await.unwrap().is_none());

            let known = accounts.authenticate("ada", PASSWORD).await.unwrap();
            assert_eq!(known.map(|account| account.id), Some(created.id));
            assert!(accounts.authenticate("ada", "wrong password").await.unwrap().is_none());
            assert!(accounts.authenticate("nobody", PASSWORD).await.unwrap().is_none());
        })
        .await
        .unwrap();
}
```

`#[furnace_rs::test]` replaces `#[tokio::test]` and creates `test_fixture()`
inside the function. `subject::<T>()` builds `T` and everything it depends on;
`context.resolve::<T>()` hands it to the test.

## Test a controller over HTTP

```rust
#[furnace_rs::test]
async fn http_boundary_validates_input_and_hides_which_part_failed() {
    test_fixture()
        .provide(jwt())
        .provide(fast_hasher())
        .provide(AccountRepository::default())
        .controller::<AccountController>()
        .run(|client| async move {
            let credentials = json!({ "username": "ada", "password": PASSWORD });

            client
                .post("/accounts/register")
                .json(&credentials)
                .unwrap()
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::OK)
                .assert_json(json!({ "id": 1, "username": "ada" }));

            let rejected = json!({
                "error": { "code": "unauthorized", "message": "invalid credentials" }
            });
            for attempt in [
                json!({ "username": "ada", "password": "wrong password!!" }),
                json!({ "username": "nobody", "password": PASSWORD }),
            ] {
                client
                    .post("/accounts/login")
                    .json(&attempt)
                    .unwrap()
                    .send()
                    .await
                    .unwrap()
                    .assert_status(StatusCode::UNAUTHORIZED)
                    .assert_json(rejected.clone());
            }
        })
        .await
        .unwrap();
}
```

`controller::<T>()` installs only that controller's routes and guard and
returns a client. The loop asserts the security property from step 4: a wrong
password and an unknown username produce the same response. The listing is
abridged; the file also covers the 422 and 409 paths.

## What to supply

| Dependency | In a test |
| --- | --- |
| Built by `#[burner]`, `#[storage]`, or `#[controller]` | Found automatically. |
| A hand-written `impl Injector` | Supply it with `provide`, as `AccountRepository` is above. |
| Configuration-backed services such as `JwtService` | Supply one built from a `MapSource`. |
| `DatabaseConnection` | `mock_database(MockDatabase::new(DbBackend::Sqlite))` with scripted results. |
| Anything slow or external | Supply a cheap or fake implementation. |

## Assertions

`get`, `post`, `put`, `patch`, `delete`, and `request(method, uri)` build a
request; `header` and `json` fill it; `send` returns a response with
`assert_status`, `assert_json`, `assert_text`, and `assert_header`. JSON key
order does not matter.

## Try it

- Add a test that registers, logs in, and asserts the `access_token` field is
  present.
- Supply a `PasswordHasher` built from your own failing
  `PasswordHasherService` and assert the endpoint answers 500 with
  `internal server error`.

Reference: [testing crate guide](../../crates/furnace-rs-testing/README.md)

## Where next

| Goal | Read |
| --- | --- |
| Store data in PostgreSQL | [Persistence](../guides/furnace-rs-persistence.md), [Entity-bound storage](../guides/storage.md), [posts-crud](../../example/posts-crud/) |
| Real-time connections | [WebSocket transport](../guides/socket.md), [socket-transport](../../example/socket-transport/) |
| Service-to-service calls | [gRPC transport](../guides/grpc.md), [grpc-transport](../../example/grpc-transport/) |
| Understand the internals | [Architecture](../guides/ARCHITECTURE.md), [Implementation guide](../guides/IMPLEMENTATION.md) |
| Daily tooling | [CLI](../guides/CLI.md) |
