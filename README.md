# furnace-rs

[![Latest release](https://img.shields.io/github/v/release/Adriannathan89/furnace-rs?display_name=tag&sort=semver)](https://github.com/Adriannathan89/furnace-rs/releases/latest)
[![CI](https://github.com/Adriannathan89/furnace-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Adriannathan89/furnace-rs/actions/workflows/ci.yml)

furnace-rs 0.9.2 is a Rust application framework with a framework-neutral
core, a scoped Axum HTTP runtime, source-aware typed configuration, safe REST
errors, request validation, and opt-in native SeaORM persistence. A root
module selects one application; startup validates its scoped graph and routes
before it starts lifecycle hooks, checks a database, or binds a socket.

## What is furnace-rs?

The furnace metaphor describes startup: a `Cauldron` groups explicitly registered
components, and `Furnace::burn` starts the selected application. Services use
`#[burner]`, repositories use `#[storage]`, and factories use `#[element]`.

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

For runnable FURNACE 0.9 walkthroughs, see the [three example projects](example/):
Hello World, PostgreSQL posts CRUD, and a JWT-protected route with validation
and logging.
For repeatable HTTP load and failure checks, see the [benchmark suite](benchmark/).

## Standard application

```rust,no_run
use furnace::prelude::*;

#[controller]
struct HelloController;
impl Sealable for HelloController {
    fn seals() -> SealRegistration<Self> { SealRegistration::new() }
}
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

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

`Furnace::burn` starts the root cauldron and its imports. Register dependencies with
`.provide::<T>()`, controllers with `.controller::<T>()`, and imported cauldrons
with `.import(OtherCauldron)`. Cross-cauldron injection requires `.export::<T>()`;
`.global()` exposes only those exports throughout the reachable application.
Rust namespaces and `pub` visibility do not determine DI membership.

See the [breaking-change migration guide](docs/importance/furnace-rs-migration.md)
for the `cauldron`, `burner`, `storage`, and `element` vocabulary and factory registration.

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
| `furnace-rs-testing` | Focused service and in-process controller fixtures with SeaORM SQLite mocks. | [crates/furnace-rs-testing/README.md](crates/furnace-rs-testing/README.md) |
| `furnace-rs-extra` | Reserved boundary for future optional integrations. | [crates/furnace-rs-extra/README.md](crates/furnace-rs-extra/README.md) |

The approach is type-driven and metadata-driven: macros emit static
descriptors, core analyzes a selected module graph before construction, and
common integrations consume the validated application. See
[Architecture](docs/ARCHITECTURE.md) for invariants and the individual crate
guides for dependencies, source layout, and change ownership.

## Installation and feature selection

~~~toml
[dependencies]
furnace = { package = "furnace-rs", version = "=0.9.2" }
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
~~~

furnace-rs supports Rust 1.94 and uses Rust edition 2024. The default facade
enables HTTP and logging with the Tokio runtime. For feature combinations, see the
[facade README](crates/furnace-rs/README.md).

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

## Validated requests and REST errors

Use `#[derive(serde::Deserialize, Input)]` with `ValidatedJson<T>`,
`ValidatedQuery<T>`, or `ValidatedPath<T>` to deserialize, validate, attach a
`body`, `query`, or `path` source, and invoke a handler only on valid input.

```rust,no_run
use furnace::prelude::*;

#[derive(serde::Deserialize, Input)]
struct CreateUser {
    #[validate(email, length(max = 254))]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}

#[controller]
struct UserController;
impl Sealable for UserController {
    fn seals() -> SealRegistration<Self> { SealRegistration::new() }
}
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
use furnace::prelude::*;

#[derive(Configuration)]
#[config(prefix = "app")]
struct AppConfig {
    #[config(rename = "bind_host")]
    host: String,
    #[config(default = 3000, validate(range(min = 1, max = 65535)))]
    port: u16,
    api_key: Secret<String>,
}

#[element]
fn app_config(config: Config) -> furnace::core::Result<AppConfig> {
    Ok(config.parse()?)
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

Database support is not a `furnace-rs` or `furnace-rs-common` feature. Add the connector
explicitly and import its global module in your application root:

```toml
furnace-rs-persistence = { version = "0.9.2", features = ["sea-orm-postgres"] }
```

```rust,ignore
use furnace_rs_persistence::sea_orm::{DatabaseConnection, DatabaseCauldron};

#[furnace::cauldron]
struct AppCauldron;
impl furnace::Cauldron for AppCauldron {
    fn register(self) -> furnace::CauldronRegistration<Self> {
        self.import(DatabaseCauldron).provide::<UserRepository>()
    }
}

#[furnace::element]
fn repository(database: DatabaseConnection) -> UserRepository {
    UserRepository::new(database)
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
use furnace::prelude::*;

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

One static policy protects every endpoint of its controller. An empty
`SealRegistration::new()` makes a controller public; use a separate public
controller for login. Roles, permissions, and predicates are ANDed;
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
[v0.5.5 security and release notes](docs/importance/version_0.5.5/passport-jwt-and-cookies.md).

## A typed HTTP route

`#[controller]` declares a managed struct and its inherent endpoint implementation.
Generated adapters resolve the controller once while building the router and
call its Rust methods directly. Handlers use native Axum extractors.

```rust,no_run
use furnace::prelude::*;

#[derive(Clone, serde::Serialize)]
struct User {
    id: u64,
}

#[controller]
struct UserController;
impl Sealable for UserController {
    fn seals() -> SealRegistration<Self> { SealRegistration::new() }
}
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

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

## Extractors, responses, and routing

The prelude exports `Path<T>`, `Query<T>`, `Json<T>`, `Header<T>`, `Request`,
`HttpResult<T>`, `Created<T>`, `NoContent`, `build_router`, `configure_router`,
`serve`, and `serve_router`.
`furnace::common::axum` remains the native Axum escape hatch for extractors,
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

See [BENCHMARK.md](BENCHMARK.md) for the complete results, methodology,
limitations, resource measurements, and interpretation guidance.

## Current scope

Version 0.9.0 includes rooted module scope, conventional startup, CORS,
native router composition, typed input validation, the seven REST errors,
explicit typed configuration and redacted secrets, focused FURNACE macro
diagnostics, Cargo-native run/dev, compiled route/graph/doctor inspection,
version-1 finite-command JSON, opt-in native SeaORM persistence, and the
offline atomic minimal-project generator. It preserves the low-level builder,
the complete-catalog rootless compatibility path, native Axum extractors and
responses, ordinary human CLI output, and application-owned database policy.

It does **not** implement trait or interface bindings, `Inject<dyn Trait>`,
asynchronous or database-backed derive validators, automatic validation for
native extractors, full-RFC/DNS email validation, login or credential
validation, refresh endpoints or persistence/rotation/revocation, password
hashing, CSRF, remote JWKS, JWE, third-party auto-configuration, arbitrary
configuration sources/shapes, multiple-listener/TLS/HTTP2 server configuration,
JSON-wrapped run/dev streams, or scaffold database/JWT/cookie/migration/Git
setup. Database errors never map automatically; applications own their
delivery policy.

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
PostgreSQL 16 database and use the commands in the [v0.5 requirements](docs/importance/version_0.5/auto-configuration.md).

## License

furnace-rs is licensed under either the [Apache License 2.0](LICENSE-APACHE) or
the [MIT License](LICENSE-MIT), at your option.

## Focused tests

Add `furnace-rs-testing = "=0.9.2"` under `[dev-dependencies]`. Annotate an async,
zero-argument test function with `#[furnace::test]`; Cargo runs it without a separate
Tokio dependency. The local `test_fixture()` builds one registered subject's
dependency chain without module setup.

```rust
#[furnace::test]
async fn controller_returns_ok() {
    test_fixture()
        .mock_database(furnace_rs_testing::sea_orm::MockDatabase::new(
            furnace_rs_testing::sea_orm::DbBackend::Sqlite,
        ))
        .controller::<UserController>()
        .run(|client| async move {
            client.get("/users").send().await.unwrap()
                .assert_status(furnace_rs_testing::http_types::StatusCode::OK);
        })
        .await
        .unwrap();
}
```

Use `.subject::<UserService>()` and `context.resolve::<UserService>()` for direct
service tests. Database dependencies require an explicit SQLite SeaORM
`MockDatabase`; it queues scripted results and never opens a production
connection. `run` awaits lifecycle shutdown on completion and unwinding assertion
panics. See the [testing guide](crates/furnace-rs-testing/README.md) for complete service
and controller examples, supplies, and response assertions.
