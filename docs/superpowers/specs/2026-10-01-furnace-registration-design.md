# Explicit furnace registration and application vocabulary

Status: Approved by the user on 2026-10-01; implementation authorized in the existing refactor branch.

## Intent and agreed vocabulary

Replace namespace-inferred module membership with explicit registration. A dependency belongs to one furnace within a rooted application, and other furnaces consume only its published interface. The application starts from one root furnace.

The conversation establishes these public names:

| Existing API | Replacement |
| --- | --- |
| `#[module]` | `#[furnace]` |
| `#[service]` | `#[burner]` |
| `#[repository]` | `#[storage]` |
| `#[provider]` | `#[element]` |
| `#[module(global)]` | registration-chain `.global()` |
| `Mads::run::<AppModule>()` | `Mads::burn::<AppModule>()` |

User-authored type names such as `UserModule` and `UserService` may stay as they are. Existing built-in type names such as `LoggerModule`, `DatabaseModule`, and `JwtService` remain; the terminology change targets declaration macros, the module contract, and standard startup.

The user also confirmed registration inside `impl Furnace for AppModule` through a `register` method.

## Approved design decisions

The following fill gaps in the conversation rather than represent previously approved requirements:

1. The confirmed `register` method uses the proposed exact signature `fn register(self) -> FurnaceRegistration<Self>`.
2. Provider and controller registration use `.provide::<T>()` and `.controller::<T>()`; imports use `.import(OtherModule)` to retain the requested unit-struct chaining syntax.
3. Cross-furnace access requires explicit `.export::<T>()`; Rust `pub` alone does not export a dependency.
4. Ownership uniqueness is enforced within the selected application graph. Independent application roots may reuse the same declarations.
5. Old declaration macros and `run` are removed in this breaking change, with no compatibility aliases. Existing low-level graph/report type names remain unless this spec explicitly replaces them.

The generic provider syntax is necessary because a named-field struct is a type, not a ready-made value. Passing `UserService` would require creating a service before dependency injection. No constructor is executed during registration.

## Current implementation and impact

The workspace is version 0.9.2, Rust edition 2024, with Rust 1.94 as its minimum. `mads-core-macros/src/module_v2.rs` emits static module metadata, imports, and global status. `mads-core/src/graph/scope.rs` assigns a provider to its longest matching Rust namespace, and allows cross-module access through direct imports and unrestricted `pub` visibility. Multiple modules in the same namespace are rejected.

Controller and Passport selection in `mads-common/src/http_scope.rs` also relies on namespaces. Provider functions already support synchronous/asynchronous construction, trait-object outputs through concrete wrappers such as `Arc<dyn Trait>`, and lifecycle resources. Standard startup and inspection share the `MadsRunExt` path in `mads-common/src/server.rs`. Persistence, logger, scaffolding, and test fixtures depend on these contracts.

Registration must therefore change metadata, ownership analysis, HTTP selection, official integrations, and generated application templates together. Merely renaming macros would preserve the behavior the user wants to break.

## Public declaration and registration API

```rust
use mads::prelude::*;

#[mads::furnace]
pub struct UserModule;

impl Furnace for UserModule {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<UserService>()
            .provide::<UserRepository>()
            .controller::<UserController>()
            .export::<UserService>()
    }
}

#[mads::furnace]
pub struct AppModule;

impl Furnace for AppModule {
    fn register(self) -> FurnaceRegistration<Self> {
        self.import(UserModule)
    }
}

#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
```

`#[furnace]` accepts only non-generic unit structs and no arguments. It emits a catalog descriptor with an erased registration callback, an assertion that the authored type implements `Furnace`, and inherent chain-entry methods. It does not also implement `Furnace`, because the user supplies that implementation.

Public contracts:

```rust
pub trait Furnace: Send + Sync + Sized + 'static {
    fn register(self) -> FurnaceRegistration<Self>;
}

pub struct FurnaceRegistration<M: Furnace> { /* private metadata */ }

impl<M: Furnace> FurnaceRegistration<M> {
    pub fn new(module: M) -> Self;
    pub fn provide<T: Send + Sync + 'static>(self) -> Self;
    pub fn controller<T: Send + Sync + 'static>(self) -> Self;
    pub fn import<I: Furnace>(self, module: I) -> Self;
    pub fn export<T: Send + Sync + 'static>(self) -> Self;
    pub fn global(self) -> Self;
}
```

