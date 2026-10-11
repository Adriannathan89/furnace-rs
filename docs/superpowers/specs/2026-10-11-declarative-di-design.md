# Declarative cauldron registration and compile-time DI

Status: draft for review; this document specifies an API proposal, not implemented support.

## Intent and bounded scope

Keep `#[burner]`, add field-level `#[inject]`, and declare providers, controllers, imports, exports, and global status inside the existing `#[cauldron]` attribute. Detect missing declared dependencies and invalid module visibility before an application artifact can be built. Preserve Furnace's runtime construction, native outputs, asynchronous constructors, managed handles, configuration, and lifecycle.

The implementation is bounded to DI metadata, macro expansion, root contract verification, and adoption by the existing database/logger modules. It does not replace the runtime graph engine. It is an API extension with infrastructure-module migration; removing the fluent API is outside scope. Architecturally, root-relative global verification is new and must be proven before the migration proceeds.

Current branch: `refactor/compile-time-di-checker`. At design time the checkout is clean and based on the existing 1.0.2 implementation; there is no checked-cauldron implementation in this checkout to extend.

## Proposed application API

```rust
#[burner]
pub struct UserService {
    #[inject]
    database: DatabaseConnection,
    #[inject]
    logger: Logger,
}

#[cauldron(
    providers = [UserService],
    exports = [UserService],
)]
pub struct UserModule;

#[controller]
pub struct UserController {
    #[inject]
    users: UserService,
}

#[cauldron(
    imports = [UserModule],
    controllers = [UserController],
)]
pub struct HttpModule;

#[cauldron(
    root,
    imports = [DatabaseCauldron, LoggerCauldron, HttpModule],
)]
pub struct AppCauldron;

// Existing entry point remains Furnace::burn::<AppCauldron>().await.
```

Examples assume the corresponding existing public imports and features. The `root` flag is a proposed design decision requiring review: a module macro sees its declaration, not the generic argument of a separate `Furnace::burn` call. Naming the root makes global verification explicit without changing the burn signature or adding a separate checker attribute.

`#[cauldron(root, ...)]` emits an unconditional type-checking assertion for its reachable static graph. The assertion is checked even if no application constructor is called. An ordinary declarative module emits reusable metadata; its dependency completeness is verified in each root that reaches it. It must not fail simply because it needs a global supplied by a future root.

A root contract is tied to that declared graph. Reusing a non-root module as a runtime root, or calling a dynamic builder, retains runtime validation and does not acquire the compile-time guarantee. Using one marked root as an imported child of another marked root is rejected in this first version; modules must be reusable ordinary modules and each application root declared separately.

## Attribute grammar

- Supported declaration remains a non-generic unit struct.
- `providers = [Type, Output => InjectorType]` registers ordinary and selected injectors respectively. `Output => InjectorType` maps to existing `provide_with::<Output, InjectorType>()`.
- `controllers = [Type]`, `imports = [ModuleType]`, and `exports = [OutputType]` contain Rust type paths, including qualified paths. Imports name unit module types, not runtime expressions.
- `global` makes this module's exported providers available throughout its reachable root graph.
- `root` enables complete verification of the declared application graph.
- `#[cauldron()]` is an empty declarative module. Bare `#[cauldron]` preserves the legacy expansion and manual `impl Cauldron` contract. The parser must distinguish these forms explicitly.
- List options default to empty; flags default to false. Unknown options, repeated options, duplicate authored list entries, unsupported shapes, and malformed binding syntax have macro diagnostics at the authored syntax.
- Entries containing aliases or different paths to the same type remain subject to type checking and existing runtime ownership checks; token comparison must not pretend to resolve Rust type identity.
- Whole declarations may use existing supported conditional-compilation placement. Per-entry runtime expressions and per-entry `cfg` are outside this first grammar.

## Injection and compatibility

`#[inject]` is a parameterless field helper consumed by `#[burner]`, `#[storage]`, and managed controller expansion. It is not an independent constructor-generating attribute. All three macros retain their existing output handle and dependency-tuple semantics.

For this additive release, fields without the marker continue to be injected exactly as before. Marked and unmarked fields may coexist and all count as dependencies. The marker documents intent; it does not silently default or ignore unmarked fields. Ordinary stored-state fields, named tokens, optional injection, and `#[inject(...)]` arguments are outside scope.

Manual `Injector<Output>` implementations remain supported. Verification reads the actual `Injector::Dependencies` associated tuple rather than re-deriving dependency types from the source fields. Existing tuple arity limits remain sixteen dependencies.

Bare legacy modules, manual `Cauldron` implementations, builder methods, native value overrides, and the conventional burn entry point retain their source contracts. Declarative modules generate `impl Cauldron`; users must not also write that implementation.

A declarative graph requiring compile-time verification may import only modules exposing the new metadata contract. Legacy modules remain usable through legacy roots, including declarative infrastructure modules through their generated `Cauldron` implementation. There is no silent downgrade of a marked root into partial runtime-only verification.

## Scope and global contract

A consumer can inject an output provided by its own module, exported by a directly imported module, or exported by a reachable global module. Merely reaching a non-global module transitively does not make its providers visible. Exports must refer to locally registered provider outputs; controllers cannot be exported. Module re-export syntax is outside this initial API.

