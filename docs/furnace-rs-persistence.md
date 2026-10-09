# Native SeaORM PostgreSQL persistence

FURNACE 1.0.2 integrates SeaORM's native PostgreSQL connection through the
separate `furnace-rs-persistence` crate. The crate has no default backend;
select `sea-orm-postgres` explicitly. Database support is not a facade feature.

## Dependencies

```toml
[dependencies]
furnace = { package = "furnace-rs", version = "=1.0.2", default-features = false, features = ["http", "runtime-tokio"] }
furnace-rs-persistence = { version = "=1.0.2", features = ["sea-orm-postgres"] }
sea-orm = { version = "2.0.0", default-features = false, features = ["macros", "sqlx-postgres", "runtime-tokio-rustls"] }
```

The workspace uses Rust edition 2024 and requires Rust 1.94 or newer.
See the runnable [Posts CRUD example](../example/posts-crud/) for entities,
queries, input validation, and HTTP error handling.

## Register the database

Import `DatabaseCauldron` into the selected application's root. It globally
exports a native `DatabaseConnection` to reachable cauldrons. Repositories
inject that value directly:

```rust,no_run
use furnace::prelude::*;
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};

#[storage]
struct PostRepository {
    database: DatabaseConnection,
}

#[cauldron]
struct AppCauldron;

impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(DatabaseCauldron).provide::<PostRepository>()
    }
}
```

Add and register a controller to run this root with `Furnace::burn`.
The imported cauldron constructs the pool, checks readiness during lifecycle
startup before listener binding, and closes the connection on graceful shutdown.
SeaORM owns entities, queries, transactions, and migrations. FURNACE does not
apply or generate migrations, and the `furnace` CLI has no database commands.

## Configuration

`Furnace::burn` reads the current directory's optional `.env` for interpolation,
optional `furnace.toml`, then scalar `FURNACE_*` environment overrides.

```toml
# furnace.toml
[persistence.seaorm]
url = "${DATABASE_URL}"
max_connections = 10
connect_timeout_seconds = 5
acquire_timeout_seconds = 5
idle_timeout_seconds = 300
max_lifetime_seconds = 1800
sqlx_logging = false
```

Only `url` is required. Omitted pool settings keep the native defaults.
Supported optional keys are `min_connections`, `max_connections`,
`connect_timeout_seconds`, `acquire_timeout_seconds`, `idle_timeout_seconds`,
`max_lifetime_seconds`, and `sqlx_logging`. Explicit connection counts must be
positive; when both are supplied, minimum must not exceed maximum.

Durations must fit the platform's monotonic-clock deadline range. Idle and
maximum-lifetime durations must be positive. With native `ConnectOptions`,
`None` disables those maintenance policies; zero is rejected. Validation
identifies configuration keys without rendering credentials or rejected values.

Use a process variable or an ignored `.env` for `DATABASE_URL`. The low-level
builder uses only the `Config` supplied by the caller and does not load files.

## Manual connection and custom construction

`DatabaseFactory::provide` returns the native connection or a typed error:

```rust,no_run
use furnace_rs_persistence::{DatabaseFactory, PersistenceResult};
use furnace_rs_persistence::sea_orm::{DatabaseConnection, SeaOrmPostgres};

async fn connect(url: String) -> PersistenceResult<DatabaseConnection> {
    DatabaseFactory.provide(SeaOrmPostgres::new(url)).await
}
```

`SeaOrmPostgres::from_options` accepts native `ConnectOptions`, and `options_mut`
allows native option customization. The connector validates PostgreSQL scheme
and duration bounds before connecting. A manually created connection does not
attach lifecycle hooks automatically. Application-authored injectors can use
`.provide_with::<DatabaseConnection, Implementer>()` and override
`Injector::lifecycle` to manage their own resources.

## Errors and tracing

`PersistenceError::kind()` classifies connector failures. Ordinary `Display`
and `Debug` retain operation/category information while hiding native sources;
explicit `std::error::Error::source()` access retains the underlying error.
Conversion to a core error uses `FURNACE140`.

The opt-in `sea-orm` feature on `furnace-rs` lets HTTP handlers propagate native
query `DbErr` values with `?` into redacted HTTP 500 responses. The
[simple CRUD example](../example/posts-crud/) uses this conversion and returns
`NotFound` for absent posts; the [service variant](../example/post-crud-with-service/)
shows an optional layer for business logic. Applications own domain-specific
query policies and client messages. Typed connector `PersistenceError` values
remain separate from query errors.

The connector suppresses tracing while polling SeaORM connection establishment,
including callbacks inside that operation, because native connection spans can
contain the URL. Surrounding application tracing and configured statement
logging keep native behavior. Explicit source logging and authored diagnostics
remain application responsibilities.

## Verification

```sh
cargo test --locked -p furnace-rs-persistence --all-features
```

Live PostgreSQL tests are ignored by default. With a disposable PostgreSQL 16
instance and `FURNACE_TEST_DATABASE_URL` set, run:

```sh
cargo test --locked -p furnace-rs-persistence --all-features \
  --test postgres --test recovery -- --ignored --test-threads=1
```

See the [security policy](SECURITY.md), [release verification](releases/1.0.2.md),
and [migration guide](importance/furnace-rs-migration.md).