The furnace macro emits inherent forwarding methods with matching generic bounds for `provide`, `controller`, `import`, `export`, and `global`; each returns `FurnaceRegistration<Self>`. An empty furnace returns `FurnaceRegistration::new(self)`. The generic builder remains framework-neutral; `controller` records a role-tagged entry that HTTP integration validates against its controller catalog. No Axum dependency enters `mads-core`.

Chain calls record type identifiers, authored order, and caller locations without constructing values, recursively registering imports, or validating the graph. Root traversal invokes each reachable furnace callback once per analysis pass and deduplicates diamond imports. Analysis followed by build may evaluate registration again; implementations must only describe static topology and perform no I/O or configuration-dependent branching.

`#[burner]` and `#[storage]` retain the current managed-struct semantics, field injection, supported shapes, and generated behavior. They publish constructor metadata, not application membership.

`#[element]` retains the current free-function provider semantics, including `#[element(lifecycle)]`. It does not become a struct macro. Register the normalized output type; `Result<T>` and `LifecycleResource<T>` register `T`:

```rust
#[mads::element]
pub fn user_repository(config: Config) -> mads::core::Result<Arc<dyn UserRepository>> {
    // Existing factory implementation.
}

// Inside a furnace registration:
self.provide::<Arc<dyn UserRepository>>()
```

The linked catalog must contain a unique constructor descriptor for each explicitly registered output type. Multiple factories with the same output cannot be selected by function name in this release; report an ambiguous binding. This retains the existing concrete-output binding model. General trait-implementation registration, factory-selection tokens, and per-furnace instances of the same output type are outside scope.

## Membership, exports, imports, and globals

For rooted applications, static inventory is only a declaration catalog. A declaration is selected only by reachable furnace registration, or by an explicitly allowed framework contribution. Neither Rust namespace proximity nor dependency traversal implicitly registers user dependencies.

Rules:

- Every explicitly registered output or controller has exactly one reachable owning furnace. Duplicate entries within a furnace and registration of the same output in two reachable furnaces are errors, including a controller listed through both `provide` and `controller`.
- Several furnaces may exist in one Rust namespace. Namespace is retained as source/inspection metadata, never as ownership evidence.
- Each dependency may access its owner's local registrations, exports of directly imported furnaces, and exports of reachable global furnaces.
- `.export::<T>()` requires a locally registered ordinary provider. An imported provider cannot be re-exported, and controllers cannot be exported.
- `pub` and `pub(crate)` remain Rust access controls; DI exports are independent. The registration source must legally name the type, but unrestricted `pub` is not an additional runtime export requirement.
- `.import(B)` makes B reachable and permits access to B's exports. It does not copy B's providers into A. Transitive imports affect reachability but do not expose transitive providers to A.
- `.global()` makes the furnace's explicit exports accessible from every reachable furnace. It does not expose private registrations, activate unimported furnaces, or make controllers globally injectable.
- Repeated imports and import cycles are rejected. A shared dependency reached through a diamond is valid and registered/constructed once.
- Providers are constructed in dependency order. Import and registration order do not determine construction order; metadata order and diagnostic ordering remain deterministic.
- Moving a declaration between Rust namespaces does not change DI membership or accessibility.
- Framework `Config` remains an ambient framework dependency. Official auto-configurations are narrowly scoped framework contributions; they must not make arbitrary unregistered user factories accessible.

All registered providers/controllers in the reachable graph are validated before constructors, lifecycle hooks, or binding the listener. Unregistered declarations and unreachable furnaces do not contribute routes, dependency errors, ownership conflicts, or global exports. Inventory lookup errors are evaluated only for selected registrations.

## Metadata and graph architecture

Keep existing `ModuleDescriptor`, `ModuleGraph`, `ModuleNode`, `ProviderOwnership`, `ProviderKind`, and report structures to avoid an unrelated wholesale type rename. Replace the public marker `Module` with `Furnace`, and adapt all root generic bounds. `ProviderKind` retains semantic variants `Service`, `Repository`, and `Provider`; macro branding does not change inspection role semantics.

Introduce `mads-core/src/furnace.rs` for the public registration contract and erased registration records. Extend `ModuleDescriptor` with a registration callback. Collect imports, exports, global status, and role-tagged members from callback output into one analyzed graph. `ModuleGraph` exposes authoritative membership and export accessors consumed by scope selection and HTTP code:

```rust
pub fn owner_of(&self, output: TypeId) -> Option<&ModuleNode>;
pub fn exports(&self, module: TypeId, output: TypeId) -> bool;
pub fn is_global(&self, module: TypeId) -> bool;
pub fn is_controller(&self, output: TypeId) -> bool;
pub fn can_access(&self, requester: TypeId, output: TypeId) -> bool;
```

