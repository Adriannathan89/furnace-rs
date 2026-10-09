# furnace-rs

[![Latest release](https://img.shields.io/github/v/release/Adriannathan89/furnace-rs?display_name=tag&sort=semver)](https://github.com/Adriannathan89/furnace-rs/releases/latest)
[![CI](https://github.com/Adriannathan89/furnace-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Adriannathan89/furnace-rs/actions/workflows/ci.yml)

furnace-rs 1.0.1 is a Rust application framework with a framework-neutral
core, a scoped Axum HTTP runtime, source-aware typed configuration, safe REST
errors, request validation, and opt-in native SeaORM persistence. A root
module selects one application; startup validates its scoped graph and routes
before it starts lifecycle hooks, checks a database, or binds a socket.

## Released version: 1.0.1

**Release date: 2026-10-06.**

**1.0.1 is the current released version.** Install the released CLI with:

```sh
cargo install furnace-rs-cli --version 1.0.1 --locked
```

To use this branch's implementation, install from the checkout with
`cargo install --path crates/furnace-rs-cli --locked`. The workspace and local
examples target 1.0.1, but this branch also contains changes listed under
**Unreleased** in the changelog: debug request logging, an expanded startup
route summary, validated-extractor error redaction, opt-in SeaORM HTTP error
conversion, and simpler CRUD examples. Descriptions of those
changes below refer to the checkout, rather than the published 1.0.1 packages.

See the [documentation index](docs/README.md), [changelog](CHANGELOG.md),
[migration from MADS 0.x](docs/importance/furnace-rs-migration.md),
[release notes and verification](docs/releases/1.0.1.md),
[security policy](docs/SECURITY.md), and [security audit](docs/SECURITY_AUDIT.md).
Version 1.0.1 adds HTTP header/body deadlines, validates database maintenance
intervals, and rejects mismatched inventory outputs during construction.
The release includes strict Bearer parsing, safe core error formatting, checked
database timeouts, protected connection tracing, and owner-only Unix CLI
control directories. The audit documents reproduction, fixes, benchmark
evidence, prior real PostgreSQL validation, and the limits of the review.

## What is furnace-rs?

The furnace metaphor describes startup: a `Cauldron` groups explicitly registered
components, and `Furnace::burn` starts the selected application. Services use
`#[burner]` and repositories use `#[storage]`; both generate `Injector` constructors.
Plain services can implement `Injector` directly.

The philosophy is to take that frustration out of Rust application
development. FURNACE keeps architecture explicit, typed, and inspectable while
automating the repetitive work around cauldrons, dependency wiring, lifecycle,
configuration, routing, and infrastructure. Developers can then spend more
time on domain logic and business systems instead of rebuilding the same
low-level application structure for every project.

## CLI quick start

Create a minimal HTTP application, then start its development server:

```bash
furnace new my-app
cd my-app
furnace dev
```

`furnace new` creates exactly `Cargo.toml`, `furnace.toml`, `src/main.rs`, and
`src/app/{mod,controller,service}.rs`. The generated application has
only the `http` and `runtime-tokio` FURNACE features—no database, JWT, cookie,
migration, or authentication setup—and answers `GET /` with `Hello World!`.
The application package starts at `0.1.0`; its FURNACE dependency is pinned to the
installed CLI version. See [the CLI reference](docs/CLI.md) for its atomic,
offline generator contract, naming rules, exact JSON output, and non-goals.

From an existing project, inspect or run a selected application:

```bash
furnace doctor
furnace routes
furnace run
```

See the [authoritative CLI reference](docs/CLI.md) for target selectors,
forwarded application arguments, diagnostics, watcher behavior, inspection
limits.

The [four independent example projects](example/) use local workspace crates.
Run commands from each example directory so configuration loads there.

| Example | Implementation | Routes | Port |
| --- | --- | --- | ---: |
| [Hello World](example/hello-world/) | Public controller, root registration, debug request logging | `GET /` | 3000 |
| [Posts CRUD](example/posts-crud/) | Native SeaORM PostgreSQL connection, controller/repository, validated input | `POST /posts`, `GET /posts`, `GET /posts/{id}`, `PUT /posts/{id}`, `DELETE /posts/{id}` | 3001 |
| [Posts CRUD with service](example/post-crud-with-service/) | The same CRUD routes with an optional service layer | `POST /posts`, `GET /posts`, `GET /posts/{id}`, `PUT /posts/{id}`, `DELETE /posts/{id}` | 3001 |
| [Protected route](example/protected-route/) | Trait bindings, in-memory repository, validated login, JWT strategy, `reader` guard, application logger | `POST /auth/login`, `GET /auth/me` | 3002 |

Start the smallest example with:

```sh
cd example/hello-world
cargo run --locked
# In another terminal:
curl http://127.0.0.1:3000/
# Hello, world!
```

Posts CRUD requires PostgreSQL and applying its SQL migration with `psql` before
startup. For the protected-route example, copy `.env.example` to `.env` first.
Its plaintext demo credentials illustrate wiring; applications own password
hashing and session policy. See the example guides for setup and requests.
For repeatable HTTP load and failure checks, see the [benchmark suite](benchmark/).

## Standard application

```rust,no_run
use furnace_rs::prelude::*;

#[controller]
struct HelloController;
#[controller(route = "/")]
impl HelloController {
    #[get]
    fn hello(&self) -> &'static str { "Hello, world!" }
}

#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.controller::<HelloController>()
    }
}

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

`Furnace::burn` starts the root cauldron and its imports. Register dependencies with
`.provide::<T>()`, controllers with `.controller::<T>()`, and imported cauldrons
with `.import(OtherCauldron)`. Cross-cauldron injection requires `.export::<T>()`;
`.global()` exposes only those exports throughout the reachable application.
For trait or third-party outputs, select an implementer with
`.provide_with::<Arc<dyn Trait>, Implementer>()`. Plain services implement
`Injector`, declaring typed tuple dependencies and an async `inject` constructor;
the managed service/repository macros generate this contract automatically.
Lifecycle constructors override `Injector::lifecycle` to attach resource hooks.
Rust namespaces and `pub` visibility do not determine DI membership.

Controllers without `impl Sealable` are public. Attach a typed `#[guard]` through
`Sealable::seals` to protect a controller. `#[seal(skip)]` explicitly bypasses
that guard for an endpoint.

See the [breaking-change migration guide](docs/importance/furnace-rs-migration.md)
for the `cauldron`, `burner`, `storage`, and `Injector` APIs and explicit output bindings.

## Workspace crates

FURNACE is split into small crates with a deliberate dependency direction. Most
applications depend only on the public `furnace-rs` facade; the implementation details
are documented beside the crate that owns them.

~~~text
application
└── furnace-rs
    ├── furnace-rs-core
    │   └── furnace-rs-core-macros
    ├── furnace-rs-common (optional)
    │   ├── furnace-rs-core
    │   └── furnace-rs-common-macros
    └── furnace-rs-extra (optional)
        └── furnace-rs-core

application (opt-in persistence)
└── furnace-rs-persistence
    └── furnace-rs-core

application tests
└── furnace-rs-testing
    ├── furnace-rs-core
    └── furnace-rs-common

furnace-rs-cli
├── furnace-rs
└── furnace-rs-common (http-only private inspection contract)
~~~

| Crate | Responsibility | Contributor guide |
| --- | --- | --- |
| `furnace-rs` | Public facade, prelude, and feature composition for application authors. | [crates/furnace-rs/README.md](crates/furnace-rs/README.md) |
| `furnace-rs-core` | Framework-neutral configuration, graph, providers, lifecycle, diagnostics, and module scope. | [crates/furnace-rs-core/README.md](crates/furnace-rs-core/README.md) |
| `furnace-rs-core-macros` | Procedural macros that generate core metadata and constructors. | [crates/furnace-rs-core-macros/README.md](crates/furnace-rs-core-macros/README.md) |
| `furnace-rs-common` | Optional HTTP, validation, CORS, JWT, cookie, and Passport integrations. | [crates/furnace-rs-common/README.md](crates/furnace-rs-common/README.md) |
| `furnace-rs-persistence` | Explicit native SeaORM PostgreSQL connector and lifecycle integration. | [crates/furnace-rs-persistence/README.md](crates/furnace-rs-persistence/README.md) |
| `furnace-rs-common-macros` | Procedural macros for routes, controllers, validation, and Passport. | [crates/furnace-rs-common-macros/README.md](crates/furnace-rs-common-macros/README.md) |
| `furnace-rs-cli` | Cargo-native execution, inspection, development loop, and scaffolding. | [crates/furnace-rs-cli/README.md](crates/furnace-rs-cli/README.md) |
| `furnace-rs-testing` | Focused service and in-process controller fixtures with scripted SeaORM mocks. | [crates/furnace-rs-testing/README.md](crates/furnace-rs-testing/README.md) |
| `furnace-rs-extra` | Reserved boundary for future optional integrations. | [crates/furnace-rs-extra/README.md](crates/furnace-rs-extra/README.md) |

The approach is type-driven and metadata-driven: macros emit static
descriptors, core analyzes a selected module graph before construction, and
common integrations consume the validated application. See
[Architecture](docs/ARCHITECTURE.md) for invariants and the individual crate
guides for dependencies, source layout, and change ownership.

## Installation and feature selection

~~~toml
[dependencies]
furnace-rs = { version = "=1.0.1" }
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
~~~

furnace-rs supports Rust 1.94 and uses Rust edition 2024. The default facade
enables HTTP and application logging with the Tokio runtime. The dependency
uses the package name `furnace-rs`; Rust imports use `furnace_rs::...`,
including `use furnace_rs::prelude::*;`, as in the runnable examples.

| Facade feature | Enables |
| --- | --- |
| `http` | Axum routing/server, controllers, extractors, validation, REST errors, and CORS |
| `logger` | Injectable application logging; import `LoggerCauldron` for the default console logger |
| `jwt` | JWT signing/verification; Passport guards and strategies when combined with `http` |
| `cookies` | Cookie extraction and response cookies; implies `http` |
| `sea-orm` | Converts SeaORM `DbErr` into redacted HTTP 500 errors through `?`; implies `http` |
| `runtime-tokio` | Tokio entry-point support for `#[furnace_rs::main]` |
| `common` | Convenience feature enabling `http` and `logger` |
| `extra` | Reserved integration boundary |

Use `default-features = false` to select integrations explicitly. Persistence
and focused testing are separate dependencies. See the
[facade README](crates/furnace-rs/README.md) for feature boundaries.

## Conventional configuration and HTTP

Only `Furnace::burn` loads conventional configuration. It reads the process
current working directory in this order:

1. optional `.env`, used only for interpolation;
2. optional `furnace.toml` as ordinary configuration;
3. final scalar `FURNACE_*` environment overrides.

Process variables win during `${NAME}` interpolation, dotenv loading never
mutates the process environment, `FURNACE_SERVER__HOST` maps to `server.host`, and
`FURNACE_SERVER__PORT` maps to `server.port`. Both files may be absent; a present
unreadable or malformed file is a bootstrap failure. FURNACE does not search
parent directories or `CARGO_MANIFEST_DIR`.

```toml
# furnace.toml
[server]
host = "127.0.0.1" # default
port = 3000        # default

[server.cors]
origins = ["https://app.example.com"]
methods = ["GET", "POST"]
allowed_headers = ["authorization", "content-type"]
exposed_headers = ["x-request-id"]
credentials = false
max_age_seconds = 600
```

Enable HTTP request logging in this checkout (an Unreleased change) with:

```toml
[furnace]
mode = "debug"
```

Debug mode writes one line to standard output when each response is ready,
including the local timestamp with milliseconds, final status code, HTTP method,
and request path (without its query string). Columns have fixed widths:

```text
[debug] 2026-10-08 14:32:05.123 | 200 | GET     | /users
[debug] 2026-10-08 14:32:06.456 | 404 | DELETE  | /users/42
```

Logging is disabled when `furnace.mode` is absent or differs from `debug`.
`FURNACE_FURNACE__MODE=debug` can override the TOML value through the standard
configuration loader. Low-level `serve` and `serve_router` use the configuration
already supplied to the application builder, and honor the same setting.
Request logging is included in the `http` feature.

Wildcard-capable CORS fields use a scalar, not a one-element list:

```toml
[server.cors]
origins = "*"
methods = ["GET", "POST"]
allowed_headers = "*"
exposed_headers = "*"
```

CORS is opt-in, validated before lifecycle startup, and applied as the
outermost layer to both generated and native routes. Wildcard origins or
headers cannot be combined with credentials. It is a browser response-access
policy, not authorization or CSRF protection.

Use the tracked [`.env.example`](.env.example) as a local template, copy it to
the ignored `.env`, and put real secrets in process variables in CI and
production.

The serving APIs bound incomplete initial requests and HTTP/1 headers to ten
seconds. Pending request-body reads have a ten-second idle deadline renewed by
progress. A stalled body returns a safe 408 before response headers are sent;
a timeout during response streaming terminates the stream. These deadlines do
not impose a handler execution limit. HTTP/2 and upgrades remain supported;
there is no built-in TLS configuration.

## Validated requests and REST errors

Use `#[derive(serde::Deserialize, Input)]` with `ValidatedJson<T>`,
`ValidatedQuery<T>`, or `ValidatedPath<T>` to deserialize, validate, attach a
`body`, `query`, or `path` source, and invoke a handler only on valid input.

```rust,no_run
use furnace_rs::prelude::*;

#[derive(serde::Deserialize, Input)]
struct CreateUser {
    #[validate(email, length(max = 254))]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[controller]
struct UserController;
#[controller(route = "/users")]
impl UserController {
    #[post]
    async fn create(&self, _body: ValidatedJson<CreateUser>) -> HttpResult<&'static str> {
        Ok("created")
    }
}
```

The built-ins are `email`, `length(min = N)`, `length(max = N)`,
`length(exact = N)`, `nonempty`, `range(min = N)`, `range(max = N)`,
`positive`, `negative`, `multiple_of = N`, `required`, `nested`, and
`custom = path`. They cover supported strings, numbers, `Option`, structs,
enums, tuples, arrays, `Vec`, and string-keyed `HashMap`/`BTreeMap` shapes;
string length is Unicode code-point length. Derived callbacks can report one or
many relative `ValidationIssue`s, and applications may implement `Input`
manually for complete control. Validators run in source order; nested values
follow declaration, index, and lexical map-key order.

Email follows the practical default Zod syntax policy on the unmodified
string; FURNACE does not trim, normalize, perform DNS checks, or claim full-RFC
mailbox validation. Numeric bounds are inclusive, while `positive` and
`negative` are strict.

```rust,ignore
fn validate_username(value: &str) -> ValidationResult {
    if value == "root" {
        Err(ValidationErrors::from_issue(
            ValidationIssue::custom("reserved_username", "username is reserved"),
        ))
    } else {
        Ok(())
    }
}

// Use #[validate(custom = validate_username)] on a field, a whole-value
// callback on the derived type, or implement Input manually for full control.
```

Custom issue paths are relative; derive-generated nesting prefixes external
Serde field/variant names or collection indices. Body/query/path source is
attached only by the validated extractor, so transport-independent manual and
derived implementations share the same HTTP boundary.

Validation returns status 422 with the fixed safe envelope:

```json
{
  "error": {
    "code": "validation_error",
    "message": "input validation failed",
    "issues": [{
      "source": "body",
      "path": ["email"],
      "code": "invalid_format",
      "message": "invalid email address"
    }]
  }
}
```

Serde conversion remains authoritative and can report its first conversion
issue; after successful deserialization FURNACE aggregates independent validation
issues. Rejected values never appear in built-in issues. Validated JSON keeps
415 unsupported-media-type and 413 body-limit semantics in the standard error
envelope.

Native `Json<T>`, `Query<T>`, and `Path<T>` remain the unmodified Axum
extractors and deliberately do not run `Input`. Use them when an application
needs its own extraction or validation policy; do not rename a native `Json`
alias and present it as validation.

The `http` feature exports the seven standard errors: `BadRequest` (400),
`Unauthorized` (401), `Forbidden` (403), `NotFound` (404), `Conflict` (409),
`ValidationError` (422), and `InternalError` (500). They use one
`{ "error": { "code", "message" } }` envelope; validation alone adds ordered
issues. Internal errors always render `internal server error` and retain their
source only for server-side error chaining. FURNACE-owned Passport and cookie
failures use this envelope; Passport authentication rejection retains
`WWW-Authenticate: Bearer`. Native Axum responses remain native.

| Type | Status | Code | Message policy |
| --- | ---: | --- | --- |
| `BadRequest` | 400 | `bad_request` | application-supplied safe message |
| `Unauthorized` | 401 | `unauthorized` | application-supplied safe message |
| `Forbidden` | 403 | `forbidden` | application-supplied safe message |
| `NotFound` | 404 | `not_found` | application-supplied safe message |
| `Conflict` | 409 | `conflict` | application-supplied safe message |
| `ValidationError` | 422 | `validation_error` | fixed `input validation failed` |
| `InternalError` | 500 | `internal` | fixed `internal server error` |

FURNACE fixes its own Passport messages to `authentication was rejected` or
`access was denied`, malformed cookies to `cookie request is malformed`,
unsupported validated JSON content types to
`content type must be application/json`, payload overflow to
`request body is too large`, other safe client body-read failures to
`request body could not be read`, and all server-class failures to
`internal server error`.

Database-to-HTTP error conversion remains an application delivery-policy
decision. The persistence connector returns a native SeaORM connection and
retains typed connector errors; it does not map them automatically to HTTP.

## Typed configuration and secrets

Typed configuration reads the existing loaded `Config`; it does not introduce a
new loader or global type discovery. Derive a named configuration struct and
request it explicitly through `Config::parse`:

```rust,no_run
use furnace_rs::prelude::*;

#[derive(Configuration)]
#[config(prefix = "app")]
struct AppConfig {
    #[config(rename = "bind_host")]
    host: String,
    #[config(default = 3000, validate(range(min = 1, max = 65535)))]
    port: u16,
    api_key: Secret<String>,
}

impl Injector for AppConfig {
    type Dependencies = (Config,);
    async fn inject((config,): Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(config.parse()?)
    }
}
```

Supported fields are strings, booleans, characters, finite numeric primitives,
source-relative `PathBuf`, `Option<T>`, `Secret<T>`, `Option<Secret<T>>`,
`Vec<String>`, nested `Configuration`, and a scalar `parse_with` callback.
Prefixes and `rename` compose dotted keys; defaults and compatible validators
are checked at compile time. Missing, parse, and validation failures aggregate
in declaration order with full keys, stable codes, and winning-source labels,
never configured values.

A `parse_with` callback receives `&str` and returns `Result<FieldType, E>`;
arbitrary parser error text is discarded so it cannot leak an input. Options
become `None` only when absent, present invalid values never fall back to a
default, and secrets cannot have source-code defaults. Maps, non-string
vectors, arbitrary arrays, inline tables, arrays of tables, and TOML datetimes
remain outside the existing flattened `Config` shape.

The provider makes parsing a startup requirement only when the selected graph
uses it: failure occurs before lifecycle startup and listener binding. The
conventional source order is unchanged: optional `.env` for interpolation,
optional `furnace.toml`, then final scalar `FURNACE_*` overrides. Only an entire
`${NAME}` scalar/array element is interpolated; process variables win over
dotenv, and dotenv is not a configuration source. `Secret<T>` exposes a value
only through `.expose()` or `.into_exposed()`; ordinary `Display` and `Debug`
always print `[REDACTED]`.

## Low-level builder

Use the builder when configuration, hooks, binding, or router
composition must be explicit. It never loads `.env`, `furnace.toml`, or `FURNACE_*`
on its own. The explicit address overrides `[server]` binding and may use port
zero; merge native Axum routes before passing the raw router to `serve_router`.

```rust,ignore
let mut builder = Furnace::builder_with_config(config);
builder.root::<AppCauldron>()?;
// builder.lifecycle_hook(MyHook);
let application = builder.build().await?;
let router = build_router(&application)?.merge(native_router);
serve_router(application, router, "127.0.0.1:0").await?;
```

For direct in-process router use, call `configure_router(&application, router)`
after the merge. A builder without `root::<AppCauldron>()` intentionally retains
the complete-catalog compatibility behavior.

## Native database provisioning

Database provisioning comes from the separate persistence crate. Add the connector
explicitly and import its global module in your application root:

```toml
furnace-rs-persistence = { version = "1.0.1", features = ["sea-orm-postgres"] }
```

In this checkout, the opt-in `sea-orm` feature on `furnace-rs` adds
`From<sea_orm::DbErr> for HttpError`. HTTP handlers returning `HttpResult<T>` can
use `repository.list().await?` directly. Database failures produce a redacted
500 JSON envelope and preserve the original source for diagnosis. The feature
does not select a database driver or provision a connection. See the
[simple CRUD example and optional service variant](example/posts-crud/).

```rust,ignore
use furnace_rs_persistence::sea_orm::{DatabaseConnection, DatabaseCauldron};

#[furnace_rs::cauldron]
struct AppCauldron;
impl furnace_rs::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs::CauldronRegistration<Self> {
        self.import(DatabaseCauldron).provide::<UserRepository>()
    }
}

impl furnace_rs::Injector for UserRepository {
    type Dependencies = (DatabaseConnection,);
    async fn inject((database,): Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(Self::new(database))
    }
}
```

For an explicit connection, `DatabaseFactory::provide` returns the native
`DatabaseConnection` on success or a typed `PersistenceError` on failure:

```rust,ignore
use furnace_rs_persistence::{DatabaseFactory, PersistenceResult};
use furnace_rs_persistence::sea_orm::{DatabaseConnection, SeaOrmPostgres};

async fn connect(url: String) -> PersistenceResult<DatabaseConnection> {
    DatabaseFactory.provide(SeaOrmPostgres::new(url)).await
}
```

The imported module checks the connection before serving and closes it on
graceful shutdown. SeaORM owns entities, queries, transactions, and migrations;
FURNACE does not run or generate migrations. See the
[persistence guide](docs/furnace-rs-persistence.md).

`serve(application, "127.0.0.1:3000")` remains the explicit generated-router
escape hatch. Its address overrides automatic server binding; use
`serve_router` when the raw generated router has been merged with native Axum
routes.

## Passport configuration and JWT profiles

`Furnace::burn` supplies the standard conventional source order; the low-level
builder stays explicit. Dotenv sources provide interpolation values, and
ordinary sources merge from first to last; a later scalar or string array
replaces an earlier value at the same key completely. Process variables override
dotenv values during `${NAME}` interpolation. `EnvSource` is scalar-only, so
arrays such as `algorithms` and `audiences` belong in TOML or a programmatic
`ConfigDocument`/`MapSource`.

```toml
# furnace.toml
[passport]
secret = "${JWT_SECRET}"
algorithms = ["HS256"]
issuer = "https://auth.example.com"
audiences = ["furnace-rs-api"]
```

Simple `secret` mode permits one HMAC algorithm: HS256 by default, or one of
HS384/HS512 when explicitly selected. Minimum secret sizes are 32/48/64 bytes.
For rotation, configure a named key ring; the active key signs and all retained
keys verify by `kid`:

```toml
[passport]
active_key = "2026-08"
algorithms = ["RS256"]

[passport.keys."2026-08"]
algorithm = "RS256"
private_key_file = "keys/current-private.pem"
public_key_file = "keys/current-public.pem"

[passport.keys."2026-07"]
algorithm = "RS256"
public_key_file = "keys/previous-public.pem"
```

FURNACE supports HS256/384/512, RS256/384/512, and ES256/384. The configured
allowlist—not an untrusted token header—selects eligible algorithms, and every
named key is bound to one algorithm. Relative paths from TOML resolve beside
that TOML file; paths from environment or programmatic sources resolve from the
process working directory.

```rust,ignore
use std::time::Duration;
use furnace_rs::prelude::*;

let access = jwt.sign(
    UserClaims { user_id: 7 },
    JwtSignOptions::access(Duration::from_secs(900)).subject("7"),
)?;
let refresh = jwt.sign(
    UserClaims { user_id: 7 },
    JwtSignOptions::refresh(Duration::from_secs(604_800)).subject("7"),
)?;
let verified_access = jwt.verify::<UserClaims>(&access, JwtValidation::access())?;
let verified_refresh = jwt.verify::<UserClaims>(&refresh, JwtValidation::refresh())?;
```

Access and refresh tokens have different protected `typ` values and
`token_use` claims. They are not interchangeable. Unverified decode APIs are
inspection-only and must never authenticate a request.

## Managed strategies, principals, and guards

A custom strategy is both a managed provider and an annotated
`PassportStrategy` implementation. Framework signature, registered-claim, and
token-kind verification always happens before `validate`; the strategy sees
verified claims and a credential-sanitized, read-only `PassportContext`.

```rust,ignore
#[derive(PassportPrincipal)]
struct UserPrincipal {
    user_id: u64,
    #[roles]
    roles: Vec<String>,
    #[permissions]
    permissions: std::collections::BTreeSet<String>,
}

#[burner]
struct AppJwtStrategy { users: UserService }

#[passport_strategy(name = "jwt")]
impl PassportStrategy for AppJwtStrategy {
    type Claims = UserClaims;
    type Principal = UserPrincipal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        context: &PassportContext<'_>,
        claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        self.users.authenticate_current(context, claims.custom.user_id).await
    }
}
```

`jwt` is the built-in access strategy and can authorize directly as
`ClaimsPrincipal<C>`. A custom `jwt` strategy overrides it. `jwt-refresh` is not
built in: applications define it with `JwtTokenKind::Refresh` and own any
persistence, rotation, reuse detection, and revocation.

```rust,ignore
fn owns_profile(principal: &UserPrincipal) -> bool { principal.user_id == 7 }

#[guard(
    strategy = "jwt", principal = UserPrincipal, source = bearer,
    roles(any = ["user", "admin"]), permissions(all = ["profile:read"]),
    predicate = owns_profile,
)]
struct UserGuard;
#[controller]
struct UserController;
impl Sealable for UserController {
    fn seals() -> SealRegistration<Self> { Self::seal::<UserGuard>() }
}
#[controller(route = "/users")]
impl UserController {
    #[get("/profile")]
    async fn profile(&self, principal: Authenticated<UserPrincipal>) -> HttpResult<&'static str> {
        let _ = principal;
        Ok("profile")
    }
}
```

Controllers are public by default without `impl Sealable`. An explicit empty
`SealRegistration::new()` also makes a controller public. One static policy
protects its controller's endpoints except methods marked `#[seal(skip)]`,
which bypass authentication and policy checks. For example:

```rust,ignore
#[post("/login")]
#[seal(skip)]
fn login(&self) -> &'static str { "public login" }
```

Use this method inside the annotated controller implementation. Skipped methods
do not receive an authenticated principal from the seal. Roles, permissions, and predicates are ANDed;
`any`/`all` controls matching inside each clause. Every predicate must be a
synchronous `fn(&UserPrincipal) -> bool`. A guard uses exactly one source.
With `cookies`, select `source = cookie("refresh_token")`.

Authentication and strategy rejection map to generic `401 Unauthorized` with
`WWW-Authenticate: Bearer`, authorization policy failures to `403 Forbidden`,
and operational failures to `500 Internal Server Error`. Ordinary malformed
cookie extraction remains `400 Bad Request`; a missing, malformed, or duplicate
guard cookie is a generic `401`.

Cookie jars compose with response tuples and emit checked `Set-Cookie` headers:

```rust,ignore
let cookie = Cookie::build(("refresh_token", refresh))
    .path("/")
    .http_only(true)
    .secure(true)
    .same_site(SameSite::Strict)
    .max_age(cookie::time::Duration::days(7))
    .build();
Ok((jar.add(cookie), Json(response)))
```

For native Axum routes, apply a typed `PassportGuard<P>` Tower layer. This is a
runtime escape hatch, not static FURNACE guard metadata, so it cannot activate JWT
auto-configuration. Before `PassportGuard::build()`, a managed provider must
directly require `JwtService`, or the builder must explicitly provide a
concrete `JwtService`; otherwise construction fails with `FURNACE131`.

See the [current migration guide](docs/importance/furnace-rs-migration.md) and the
[runnable protected-route example](example/protected-route/).

## A typed HTTP route

`#[controller]` declares a managed struct and its inherent endpoint implementation.
Generated adapters resolve the controller once while building the router and
call its Rust methods directly. Handlers use native Axum extractors.

```rust,no_run
use furnace_rs::prelude::*;

#[derive(Clone, serde::Serialize)]
struct User {
    id: u64,
}

#[controller]
struct UserController;
#[controller(route = "/readme-users")]
impl UserController {
    #[get("/:id")]
    async fn get_user(&self, Path(id): Path<u64>) -> HttpResult<Json<User>> {
        Ok(Json(User { id }))
    }
}

#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> { self.controller::<UserController>() }
}

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

## Extractors, responses, and routing

The prelude exports `Path<T>`, `Query<T>`, `Json<T>`, `Header<T>`, `Request`,
`HttpResult<T>`, `Created<T>`, `NoContent`, `build_router`, `configure_router`,
`serve`, and `serve_router`.
`furnace_rs::common::axum` remains the native Axum escape hatch for extractors,
responses, routers, middleware, and Tower composition.

Endpoint attributes accept `/:id` or `/{id}` and a final `/*rest` or
`/{*rest}` wildcard. Metadata uses canonical Axum brace paths. Invalid selected
metadata and path-tree conflicts fail before provider construction. GET also handles
HEAD, OPTIONS is not synthesized, static routes win over parameter routes, and
trailing slashes remain strict. `build_router(&application)` returns the raw
generated router; merge native routes before `configure_router` or
`serve_router` applies final application-wide CORS. Use the configured router
with Tower's `ServiceExt::oneshot` for in-process route tests without binding a
listener.

## Benchmarks

The current benchmark suite covers native Axum/FURNACE throughput and
process-start-to-ready comparisons with Axum, Go/Gin, and NestJS/Fastify.

| Application | Startup P50 | Startup P95 |
| --- | ---: | ---: |
| Native Axum | 21 ms | 30 ms |
| Go/Gin | 22 ms | 29 ms |
| FURNACE | 22 ms | 30 ms |
| NestJS/Fastify | 428 ms | 443 ms |

The startup comparison uses 1,000 release-build starts per application and an
equivalent PostgreSQL readiness check. In the exploratory throughput suite,
every native Axum/FURNACE saturation range overlaps, while both sustain the fixed
1,000 requests/second target with closely grouped latency.

See [the benchmark guide](benchmark/README.md) for the complete results, methodology,
limitations, resource measurements, and interpretation guidance.

## Current scope

Version 1.0.1 includes rooted module scope, conventional startup, CORS,
native router composition, typed input validation, the seven REST errors,
explicit typed configuration and redacted secrets, focused FURNACE macro
diagnostics, Cargo-native run/dev, compiled route/graph/doctor inspection,
schema-version-2 finite-command JSON, opt-in native SeaORM persistence, and the
offline atomic minimal-project generator. It preserves the low-level builder,
the complete-catalog rootless compatibility path, native Axum extractors and
responses, ordinary human CLI output, and application-owned database policy.

Application-authored `Injector<Output>` bindings support native shared trait
objects through `.provide_with::<Output, Implementer>()`.
It does **not** implement `Inject<dyn Trait>`,
asynchronous or database-backed derive validators, automatic validation for
native extractors, full-RFC/DNS email validation, login or credential
validation, refresh endpoints or persistence/rotation/revocation, password
hashing, CSRF, remote JWKS, JWE, third-party auto-configuration, arbitrary
configuration sources/shapes, multiple-listener/TLS or configurable HTTP/2 server settings,
JSON-wrapped run/dev streams, or scaffold database/JWT/cookie/migration/Git
setup. Database errors never map automatically; applications own their
delivery policy.

## Focused tests

Add `furnace-rs-testing = "=1.0.1"` under `[dev-dependencies]`. Annotate an async,
zero-argument test function with `#[furnace_rs::test]`; Cargo runs it without a separate
Tokio dependency. The local `test_fixture()` builds one registered subject's
dependency chain without module setup.

```rust
use furnace_rs::prelude::*;

#[controller]
struct TestController;

#[controller(route = "/test")]
impl TestController {
    #[get]
    async fn hello(&self) -> &'static str { "Hello, world!" }
}

#[furnace_rs::test]
async fn controller_returns_ok() {
    test_fixture()
        .controller::<TestController>()
        .run(|client| async move {
            client.get("/test").send().await.unwrap()
                .assert_status(furnace_rs_testing::http_types::StatusCode::OK);
        })
        .await
        .unwrap();
}
```

Use `.subject::<UserService>()` and `context.resolve::<UserService>()` for direct
service tests. Database dependencies require an explicit SeaORM
`MockDatabase` configured with `DbBackend::Sqlite`; it queues scripted results and never opens a production
connection. `run` awaits lifecycle shutdown on completion and unwinding assertion
panics. See the [testing guide](crates/furnace-rs-testing/README.md) for complete service
and controller examples, supplies, and response assertions.

## Development

Run the available release checks locally:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --all-features --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo +1.94.0 test --locked --workspace --all-features
```

CI also provisions PostgreSQL 16 and runs the ignored database suites plus the
85% line-coverage gate. To run those locally, set `FURNACE_TEST_DATABASE_URL` to a
PostgreSQL 16 database and use the commands in the
[1.0.1 release verification guide](docs/releases/1.0.1.md).

## Community

Read the [contribution guide](CONTRIBUTING.md) before submitting changes.
All participants are expected to follow the [code of conduct](CODE_OF_CONDUCT.md).

## License

furnace-rs is licensed under either the [Apache License 2.0](LICENSE-APACHE) or
the [MIT License](LICENSE-MIT), at your option.
