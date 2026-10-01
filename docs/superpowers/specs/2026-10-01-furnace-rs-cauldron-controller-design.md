# furnace-rs branding, cauldrons, direct controller endpoints, and seals

Status: proposed specification for user review. The conversation approves the names, direct endpoints, the two uses of `#[controller]`, and controller-wide protection. The concrete contracts and migration boundaries below require review before writing the implementation plan.

## Intent

Rename the framework completely from MADS.rs to furnace-rs. An application burns inside `Furnace`, feature composition belongs to `Cauldron`, and HTTP endpoints live directly on controller implementations. Protection is an explicit, statically analyzable seal on the complete controller.

Preserve the explicit ownership and export rules implemented on `refactor/mads-declaration-module`. Remove the public route-trait system, while retaining HTTP routing, native Axum interoperability, request extraction, validation, authentication, lifecycle, and CLI inspection.

## Confirmed decisions

- Project name: **furnace-rs**.
- Application runtime: **Furnace**.
- Module declaration: **`#[cauldron]`**; HTTP boundary stays **`#[controller]`**.
- Keep `#[burner]`, `#[storage]`, and `#[element]`, including `#[element(lifecycle)]`.
- Remove `#[routes]` and its requirement to author a separate endpoint trait.
- Use `#[controller]` on the DI struct and `#[controller(route = "/user")]` on its inherent implementation containing endpoint methods.
- Protection uses `impl Sealable` and a typed `.seal::<Guard>()` declaration.
- A seal protects every endpoint of the controller. No endpoint-level public escape or protection override.

## Proposed complete public API

```rust,ignore
use furnace::prelude::*;

#[burner]
pub struct UserService;

#[controller]
pub struct UserController {
    service: UserService,
}

#[controller(route = "/user")]
impl UserController {
    #[get("/:id")]
    async fn find(&self, Path(id): Path<u64>) -> String {
        format!("user {id}")
    }

    #[get]
    async fn list(&self) -> &'static str {
        "user list"
    }

    #[post]
    async fn create(&self) -> &'static str {
        "user created"
    }
}

#[derive(serde::Deserialize)]
pub struct UserClaims;
impl PassportPrincipal for UserClaims {
    fn has_role(&self, _role: &str) -> bool { false }
    fn has_permission(&self, _permission: &str) -> bool { false }
}

#[guard(strategy = "jwt", principal = ClaimsPrincipal<UserClaims>)]
pub struct JwtGuard;

impl Sealable for UserController {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<JwtGuard>()
    }
}

#[cauldron]
pub struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<UserService>().controller::<UserController>()
    }
}

#[furnace::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

Public controllers explicitly return an empty seal registration:

```rust,ignore
impl Sealable for HealthController {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}
```

The snippets describe the proposed API rather than APIs already available in the current checkout.

## Branding and rename boundary

Use `furnace-rs` for the project title, repository branding, and facade Cargo package. The crates.io package `furnace` is already registered, so the new packages use the `furnace-rs` family. A public crates.io API check on 2026-10-01 returned HTTP 200 for `furnace` and HTTP 404 for all proposed names in the table below. This is a point-in-time check, not a package-name reservation. Rust imports may keep the shorter namespace through explicit Cargo dependency aliases:

| Current name | New name |
| --- | --- |
| `mads`, `mads-core`, `mads-core-macros` | `furnace-rs`, `furnace-rs-core`, `furnace-rs-core-macros` |
| `mads-common`, `mads-common-macros` | `furnace-rs-common`, `furnace-rs-common-macros` |
| `mads-persistence`, `mads-testing`, `mads-extra` | `furnace-rs-persistence`, `furnace-rs-testing`, `furnace-rs-extra` |
| `mads-cli` package and `mads` executable | `furnace-rs-cli` package and `furnace` executable |
| `Mads`, `MadsBuilder`, `MadsBurnExt` | `Furnace`, `FurnaceBuilder`, `FurnaceBurnExt` |
| `Furnace` module trait, `FurnaceRegistration` | `Cauldron`, `CauldronRegistration` |
| `FurnaceDefinition`, `FurnaceMember`, `FurnaceImport` | `CauldronDefinition`, `CauldronMember`, `CauldronImport` |
| `ModuleDescriptor`, `ModuleGraph`, `ModuleNode`, `ModuleImportEdge` | `CauldronDescriptor`, `CauldronGraph`, `CauldronNode`, `CauldronImportEdge` |
| Built-in `LoggerModule`, `DatabaseModule` | `LoggerCauldron`, `DatabaseCauldron` |
| `mads.toml` | `furnace.toml` |
| `MADS_*` environment convention | `FURNACE_*` |
| `MADS_TEST_DATABASE_URL` | `FURNACE_TEST_DATABASE_URL` |
| `MADS_INTERNAL_INSPECTION_*` | `FURNACE_INTERNAL_INSPECTION_*` |
| Diagnostic constants/codes `MADS008`, etc. | `FURNACE008`, preserving numeric suffixes |
| Official identifiers `mads.common.*`, `mads.persistence.*` | `furnace.common.*`, `furnace.persistence.*` |

Recommended dependency declaration for facade consumers and generated projects:

```toml
[dependencies]
furnace = { package = "furnace-rs", version = "=0.9.2", default-features = false, features = ["http", "runtime-tokio"] }
```

This keeps `use furnace::prelude::*` and `#[furnace::main]`. Without a dependency alias, the default import name is `furnace_rs`. Internal dependencies may likewise use short keys such as `furnace-core = { package = "furnace-rs-core", ... }`; proc-macro crate discovery must resolve actual Cargo package names and honor consumer aliases. Example persistence imports use an explicit `furnace-persistence` alias for `furnace-rs-persistence`. Cargo package identity, Rust import aliases, and the CLI executable are separate names.

