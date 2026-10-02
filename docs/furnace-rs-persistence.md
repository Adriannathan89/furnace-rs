# FURNACE Persistence Connector Design

Status: original connector design record. The 0.9 database/CLI boundary is
superseded by the [approved removal design](superpowers/specs/2026-09-23-mads-0.9-database-surface-removal-design.md).

For the unreleased 1.0.0 preparation, applications add
`furnace-rs-persistence = { version = "=1.0.0", features = ["sea-orm-postgres"] }`
explicitly and import `furnace_rs_persistence::sea_orm::DatabaseCauldron`.
`DatabaseFactory::provide` returns the native `DatabaseConnection` or typed
`PersistenceError`; database support is not a `furnace-rs`/`furnace-rs-common`
feature, and the CLI has no database commands. SeaORM owns migrations.
The workspace currently aligns all nine packages at 1.0.0 with exact internal
pins. Rust edition 2024, MSRV 1.94, and the minimum SeaORM 2.0.0 dependency
remain the release baseline.

Current custom construction uses `.provide_with::<T, I>()` with `I: Injector<T>`
and `Injector::lifecycle` for native resource hooks. Follow the
[migration guide](importance/furnace-rs-migration.md) and
[release preparation guide](releases/1.0.0.md) for the current API.
The original proposal below is retained as historical context: its version
examples, `#[element]` forms, independent versioning proposal, and statements
about retaining Diesel do not describe the 1.0.0 API.

## Summary

`furnace-rs-persistence` is a connector crate between FURNACE and existing Rust ORM
ecosystems. It is not a new ORM, query builder, schema language, entity model,
or repository abstraction.

The crate integrates an ORM's native database instance with FURNACE application
construction, dependency injection, configuration, diagnostics, and lifecycle.
Application code continues to use the ORM exactly as documented by that ORM.
The first supported combination is SeaORM with PostgreSQL.

The initial public contract is:

- an application imports `furnace_rs_persistence::sea_orm::DatabaseCauldron` from its
  root module;
- `DatabaseCauldron` is a FURNACE global module;
- startup constructs one native `sea_orm::DatabaseConnection`;
- FURNACE registers that native connection as an application-scoped provider;
- repositories and services inject `DatabaseConnection` directly;
- SeaORM owns entities, relations, indexes, queries, transactions, and
  migrations;
- FURNACE checks the connection before serving and closes the pool during orderly
  shutdown.

The package and Rust crate name is `furnace-rs-persistence` / `furnace_rs_persistence`.
The misspelling `furnace-rs-persistance` is not used.

## Motivation

Building a FURNACE-owned ORM would duplicate mature work and force FURNACE to own a
large persistence surface: SQL generation, entity metadata, relations,
transactions, migrations, backend differences, and compatibility with database
drivers. It would also make native ORM examples harder to reuse.

A connector has a smaller and more durable responsibility:

1. construct the ORM's native database value;
2. register that value in the FURNACE provider graph;
3. expose it across module boundaries through a global module;
4. integrate readiness and shutdown with the FURNACE lifecycle;
5. normalize framework-facing failures without disclosing credentials.

This boundary lets FURNACE support multiple ORM ecosystems without inventing a
lowest-common-denominator database API.

## Goals

The first release must:

- ship as a crate that can be versioned and downloaded separately from the
  main `furnace-rs` facade;
- integrate with FURNACE dependency injection and module scoping;
- support SeaORM with PostgreSQL;
- register the native `sea_orm::DatabaseConnection` type;
- support the conventional `Furnace::burn::<AppCauldron>()` startup path;
- read connection settings from normal FURNACE configuration sources;
- perform connection readiness before the HTTP listener binds;
- explicitly close the SeaORM pool during graceful shutdown;
- retain typed internal causes while keeping configuration values and
  credentials out of public diagnostics;
- preserve an extension point for later ORM connectors.

## Non-goals

The first release does not:

- define a common query, entity, relation, index, or transaction API;
- wrap `DatabaseConnection` in an application-facing FURNACE type;
- generate SeaORM entities;
- replace `sea-orm-cli` or `sea-orm-migration`;
- automatically run migrations;
- expose MySQL or SQLite connectors;
- support multiple named SeaORM connections in one application;
- translate SeaORM errors into HTTP responses automatically;
- move the existing Diesel integration out of `furnace-rs-common`;
- make a database connection available unless the application explicitly
  imports the connector module.

