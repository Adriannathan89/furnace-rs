# Migrating to furnace-rs

This is an unreleased breaking API change. Workspace package versions remain
0.9.2, Rust edition 2024 and MSRV 1.94. The renamed package family must be
published before registry-only installation can work; local examples use path
dependencies. There are no compatibility aliases for the removed declarations.

| Previous API | Current API |
| --- | --- |
| `mads` / `mads-*` packages | `furnace-rs` / `furnace-rs-*` packages |
| `mads` executable | `furnace` executable |
| `Mads::run`, `MadsRunExt` | `Furnace::burn`, `FurnaceBurnExt` |
| `#[module]`, `Module` | `#[cauldron]`, `Cauldron` |
| `#[service]` | `#[burner]` |
| `#[repository]` | `#[storage]` |
| `#[provider]`, `#[provider(lifecycle)]` | `#[element]`, `#[element(lifecycle)]` |
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
furnace = { package = "furnace-rs", version = "=0.9.2", default-features = false, features = ["http", "jwt", "runtime-tokio"] }
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
}

#[controller]
pub struct HealthController;
impl Sealable for HealthController {
    fn seals() -> SealRegistration<Self> { SealRegistration::new() }
}
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

The empty health seal is public. `UserGuard` protects GET `/users/{id}`, GET
`/users`, and POST `/users`. A controller supports at most one seal, with no
per-endpoint guard overrides or skips. Put public login on a separate controller.
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
visibility. Register output types with `.provide::<T>()`; register controllers
with `.controller::<T>()`. Factory functions remain ordinary callable functions:
for an `#[element]` factory returning `Result<T>` or `Result<LifecycleResource<T>>`,
register `T`; for a trait factory, register the actual `Arc<dyn Trait>` output.

`.import(UserCauldron)` exposes that directly imported cauldron's explicit
`.export::<UserService>()` outputs. Transitive imports do not expose transitive
exports, and imported outputs cannot be re-exported. Register a shared output
once in its owning cauldron and import that cauldron wherever needed.

A reachable cauldron marked with `.global()` exposes only its explicit exports
to the selected application. It does not select an otherwise unreachable
cauldron. Independent roots keep distinct instances and analysis state.

`LoggerCauldron` exports `Logger`; persistence's `DatabaseCauldron` exports native
`DatabaseConnection`. Import these infrastructure cauldrons explicitly. Core has
no HTTP, JWT, cookie, or database dependency. HTTP-only controllers use empty
seals; Passport requires HTTP+JWT. Focused test fixtures select only their target
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