Rename workspace directories, manifests, internal dependency keys, crate-path discovery, lockfiles, generated symbols, CLI metadata fields, temporary file prefixes, scripts, workflows, runnable examples, and active documentation. Keep feature names and existing package versions; this work does not select a release version.

Framework-authored topology APIs and inspection fields use `cauldron`, including `root_cauldron`, imports, and provider ownership. General domain vocabulary remains: `ProviderKind::{Service, Repository, Provider}`, `UserService`, and repository/service functionality do not need unrelated renaming. User-authored `AppModule` names still work as ordinary Rust type names, but shipped examples use `AppCauldron`.

There are no compatibility exports, old executable aliases, old config filename fallbacks, old environment prefixes, or old declaration macros. Ordinary explicit `TomlSource`/`EnvSource` APIs can still load names selected by the application.

Increment the private child inspection protocol and public CLI JSON schema to version 2 because topology field names change. Old protocol versions fail clearly before application construction. Command names remain `new`, `run`, `dev`, `routes`, `graph`, and `doctor`; `furnace routes` still inspects HTTP endpoints.

Historical specs, plans, and verification records remain historical records. Replace active documentation links and branding; do not rewrite historical examples into a misleading mixture of old and new APIs. Renaming a remote repository, publishing packages, acquiring registry names, pushing, or merging is outside this implementation.

## Cauldron registration and ownership

`Cauldron::register(self) -> CauldronRegistration<Self>` keeps the established unit-struct callback contract. Generic `.provide::<T>()`, `.controller::<T>()`, `.import(C)`, `.export::<T>()`, and `.global()` retain their behavior and caller locations.

One selected application graph owns each registered output exactly once. Direct imports expose only explicit exports. Reachable globals expose only explicit exports. A declaration's Rust namespace or `pub` visibility does not grant dependency access. Independent application roots may reuse declarations without sharing callback state or instances.

Inventory remains a declaration catalog. Only selected registrations contribute dependency errors, endpoint conflicts, strategy validation, and global exports. Diamonds evaluate each cauldron callback once per analysis. Factories register their normalized output types. Multiple linked constructors for one selected output remain ambiguous unless an explicitly registered supplied value overrides the unused factory metadata. Overrides preserve ownership and access boundaries.

`Config` and narrowly scoped unowned official contributions remain ambient. Explicitly registered `JwtService`, including a supplied override, must be accessible from every controller seal using it. Registered Passport strategies obey the same access checks.