Entity generation, schema definitions, indexes, relations, migrations, and
query execution remain native SeaORM concerns. Applications may run SeaORM
migrations explicitly using `MigratorTrait` without a FURNACE-specific migration
API.

## Approaches considered

### FURNACE-owned ORM

This would give FURNACE complete control over the API, but it would duplicate an
ORM ecosystem and make FURNACE responsible for database behavior far beyond its
application-framework role. This approach is rejected.

### Universal FURNACE database wrapper

A type such as `FurnaceDatabase<Backend>` could hide connector differences, but
applications would no longer receive the ORM's native type. It would either
leak ORM-specific methods over time or restrict users to an artificial common
subset. This approach is rejected.

### Native provider with lifecycle contribution

The selected approach constructs the native ORM value and contributes a
lifecycle hook alongside it. FURNACE stores only the native value in the provider
registry. The lifecycle contribution is framework metadata and is not visible
in repository or service APIs.

This approach requires one general addition to `furnace-rs-core`: an async provider
must be able to return a provider value together with one or more lifecycle
registrations. That capability is useful for database pools, message brokers,
background clients, and other resources created during provider construction.

## Crate and dependency boundaries

The intended dependency direction is:

```text
application
    |
    +-- furnace-rs
    |
    +-- furnace-rs-persistence
            |
            +-- furnace-rs-core
            +-- sea-orm
                    |
                    +-- sqlx-postgres

furnace-rs-core has no dependency on SeaORM or furnace-rs-persistence.
```

`furnace-rs-persistence` depends only on the FURNACE core contracts required for
configuration, providers, cauldrons, diagnostics, and lifecycle. It must not
depend on Axum or the HTTP layer.

The original proposal retained the Diesel integration in `furnace-rs-common`, but
that integration was removed for 0.9. SeaORM remains an explicit, native
provider, not a universal `Database` wrapper.

The connector module re-exports the supported ORM surface so applications can
use one coherent type identity while the same module also owns FURNACE-specific
connector types:

```rust
pub mod sea_orm {
    pub use ::sea_orm::*;

    pub struct DatabaseCauldron;
    pub struct SeaOrmPostgres;
}
```

Application code should prefer:

```rust
use furnace_rs_persistence::sea_orm::{DatabaseConnection, EntityTrait};
```

This prevents two independently selected SeaORM versions from producing
different `DatabaseConnection` types in the DI graph.

The first release has no default connector feature. The application enables:

```toml
furnace-rs-persistence = {
    version = "0.1",
    default-features = false,
    features = ["sea-orm-postgres"]
}
```

`sea-orm-postgres` enables SeaORM's PostgreSQL SQLx driver and Tokio/Rustls
runtime integration. Additional SeaORM value-type features remain explicitly
selected by `furnace-rs-persistence` features or by a compatible direct SeaORM
dependency. MySQL and SQLite require separate future connector features rather
than runtime backend selection.

## Public connector model

The connector abstraction is statically typed. It does not erase the native
database output:

```rust
pub trait DatabaseConnector: Send + Sync + 'static {
    type Database: Send + Sync + 'static;

    fn connect(
        self,
    ) -> impl Future<Output = Result<Self::Database, PersistenceError>> + Send;
}
```

`DatabaseFactory` provides the common construction entry point:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct DatabaseFactory;

impl DatabaseFactory {
    pub async fn provide<C>(
        &self,
        connector: C,
    ) -> Result<C::Database, PersistenceError>
    where
        C: DatabaseConnector,
    {
        connector.connect().await
    }
}
```

`provide` constructs and returns the connector's native instance. It does not
mutate the FURNACE registry by side effect. Registration occurs because a FURNACE
provider returns that instance. This keeps graph construction deterministic and
makes duplicate-provider validation remain a core responsibility.

The first connector is:

```rust
pub struct SeaOrmPostgres {
    options: sea_orm::ConnectOptions,
}

impl SeaOrmPostgres {
    pub fn new(url: impl Into<String>) -> Self;
    pub fn from_options(options: sea_orm::ConnectOptions) -> Self;
    pub fn options_mut(&mut self) -> &mut sea_orm::ConnectOptions;
}