Namespace metadata is retained for reports; delete namespace collision restrictions and ownership inference. Retained ownership/report records describe the explicit graph. Use `MADS008` for furnace topology/registration failures, `MADS009` for inaccessible dependencies, and existing missing/ambiguous provider codes for missing/ambiguous constructor metadata. Errors include requester, owner where available, output type, registration location, and dependency path where available. Error titles and suggestions use the new public vocabulary.

## Runtime, HTTP, and tooling integration

Replace `MadsRunExt` with `MadsBurnExt` and `run<M: Module>` with `burn<M: Furnace>`, retaining the existing `Send` future and `HttpRuntimeError` result. Keep conventional configuration precedence, server defaults, automatic CORS, lifecycle behavior, shutdown, and the private inspection handshake. `.burn` is available under the same HTTP/runtime feature conditions as `.run` today; core-only applications still use the low-level builder.

`MadsBuilder::root<M: Furnace>()` selects the new graph. Rooted builder overrides must target registered outputs and retain their furnace ownership/export rules; a supplied value cannot bypass visibility or satisfy an unregistered arbitrary user dependency. Reject an override outside the registered graph. Focused test fixtures and unrooted complete-catalog builders retain their intentional isolated/low-level modes; they are outside rooted application membership enforcement.

HTTP selection uses registered controller TypeIds and explicit owners. Selecting a controller retains its routes and attached guards. Route traits and guards remain metadata associated with selected controllers and do not require new registration methods. Passport strategies must be registered providers accessible from the requesting controller's furnace through local membership, direct exports, or global exports. Route, guard, and strategy namespaces must not grant access. Exporting a provider does not export or copy routes.

Convert `LoggerModule` into a global furnace registering/exporting `Logger`. Convert `DatabaseModule` into a global furnace registering its private factory and connector and exporting only `DatabaseConnection`. Existing official JWT/server/CORS auto-configurations retain feature gates, condition checks, override precedence, and redacted reports; associate contributions with the requesting registered graph rather than inferred namespaces.

Update facade/core/common exports and preludes, CLI templates and fixtures, examples, testing consumers, crate README files, root README, rustdoc, and migration documentation. The private inspection protocol stays version 1 if its wire fields remain unchanged; this design changes how values are derived rather than adding wire fields. Keep CLI JSON output semantics consistent with the existing report contracts.

## Breaking-change migration

1. Rename declaration macros using the vocabulary table and `provider(lifecycle)` to `element(lifecycle)`.
2. Replace attribute imports/global flags with an authored `Furnace::register` chain.
3. Explicitly list every provider output and controller previously inferred from namespaces.
4. Add exports for deliberate cross-furnace dependencies; imports alone no longer expose every public provider.
5. Replace `Module` bounds with `Furnace`, `MadsRunExt` with `MadsBurnExt`, and `Mads::run` with `Mads::burn`.
6. Use one shared owning furnace for dependencies that multiple consumers need, rather than registering the same output twice.

Select a breaking pre-1.0 release version separately; drafting this change does not modify versions or publish packages.

## Constraints and acceptance criteria

- Rust edition 2024; minimum supported Rust version 1.94.
- No new dependencies and no unsafe code.
- Core-only builds must not acquire HTTP, JWT, or database dependencies.
- Rooted ownership is explicit and unique within one selected application graph.
- Global furnaces expose only explicit exports and must be reachable from the selected root.
- Registration and analysis execute no provider constructors or lifecycle hooks.
- Existing standard configuration, lifecycle, shutdown, and inspection behavior is preserved under `burn`.

Acceptance tests cover two furnaces in the same namespace; moving a provider across namespaces; omitted registration; duplicate membership; missing/ambiguous factory metadata; explicit and missing exports; direct and transitive imports; unreachable and reachable globals; duplicate imports; cycles and diamond imports; private local dependencies; async and lifecycle factory outputs; registered controllers versus stray controllers; Passport strategy visibility; official integrations and builder overrides; independent application roots; renamed dependency consumers; feature boundaries; and generated starter compilation.

## Review and execution boundary

Review the proposed decisions and the companion execution plan before code implementation. The largest API choice is the registration method plus generic `.provide::<T>()` form. If value-style `.provide(Type)` is essential, revise this design with an explicit registration-token mechanism before execution. No implementation, release, merge, or publishing is part of this documentation task.