## Direct controller macro contract

`#[controller]` on a struct creates the current application-scoped managed handle, constructor metadata, controller role marker, and a static seal callback. Support non-generic named-field and unit structs. Preserve dependency ordering, visibility, documentation, lint/cfg attributes, and existing cheap handle cloning.

`#[controller(route = "/user")]` on an inherent `impl UserController` reads the complete implementation token stream. The macro knows the controller type and can parse its methods without scanning source files or relying on expansion-order global state. No trait implementation is accepted as an endpoint block. Exactly one annotated endpoint implementation contributes metadata for a controller; duplicate declarations are registration failures. Additional ordinary inherent implementations for helper methods remain valid.

The prefix argument is optional and defaults to the root. Endpoint methods use `#[get]`, `#[post]`, `#[put]`, `#[patch]`, or `#[delete]`, optionally with one string path. A bare verb selects the controller base path. A method without a verb is a normal helper and contributes no endpoint. Verb attributes outside an annotated endpoint implementation produce focused compile errors.

Accept synchronous and asynchronous handlers, `&self` receivers or associated functions without a receiver, typed native Axum extractors, and return values implementing the existing response contract. Reject generic endpoints, consuming or mutable receivers, malformed paths, incompatible extractor layouts, and multiple HTTP verbs on one method. Generated adapter futures must satisfy the existing Send requirements. Synchronous handlers run inline and do not gain automatic blocking-task semantics.

`route = "/user"` plus `#[get("/:id")]` describes the canonical native path `/user/{id}`. Accept the colon parameter form from the proposed API and native `{id}` syntax; normalize both to the same path for registration, conflict checking, and inspection. Preserve existing wildcard validation and native extractor behavior. Bare verbs produce `/user`; base `/` remains `/`, without accidental doubled slashes. Missing/invalid parameter names fail at compile time.

Remove the old `#[controller(routes = [...])]` argument and route-trait contract marker machinery. Runtime endpoint descriptors may retain an internal grouping record to share validation/registration code, but no public API requires or invents a Rust route trait. Inspection identifies the controller and method directly instead of serializing a fictional route-trait name.

## Sealable and typed guards

The proposed HTTP trait is:

```rust,ignore
pub trait Sealable: Send + Sync + Sized + 'static {
    fn seals() -> SealRegistration<Self>;
}
```

Every managed HTTP controller implements it, including public controllers returning `SealRegistration::new()`. The struct macro emits the associated `Self::seal::<G>()` convenience method, returning `SealRegistration<Self>`. `SealRegistration::seal::<G>()` records guard type identity and caller location; it constructs neither a controller nor a guard. Endpoint method names must not collide with the generated `seal` helper.

For this change, a controller has either no seal or exactly one guard policy. Repeated or multiple guard registrations fail deterministically; ordering/composition of multiple authentication pipelines is outside scope. No endpoint may opt out, replace the guard, or add an independent endpoint guard. Separate public/protected controllers when access differs.

`#[guard(...)]` now declares a non-generic unit policy type implementing the doc-hidden guard-policy metadata contract. It retains the existing strategy, principal, token source, roles, permissions, and predicate grammar, excluding `skip`. Bearer remains the default source; cookies require the cookies feature. Invalid policy syntax fails on the declaration. Applying `#[guard]` to endpoint methods or route traits is rejected.

A guard type is static policy metadata, not an injectable service or a constructed provider. It can be referenced from multiple controllers without registering it with `.provide`. Its actual strategy, JWT service, and other managed strategy dependencies must satisfy the requesting controller's cauldron access rules. Guard adapters and principal typing remain statically checked. A selected controller with missing seal metadata, invalid strategy selection, or inaccessible registered infrastructure fails analysis before provider constructors or listening.

Sealable/empty registrations are available with HTTP alone. Actual Passport guard declarations and nonempty seals require HTTP plus JWT; cookie-source guards additionally require cookies. A core-only dependency does not acquire HTTP, Passport, or database dependencies.

## Analysis and runtime sequencing