impl DatabaseConnector for SeaOrmPostgres {
    type Database = sea_orm::DatabaseConnection;
}
```

`SeaOrmPostgres` validates that its URL selects PostgreSQL. It does not accept
MySQL or SQLite URLs when compiled as the PostgreSQL connector. Its `Debug`
implementation must redact the URL.

The connector accepts `ConnectOptions` rather than reproducing every SeaORM
pool option in the factory API. This preserves SeaORM as the authority for pool
configuration and provides a native escape hatch for advanced settings.

`DatabaseFactory::provide` can also be called outside FURNACE construction when
an application only wants the connector utility:

```rust
let factory = DatabaseFactory;
let database: sea_orm::DatabaseConnection = factory
    .provide(SeaOrmPostgres::new(database_url))
    .await?;
```

That standalone call does not register the value or attach a lifecycle hook.
The imported `DatabaseCauldron` is the supported path for full FURNACE integration.

## Configuration

The standard module reads this namespace:

```toml
[persistence.seaorm]
url = "${DATABASE_URL}"
min_connections = 2
max_connections = 20
connect_timeout_seconds = 8
acquire_timeout_seconds = 8
idle_timeout_seconds = 600
max_lifetime_seconds = 1800
sqlx_logging = false
```

Only `url` is required. Every optional setting that is absent leaves the
corresponding SeaORM default unchanged. Numeric durations are whole seconds.
Zero connection counts and invalid relationships such as
`min_connections > max_connections` fail configuration validation before
lifecycle startup.

The internal typed configuration uses `Secret<String>` for the URL. It may
expose the URL only while creating `ConnectOptions`. `Debug`, `Display`, FURNACE
diagnostics, inspection output, and startup summaries must never contain the
URL, username, password, query parameters, or interpolated environment value.

Conventional startup retains the existing source order:

```text
optional .env interpolation
  -> optional furnace.toml
  -> FURNACE_* environment overrides
  -> typed persistence configuration
```

For example, `FURNACE_PERSISTENCE__SEAORM__MAX_CONNECTIONS` overrides
`persistence.seaorm.max_connections` under the existing configuration rules.

## Cauldron registration and application startup

The public connector module is namespaced by ORM:

```rust
use furnace_rs_persistence::sea_orm::DatabaseCauldron;
```

The namespaced path leaves room for other connectors in future releases
without creating one runtime enum for incompatible native database types.

`DatabaseCauldron` is global:

```rust
#[furnace_rs_core::cauldron]
pub struct DatabaseCauldron;
impl furnace_rs_core::Cauldron for DatabaseCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.provide::<DatabaseFactory>()
            .provide::<SeaOrmPostgres>()
            .provide::<DatabaseConnection>()
            .export::<DatabaseConnection>()
            .global()
    }
}
```

Importing the module is the explicit opt-in that selects and constructs the
connection. Once the root imports it, its explicitly exported native connection is visible to every reachable cauldron.
The factory and connector remain private.

A complete application entry point is:

```rust
use furnace-rs::prelude::*;
use furnace_rs_persistence::sea_orm::DatabaseCauldron;

mod users {
    use furnace-rs::prelude::*;
    use furnace_rs_persistence::sea_orm::{
        self,
        DatabaseConnection,
        EntityTrait,
    };

    use crate::entities::user;

    #[storage]
    pub struct UserRepository {
        db: DatabaseConnection,
    }

    impl UserRepository {
        pub async fn find_all(
            &self,
        ) -> Result<Vec<user::Model>, sea_orm::DbErr> {
            user::Entity::find().all(&self.db).await
        }
    }

    #[cauldron]
    pub struct UserCauldron;
    impl Cauldron for UserCauldron {
        fn register(self) -> CauldronRegistration<Self> {
            self.provide::<UserRepository>()
        }
    }
}

use users::UserCauldron;

#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(DatabaseCauldron).import(UserCauldron)
    }
}

#[furnace-rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

The repository receives the native SeaORM pool handle. It can use all ordinary
SeaORM entity, query, transaction, and connection traits. No `.run(...)`, FURNACE
query closure, or adapter method is required.

Conceptually, `DatabaseCauldron` contains these providers:

