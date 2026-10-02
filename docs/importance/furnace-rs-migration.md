# Migrating to furnace-rs 1.0.0

1.0.0 is an unreleased major release with breaking API changes from MADS.
All nine workspace packages target 1.0.0, Rust edition 2024 and MSRV 1.94.
The renamed package family must be published before registry-only installation
can work. The manifest below shows the future registry dependency; before
publication, use local workspace/path dependencies. There are no compatibility aliases for the removed declarations.

| Previous API | Current API |
| --- | --- |
| `mads` / `mads-*` packages | `furnace-rs` / `furnace-rs-*` packages |
| `mads` executable | `furnace` executable |
| `Mads::run`, `MadsRunExt` | `Furnace::burn`, `FurnaceBurnExt` |
| `#[module]`, `Module` | `#[cauldron]`, `Cauldron` |
| `#[service]` | `#[burner]` |
| `#[repository]` | `#[storage]` |
| `#[provider]`, `#[element]`, and their lifecycle forms | `Injector::inject` and `Injector::lifecycle` |
| `#[routes]`, `#[controller(routes = [...])]` | Struct and inherent-impl `#[controller]` |
| Trait/method guards and `skip` | Unit-struct `#[guard]` policy plus controller seal |
| `mads.toml`, `MADS_*` | `furnace.toml`, `FURNACE_*` |
| CLI schema 1 / private protocol 1 | CLI schema 2 / private protocol 2 |

## Cargo package names and Rust imports

The nine packages are `furnace-rs`, `furnace-rs-core`, `furnace-rs-core-macros`,
`furnace-rs-common`, `furnace-rs-common-macros`, `furnace-rs-persistence`,
`furnace-rs-testing`, `furnace-rs-extra`, and `furnace-rs-cli`.
The facade's default Rust import is `furnace_rs`. A short explicit alias is
recommended for applications:

```toml
[dependencies]
furnace = { package = "furnace-rs", version = "=1.0.0", default-features = false, features = ["http", "jwt", "runtime-tokio"] }
serde = { version = "1", features = ["derive"] }
```

This enables `use furnace::prelude::*` and `#[furnace::main]`. Without the alias,
use `furnace_rs::prelude` and `#[furnace_rs::main]`. The same rule applies to
integration packages, for example `furnace-persistence = { package =
"furnace-rs-persistence", ... }` imports as `furnace_persistence`.

## A protected controller and a public controller

Rust procedural attributes cannot inspect another unannotated implementation.
Use a bare struct declaration and annotate exactly one inherent implementation
with its route prefix. Other inherent implementations may contain helpers.

```rust,no_run
use furnace::prelude::*;

#[storage]
pub struct UserStorage;
#[burner]
pub struct UserService { storage: UserStorage }
impl UserService {
    fn list(&self) -> &'static str { let _ = &self.storage; "users" }
}

#[derive(serde::Deserialize)]
pub struct UserClaims;
impl PassportPrincipal for UserClaims {
    fn has_role(&self, _: &str) -> bool { false }
    fn has_permission(&self, _: &str) -> bool { false }
}
#[guard(strategy = "jwt", principal = ClaimsPrincipal<UserClaims>)]
pub struct UserGuard;

#[controller]
pub struct UserController { service: UserService }
impl Sealable for UserController {
    fn seals() -> SealRegistration<Self> { Self::seal::<UserGuard>() }
}
#[controller(route = "/users")]
impl UserController {
    #[get("/:id")]
    fn find(&self, Path(id): Path<u64>) -> String { id.to_string() }
    #[get]
    fn list(&self) -> &'static str { self.service.list() }
    #[post]
    async fn create(&self) -> &'static str { "created" }
    #[post("/login")]
    #[seal(skip)]
    fn login(&self) -> &'static str { "public login" }
}

#[controller]
pub struct HealthController;
#[controller(route = "/health")]
impl HealthController {
    #[get]
    fn health() -> &'static str { "ok" }
}

#[cauldron]
pub struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<UserStorage>()
            .provide::<UserService>()
            .controller::<UserController>()
            .controller::<HealthController>()
    }
}
#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

A controller without `impl Sealable` is public by default, as shown by
`HealthController`. An explicit empty `SealRegistration::new()` also remains public.
`UserGuard` protects GET `/users/{id}`, GET `/users`, and POST `/users`.
POST `/users/login` uses `#[seal(skip)]` and bypasses authentication and policy
checks, including when a caller sends an invalid token. A controller supports
at most one seal. The marker accepts exactly `skip` on an HTTP endpoint; it does
not replace the controller policy with another guard. Skipped endpoints do not
receive an authenticated principal from the seal. A controller whose enabled
endpoints all skip needs no JWT output or configuration for those endpoints.
Policies are non-generic unit structs and do not participate in dependency
construction. Role, permission, and synchronous predicate clauses are ANDed.
Cookie policies additionally require `cookies` and select exactly one source.

Handlers may be sync or async, with `&self` or no receiver. Bare verbs and
`#[get("/")]` address the base prefix; `/:id` and `/{id}` normalize to canonical
`/{id}` metadata, and final `/*rest` normalizes to `/{*rest}`. Selected path-tree
conflicts, missing endpoint sets, invalid seals, unavailable JWT outputs, and
strategy visibility failures reject startup before provider construction,
lifecycle hooks, or socket binding. Native Axum extractors and Passport errors
retain their existing behavior.

## Explicit ownership, imports, and exports