Global visibility is scoped to the marked root's import closure, not the process-wide inventory. An unrelated linked global module cannot satisfy a dependency. Nested imports can bring a global module into the closure. Multiple roots in one binary are independently checked.

Repeated reachability of the same global module through a valid diamond must not introduce inference ambiguity or duplicate construction. Existing ownership and duplicate-output rules remain authoritative; this change must not permit conflicting provider owners. Globals with the same output and different owners are not a mechanism for selecting an implementation.

The verifier must prove dependencies for every registered provider and controller reachable from the root. Framework-provided `Config` is an explicit intrinsic capability matching the existing builder. Other runtime auto-configuration contributions must not be universally assumed present. A declared output may still be overridden with a native value at runtime, but an override does not erase that output's declared static constructor requirements in the checked contract.

## Engine mechanism and feasibility gate

Use additive, hidden public metadata required for cross-crate macro expansion:

```rust
pub trait ModuleContract: Cauldron {
    type Imports;
    type Providers;
    type Exports;
    type Global;
}

pub trait CheckGraph<Root, Proof>: ModuleContract {}
```

Metadata lists describe output/injector pairs, controller injectors, imports, and export types. Generated runtime registration uses the same declaration as this metadata; there must not be two separately authored registrations.

The root assertion requests a `CheckGraph<Root, Proof>` implementation whose proof is inferred. Indexed membership witnesses distinguish local, direct-import, and reachable-global capabilities. Cross-crate Rust types, not stringified type names, establish identity. Implementation may add narrowly scoped helper traits and list types behind `__private`; it must not add a public proof argument to `Furnace::burn` or builder methods.

This mechanism is a candidate, not a claim that Rust inference can trivially solve graph membership. The first implementation task must demonstrate global resolution, valid diamonds, local/global overlap, direct/global overlap, and root isolation on Rust 1.94. If candidate proofs are ambiguous or cannot represent these cases, stop before product migration, record the limitation, and revise the mechanism. Do not weaken visibility, require user-authored proof types, or silently disable global checking to get tests passing.

Static import cycles and constructor cycles are not promised new structured compile-time diagnostics. Invalid recursive graphs may be rejected by the compiler; the existing runtime diagnostics remain available on legacy paths. Acyclic valid graphs must compile without application changes to recursion limits.

## DatabaseCauldron contract

Keep the existing `sea-orm-postgres` feature gate and exported type paths. Replace its manual registration with:

```rust
#[furnace_rs_core::cauldron(
    global,
    providers = [
        DatabaseFactory,
        SeaOrmPostgres,
        DatabaseConnection => SeaOrmDatabaseInjector,
    ],
    exports = [DatabaseConnection],
)]
pub struct DatabaseCauldron;
```

Keep the actual constructors and descriptors: `DatabaseFactory` has no inputs; `SeaOrmPostgres` requires `Config`; the native connection injector requires `(DatabaseFactory, SeaOrmPostgres)`. The connection retains its readiness/shutdown hook and its existing native `DatabaseConnection` output. Factory and connector remain internal to the module.

Importing `DatabaseCauldron` into the root supplies the connection capability to all modules in that root. Omitting the import produces a compile failure for a consumer requiring it, without attempting configuration loading or a database connection. Successful compilation does not guarantee a reachable database or valid credentials.

## LoggerCauldron contract

Replace its manual registration with:

```rust
#[crate::core::cauldron(
    global,
    providers = [Logger],
    exports = [Logger],
)]
pub struct LoggerCauldron;
```

Preserve its public type path, existing no-input `Logger` injector and descriptor, default console-backed implementation, and native `Logger::new(...)` overrides. Importing it into one root must not satisfy consumers in another root. Compile fixtures must prove both missing-import failures and cross-crate success when database and logger are imported together.

## Runtime guarantees and exclusions

Keep runtime preflight before constructor execution. Compile-time checking verifies static wiring and module access, not configuration values, connectivity, readiness, shutdown outcomes, route validity, or arbitrary runtime resolution calls.

Do not replace inventory, alter HTTP routing or persistence behavior, change instance sharing or ownership semantics, introduce a new container, or implement dynamic registration analysis. No blanket claims of zero startup failures or runtime performance gains.

## Acceptance and beta delivery

1. Existing legacy compile fixtures and runtime tests still pass without source edits.
2. New declarative local/import/export/binding/controller graphs compile; missing or inaccessible dependencies fail under `cargo test` compile fixtures.
3. Root-relative global success, missing global, unrelated globals, two-root isolation, valid diamonds, and local/global overlap are verified on stable Rust and Rust 1.94.
4. Database and logger expose the static contract across crate boundaries while retaining their runtime behavior, feature gates, native overrides, and lifecycle.
5. Renamed facade/core dependencies resolve correctly; core remains usable without HTTP, SeaORM, or a mandatory Tokio runtime.
6. A standalone `example/compile-time-di/` project demonstrates `#[inject]`, declaration syntax, global imports, and a compile-fail missing dependency test. Expected-failure fixtures are not broken ordinary binaries.
7. Prepare workspace version `1.1.0-beta.1`, synchronized internal pins and example lockfiles, with migration notes and explicit guarantee boundaries. No publishing, tagging, merging, or pushing is part of this task.

Production dependencies remain unchanged. `trybuild` may be added as a core dev dependency. Rust MSRV remains 1.94 and edition remains 2024. All public API examples must be tested in the appropriate feature configurations.