```rust
#[element]
fn database_factory() -> DatabaseFactory {
    DatabaseFactory
}

#[element]
fn sea_orm_postgres_connector(
    config: Config,
) -> Result<SeaOrmPostgres, PersistenceError> {
    SeaOrmPostgres::from_config(&config)
}

#[element(lifecycle)]
pub async fn sea_orm_database(
    factory: DatabaseFactory,
    connector: SeaOrmPostgres,
) -> Result<LifecycleResource<DatabaseConnection>, PersistenceError> {
    let database = factory.provide(connector).await?;

    Ok(
        LifecycleResource::new(database)
            .with_infrastructure_hook(
                "furnace-rs.persistence.seaorm.postgres",
                SeaOrmLifecycle,
            ),
    )
}
```

The graph-visible output of `sea_orm_database` is
`sea_orm::DatabaseConnection`, not `LifecycleResource<DatabaseConnection>`.
The wrapper exists only as a construction contribution understood by the
provider macro and builder.

## One default connection in v1

FURNACE identifies providers by concrete Rust `TypeId`. Two unqualified
`DatabaseConnection` providers are therefore duplicates, which is desirable
for the default contract: an application gets exactly one unambiguous native
SeaORM connection.

Multiple named databases are deferred. A later design may introduce explicit
newtypes such as `PrimaryDatabase` and `AnalyticsDatabase`, each containing a
native `DatabaseConnection`. It must not introduce string-based resolution or
silently pick one duplicate connection.

## New lifecycle-provider support in `furnace-rs-core`

### Current limitation

Today, ordinary providers return only an `ErasedProvider`. Lifecycle hooks are
registered directly on `FurnaceBuilder` or supplied by the synchronous official
auto-configuration path. An async provider can create a SeaORM connection, but
it cannot contribute the hook that should ping and close that connection.

Using only a normal async provider would still close the pool eventually when
the application context is dropped, but it would not make readiness and
orderly shutdown explicit FURNACE lifecycle operations. A builder-only extension
could register the hook, but it would not work with the standard
`Furnace::burn::<AppCauldron>()` path requested by this design.

### Provider contribution

`furnace-rs-core` will generalize the erased constructor result:

```rust
pub(crate) struct ProviderContribution {
    provider: ErasedProvider,
    lifecycle: Vec<LifecycleRegistration>,
}
```

`ProviderFuture` will return `Result<ProviderContribution>` internally.
Existing `#[element]`, `#[burner]`, and `#[storage]` declarations continue
to return their current Rust values; generated code wraps them in a
contribution with no lifecycle registrations. This is source-compatible for
application code.

The builder construction loop becomes:

1. invoke the provider constructor;
2. split its value and lifecycle registrations;
3. insert the native value under the descriptor's existing output `TypeId`;
4. add contributed hooks to the lifecycle manager;
5. continue construction in dependency order.

If later provider construction fails, lifecycle startup never begins and all
already constructed values are dropped with the failed builder. No listener is
bound.

### Lifecycle resource declaration

Providers that create lifecycle-owned infrastructure use an explicit marker:

```rust
#[element(lifecycle)]
async fn resource(...) -> Result<LifecycleResource<T>, Error>;
```

The macro requires exactly one `LifecycleResource<T>` success value and emits
provider metadata for `T`. Consumers therefore depend on `T`; the wrapper never
appears in the application graph or `ApplicationContext`.

The proposed core type is:

```rust
pub struct LifecycleResource<T> {
    value: T,
    registrations: Vec<LifecycleRegistration>,
}

impl<T> LifecycleResource<T> {
    pub fn new(value: T) -> Self;

    pub fn with_infrastructure_hook<H>(
        self,
        owner: &'static str,
        hook: H,
    ) -> Self
    where
        H: LifecycleHook + 'static;

    pub fn with_application_hook<H>(self, hook: H) -> Self
    where
        H: LifecycleHook + 'static;
}
```

Infrastructure ownership is explicit so framework resources still start
before application hooks and stop after application hooks. Owner identifiers
must be stable static strings. The SeaORM/PostgreSQL owner is
`furnace-rs.persistence.seaorm.postgres`.

This API is general core infrastructure. It contains no database concepts and
can later support clients, consumers, schedulers, or background resources.

### Lifecycle ordering

The established FURNACE lifecycle rules remain authoritative:

```text
construct complete provider graph
  -> validate routes and finalize router configuration
  -> start infrastructure hooks in deterministic owner/registration order
  -> start application hooks in registration order
  -> bind and serve HTTP
  -> stop application hooks in reverse order
  -> stop infrastructure hooks in reverse order
```

For the SeaORM connector:

1. provider construction calls `sea_orm::Database::connect`;
2. lifecycle startup calls `DatabaseConnection::ping`;
3. only successful lifecycle startup permits listener binding;
4. lifecycle shutdown calls `DatabaseConnection::close_by_ref`;
5. dropping `DatabaseConnection` remains the fallback cleanup path.

The hook resolves `DatabaseConnection` from `ApplicationContext`, so it uses
the same registered native instance as application repositories. It does not
store a second pool or reconnect.

### Failure behavior

Connection construction failures are provider-construction failures. They stop
the build before any lifecycle hook or listener starts. The ordinary FURNACE
provider diagnostic retains `PersistenceError` as its source.

A failed startup ping is a lifecycle startup failure. The lifecycle manager:

- stops already-started hooks in reverse order;
- never binds the HTTP listener;
- returns the existing `FURNACE011` lifecycle diagnostic with the safe persistence
  error retained as its source.

A shutdown close failure does not prevent later hooks from being attempted.
The first shutdown failure remains the primary error under current lifecycle
rules. SeaORM connection drop remains a best-effort fallback after the
application is released.

The provider that fails during its own startup is responsible for cleaning up
any partially started external activity before returning an error. SeaORM's
`ping` creates no separately managed activity, so no special rollback is
needed for this connector.

## Error and redaction contract

`furnace-rs-persistence` exposes:

```rust
pub struct PersistenceError { /* redacted context and retained source */ }
pub type PersistenceResult<T> = Result<T, PersistenceError>;
pub const FURNACE140: DiagnosticCode = DiagnosticCode::new("FURNACE140");
```

`PersistenceError` converts into `furnace_rs_core::Error`. This allows fallible
connector providers and lifecycle hooks to retain the persistence cause while
participating in the existing `FURNACE006` provider-construction and `FURNACE011`
lifecycle diagnostic boundaries.

Errors have stable safe categories, including:

- invalid connector configuration;
- unsupported database URL scheme;
- connection establishment failure;
- readiness failure;
- graceful close failure.

Public `Display` and `Debug` output may name the connector, backend, operation,
and safe category. They must not include the URL, credentials, SQL text,
bindings, entity values, or arbitrary driver error messages. The original
SeaORM `DbErr` remains available through `std::error::Error::source` for
server-side diagnostics, subject to the same rendering discipline used by
other FURNACE internal sources.

No blanket conversion from `DbErr` to an HTTP response is added. Applications
own domain and delivery mappings.

## Native SeaORM behavior

The injected connection is the exact native type:

```rust
sea_orm::DatabaseConnection
```

This guarantees that normal SeaORM examples remain applicable:

```rust
let users = user::Entity::find().all(&database).await?;

let user = user::Entity::find_by_id(id).one(&database).await?;

let inserted = user::ActiveModel {
    name: Set("Ada".to_owned()),
    ..Default::default()
}
.insert(&database)
.await?;

database
    .transaction::<_, (), sea_orm::DbErr>(|transaction| {
        Box::pin(async move {
            // ordinary SeaORM transaction work
            Ok(())
        })
    })
    .await?;
```

`furnace-rs-persistence` must not define parallel entity, relation, index, active
model, query, or transaction traits.

## Inspection and observability

FURNACE graph output exposes:

- the native provider type name;
- provider ownership by the global `DatabaseCauldron`;
- the ordinary provider origin already supported by the core graph.

Inspection must never establish a real database connection. The existing FURNACE
inspection path performs graph analysis without provider construction, and the
connector must preserve that boundary.

Inspection consequently reports no live readiness result. Readiness belongs to
runtime lifecycle startup, not static graph analysis. Adding public lifecycle
owner metadata to inspection output is outside the first connector release.

SQL logging remains a SeaORM/SQLx option. FURNACE does not log queries itself.
When enabled, SQL logging follows SeaORM behavior and is outside FURNACE error
normalization; documentation must warn applications not to enable verbose SQL
or binding logs when values are sensitive.

## Compatibility and versioning

`furnace-rs-persistence` is released independently but declares an explicit
compatible FURNACE core range. Its first release targets the lifecycle-provider
contract introduced for FURNACE 0.9 and Rust 1.94.