Each selected provider output and controller belongs to one cauldron. Rust
namespace placement and Rust `pub` control Rust names, not DI membership or DI
visibility. Register managed providers or plain `T: Injector<T>` services with
`.provide::<T>()`; register controllers with `.controller::<T>()`.
For a trait output, use `.provide_with::<Arc<dyn Trait>, Implementer>()`, where
`Implementer: Injector<Arc<dyn Trait>>` constructs the selected trait value.
The same method supports third-party native outputs such as `DatabaseConnection`.
Exports name the output type, not the constructor type.

### Writing an injector

`Injector<T = Self>` constructs a native output. Its `Dependencies` are `()` or
a tuple of one through sixteen `Clone + Send + Sync + 'static` types. A single
dependency needs a trailing comma. The framework records those types for missing
dependency, cycle, and visibility checks before calling the constructor.

`Dependencies` is a public associated type. Rust therefore requires dependency
types to be sufficiently visible wherever their injector implementation is public.
This also applies to fields of public `#[burner]`, `#[storage]`, and `#[controller]`
structs: the macros expose their field types through `Injector::Dependencies`.
A public managed struct with a private dependency now fails with **E0446**.
Make the dependency type public, or reduce the managed struct's visibility when
it need not be public. For example:

```rust
#[derive(Clone)]
pub struct UserDependency; // Previously private; required by the public injector.

#[burner]
pub struct UserService {
    dependency: UserDependency, // The field itself can remain private.
}
```

Rust visibility does not export a dependency from its cauldron. Making
`UserDependency` public only satisfies Rust's type visibility rules; it remains
private to its owning cauldron until explicitly exported with `.export::<T>()`.

```rust
use furnace::prelude::*;
use std::sync::Arc;

trait UserRepository: Send + Sync {
    fn name(&self) -> &str;
}
struct MemoryRepository(String);
impl UserRepository for MemoryRepository {
    fn name(&self) -> &str { &self.0 }
}
impl Injector<Arc<dyn UserRepository>> for MemoryRepository {
    type Dependencies = (Config,);
    async fn inject((config,): Self::Dependencies)
        -> furnace::core::Result<Arc<dyn UserRepository>>
    {
        Ok(Arc::new(Self(config.get("app.name").unwrap_or("demo").to_owned())))
    }
}

#[burner]
struct UserService { repository: Arc<dyn UserRepository> }

#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide_with::<Arc<dyn UserRepository>, MemoryRepository>()
            .provide::<UserService>()
    }
}
```

`inject` is an associated async constructor: `Type::inject(dependencies).await`,
without a receiver or preconstructed service instance. It returns core `Result<T>`
and a `Send` future. Registration performs no construction or I/O. Macro-managed
providers implement `Injector<Self>` automatically; custom constructors can use
plain structs without `#[burner]` or `#[storage]`.

For lifecycle resources, return the native service from `inject` and override
`fn lifecycle(value: T) -> LifecycleResource<T>`. Attach existing application or
infrastructure hooks there. Keep dependency state needed by hooks in the native
output and clone its handles when attaching hooks. The runtime attaches hooks
once after successful construction; supplied output overrides skip both steps.
Startup ordering, rollback, and reverse shutdown remain unchanged.

Manual injectors are discovered from reachable cauldron registrations, so select
an application root when building or testing them. Unrooted catalog/focused
builds discover managed macro descriptors and deliberately registered official
integration metadata. A linked but unreachable manual injector does not affect
the selected application.

The `element` macro and its lifecycle form are removed, with no compatibility
alias. Ordinary helper functions remain usable but no longer register outputs.

`.import(UserCauldron)` exposes that directly imported cauldron's explicit
`.export::<UserService>()` outputs. Transitive imports do not expose transitive
exports, and imported outputs cannot be re-exported. Register a shared output
once in its owning cauldron and import that cauldron wherever needed.

A reachable cauldron marked with `.global()` exposes only its explicit exports
to the selected application. It does not select an otherwise unreachable
cauldron. Independent roots keep distinct instances and analysis state.

`LoggerCauldron` exports `Logger`; persistence's `DatabaseCauldron` exports native
`DatabaseConnection`. Import these infrastructure cauldrons explicitly. Core has
no HTTP, JWT, cookie, or database dependency. HTTP-only controllers may omit
`Sealable` to remain public; Passport requires HTTP+JWT. Focused test fixtures select only their target
controller, dependency chain, endpoints, and seal.

## Configuration, CLI, and inspection

Rename `mads.toml` to `furnace.toml`. Optional `.env` interpolation and process
precedence are unchanged. `FURNACE_SERVER__HOST` and `FURNACE_SERVER__PORT` map to
`server.host` and `server.port`; the double underscore denotes a nested key.
Legacy config filenames and implicit `MADS_*` sources are no longer loaded.
`Config::parse` and explicit custom `EnvSource` prefixes remain supported.

Use `furnace new`, `furnace dev`, `furnace run`, `furnace routes`, `furnace graph`,
and `furnace doctor`. The starter creates exactly six files: `Cargo.toml`,
`furnace.toml`, `src/main.rs`, and `src/app/{mod,controller,service}.rs`.
It contains no route-trait file and enables only HTTP and Tokio.

Public CLI schema 2 route records contain controller, handler, method,
canonical path, location, and selected guard evidence. The removed route-trait
field has no replacement trait value. Graph records use `root_cauldron` and
`cauldrons`. Private inspection protocol 2 rejects version 1 before application
work and does not construct providers or bind listeners. Diagnostics use
`FURNACE` prefixes with the existing numeric meanings and redaction policy.

Historical version notes and verification reports retain their original names
and APIs. Follow this guide and the current README for new applications.