Registration and seal callbacks describe deterministic static metadata and perform no I/O. Analyze endpoint ownership, canonical route conflicts, selected seal policies, strategy/principal compatibility, and JWT access before constructors. This extends preconstruction validation to direct endpoint conflicts rather than preserving the old route-conflict check only at router finalization.

The low-level builder and standard `Furnace::burn` use the same selected topology and HTTP preflight. Route traits and unrelated linked controllers/guards/strategies cannot poison another root. Router assembly still validates metadata defensively and obtains controller handles only after the dependency graph is built. Controllers reached through shared cauldron imports register routes once.

Preserve startup configuration precedence, listener defaults, CORS finalization, authentication/authorization ordering, redacted errors, lifecycle preparation/rollback/shutdown, native Axum router merging, and focused fixture isolation. Only framework branding, endpoint declaration, and protection declaration change. Authentication failures remain 401, authorization failures remain 403, and rejected requests never call the handler.

## CLI, examples, testing, and migration

`furnace new` emits `furnace.toml`, an `AppCauldron`, a burner, and a controller with its endpoint implementation and empty `Sealable` registration. Remove the generated `routes.rs` file and `mod routes`; retain ordinary service/controller files. Generated manifests use renamed packages with the same exact version convention. Inspection still avoids construction, resource readiness, and listeners.

Migrate all feature-matrix/renamed-dependency consumers, compile-pass/fail fixtures, CLI fixtures, configuration fixtures, environment tests, persistence and focused testing fixtures, and examples. Replace trait-based calls in tests with inherent controller methods while preserving their behavioral purpose. Database integration keeps native SeaORM types and exports only its connection from `DatabaseCauldron`; logger exports only `Logger` from `LoggerCauldron`.

Publish a migration guide with before/after package/config/environment names, cauldron registration, direct methods, bare HTTP verbs, canonical paths, public controller seals, protected controller policies, explicit JWT/strategy exports, CLI schema changes, and removed APIs. Correct the previous deferred rustdoc sentence implying that unrestricted Rust `pub` grants DI access.

## Required acceptance coverage

- External consumers using default names (`furnace_rs`) and recommended or arbitrary aliases (`furnace`, etc.) compile the new crate family and public preludes; old packages/imports/macros/startup names are rejected in intentional failure fixtures.
- Core-only, HTTP-only, JWT-only, cookies, logger, persistence, and focused testing feature boundaries remain correct.
- A dependency-bearing controller uses direct methods without a route trait; sync/async methods and native typed extractors behave correctly.
- Base GET/POST coexist; prefixed parameter endpoints extract IDs; malformed paths and duplicate canonical verb/path combinations fail before constructors.
- Unregistered controllers/guard policies/strategies do not contribute routes or poison a valid independent root; shared imports deduplicate controllers.
- Empty seals permit public endpoints. One seal protects every endpoint, denies unauthenticated/unauthorized requests before handler execution, and preserves principal extraction.
- Private foreign JWT overrides and strategies remain inaccessible. Direct exported and reachable global exported infrastructure work; unowned official JWT defaults remain conditional and ambient.
- Unsupported controller shapes/endpoint implementations, missing Sealable, duplicate/multiple seals, endpoint guard overrides, removed routes syntax, and unsupported feature combinations have focused UI diagnostics.
- `furnace new` output compiles, produces correct version-2 inspection reports, and serves its hello-world endpoint. Dev watching recognizes `furnace.toml`.
- Inspector rejects protocol 1 and returns redacted version-2 graph/endpoint/doctor evidence without construction or binding.
- All active runnable code/doc examples use the new API; historical records and deliberate fail fixtures are classified separately.
- Full formatting, strict Clippy, workspace stable/MSRV tests and doctests, rustdoc, feature checks, examples, archive checks, and CI database tests pass or record a concrete external verification limitation.

## Implementation boundaries

Implement this as one coordinated breaking migration with staged task commits: branding/cauldron APIs, direct controller metadata/adapters, seals/preflight, integrations/consumers/scaffolds, docs and verification. Preserve the assigned local branch unless the user requests a different branch. No product implementation begins until this specification and the subsequent written execution plan are approved. Publishing and external repository changes remain separate actions.