The initial SeaORM line is 2.0. SeaORM 2.0 also declares Rust 1.94, aligning it
with the FURNACE 0.9 baseline. The connector should use a compatible 2.0 version
range and test the minimum resolved dependency accepted by its manifest.

Compatibility promises are:

- patch releases may expand compatible SeaORM patch versions;
- changing to a new SeaORM major version requires a documented
  `furnace-rs-persistence` compatibility release;
- the re-exported SeaORM version is part of the connector's public type
  contract;
- a FURNACE core lifecycle API incompatibility requires a corresponding
  `furnace-rs-persistence` release;
- `furnace-rs` and `furnace-rs-persistence` do not need identical package versions, but
  their declared Cargo dependency ranges must resolve to one compatible
  `furnace-rs-core` type identity.

## Testing strategy

### `furnace-rs-core`

- ordinary providers still produce the same graph and registry values;
- `#[element(lifecycle)]` exposes `T`, not `LifecycleResource<T>`;
- malformed lifecycle provider signatures fail through `trybuild`;
- contributed infrastructure hooks start before application hooks;
- shutdown remains reverse order;
- a later construction failure starts no contributed hook;
- startup failure rolls back earlier contributed hooks;
- shutdown attempts every hook and retains the first failure;
- inspection does not run constructors or hooks.

### `furnace-rs-persistence`

- configuration parsing validates required and related fields;
- every debug and error representation redacts a sentinel URL and password;
- non-PostgreSQL URLs are rejected before connection;
- `DatabaseFactory::provide` returns a native `DatabaseConnection`;
- importing `DatabaseCauldron` contributes exactly one global native provider;
- repositories can inject and use that provider without a wrapper;
- duplicate native connection providers retain the core duplicate diagnostic;
- graph inspection requires no live PostgreSQL server.

### PostgreSQL integration

- startup connects and pings before listener binding;
- invalid credentials prevent listener binding;
- a native SeaORM entity can insert, find, update, and delete through the
  injected connection;
- native transactions commit and roll back correctly;
- orderly application shutdown closes the pool;
- a forced operation failure still attempts shutdown;
- no credential appears in captured diagnostics or logs owned by FURNACE.

CI runs unit, macro, documentation, and graph tests without PostgreSQL. A
separate PostgreSQL service job runs ignored integration tests, following the
existing repository pattern.

## Acceptance criteria

The design is complete when all of these behaviors are implemented and tested:

1. A new application can depend on `furnace-rs-persistence` separately.
2. `DatabaseCauldron` can be imported directly by a root `AppCauldron`.
3. `Furnace::burn::<AppCauldron>()` constructs the configured PostgreSQL connection.
4. A repository field of type `sea_orm::DatabaseConnection` resolves through
   ordinary FURNACE DI.
5. Native SeaORM query and transaction APIs work without a FURNACE wrapper.
6. Graph inspection does not connect to PostgreSQL.
7. Connection or readiness failure occurs before listener binding.
8. Graceful shutdown explicitly closes the SeaORM pool.
9. Credential-bearing configuration is redacted from FURNACE-owned output.
10. Applications that do not import `DatabaseCauldron` perform no persistence
    initialization.
11. Existing providers and lifecycle hooks retain their behavior.
12. Stable and Rust 1.94 CI gates pass.

## Future connectors

Future ORM support follows the same boundary:

```text
furnace_rs_persistence::<orm>::DatabaseCauldron
  -> ORM-specific connector configuration
  -> native ORM pool/client type in DI
  -> readiness and shutdown lifecycle contribution
```

Each connector owns its features, native output type, configuration namespace,
and lifecycle adapter. The shared crate may reuse `DatabaseFactory`, connector
error normalization, and generic lifecycle-provider support, but it must not
force unrelated ORMs behind one runtime trait object.

Potential later work includes MySQL/SQLite backends, additional ORMs, named
connection newtypes, opt-in migration hooks, and connector-specific health
details. Each requires a separate design and must preserve the native-instance
principle established here.

## External references

- [SeaORM database connections](https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/)
- [SeaORM crate features](https://docs.rs/crate/sea-orm/latest/features)
- [SeaORM migration setup](https://www.sea-ql.org/SeaORM/docs/migration/setting-up-migration/)
