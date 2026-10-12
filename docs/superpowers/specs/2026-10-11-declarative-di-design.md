# Declarative cauldron registration and compile-time DI

Status: approved, implementation in progress. The first witness mechanism was rejected; the revised marker/callback mechanism passed the feasibility matrix. The user approved the beta import restriction recorded below.

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
- `controllers = [Type]`, `imports = [ModuleType]`, and `exports = [OutputType]` contain Rust paths, including qualified paths. Imports name unit cauldron declarations or their re-exports/`use ... as ...` aliases, not runtime expressions. For the beta, module aliases declared using `type Alias = Module` are unsupported in declarative imports; the user explicitly approved this restriction. Output/dependency type aliases remain supported, and the fluent API retains its existing module type-alias support.
- `global` makes this module's exported providers available throughout its reachable root graph.
- `root` enables complete verification of the declared application graph.
- Bare `#[cauldron]` and zero-argument `#[cauldron()]` both preserve the legacy expansion and manual `impl Cauldron` contract. Use `#[cauldron(providers = [])]` for an empty declarative module. Attribute macros receive identical empty argument streams for the two zero-argument forms; preserving legacy behavior takes precedence over the initial proposal to distinguish them.
- List options default to empty; flags default to false. Unknown options, repeated options, duplicate authored list entries, unsupported shapes, and malformed binding syntax have macro diagnostics at the authored syntax.
- Entries containing aliases or different paths to the same type remain subject to type checking and existing runtime ownership checks; token comparison must not pretend to resolve Rust type identity.
- Whole declarations may use existing supported conditional-compilation placement. Per-entry runtime expressions and per-entry `cfg` are outside this first grammar.

## Injection and compatibility

`#[inject]` is a parameterless field helper consumed by `#[burner]`, `#[storage]`, and managed controller expansion. It is not an independent constructor-generating attribute. All three macros retain their existing output handle and dependency-tuple semantics.

For the 2.0 prerelease, every named dependency field requires the marker. Unmarked fields fail compilation. This is a source-breaking migration; all marked fields count as dependencies. Ordinary stored-state fields, named tokens, optional injection, and `#[inject(...)]` arguments are outside scope.

Manual `Injector<Output>` implementations remain supported. Verification reads the actual `Injector::Dependencies` associated tuple rather than re-deriving dependency types from the source fields. Existing tuple arity limits remain sixteen dependencies.

Bare legacy modules, manual `Cauldron` implementations, builder methods, native value overrides, and the conventional burn entry point retain their source contracts. Declarative modules generate `impl Cauldron`; users must not also write that implementation.

A declarative graph requiring compile-time verification may import only modules exposing the new metadata contract. Legacy modules remain usable through legacy roots, including declarative infrastructure modules through their generated `Cauldron` implementation. There is no silent downgrade of a marked root into partial runtime-only verification.

## Scope and global contract

A consumer can inject an output provided by its own module, exported by a directly imported module, or exported by a reachable global module. Merely reaching a non-global module transitively does not make its providers visible. Exports must refer to locally registered provider outputs; controllers cannot be exported. Module re-export syntax is outside this initial API.

Global visibility is scoped to the marked root's import closure, not the process-wide inventory. An unrelated linked global module cannot satisfy a dependency. Nested imports can bring a global module into the closure. Multiple roots in one binary are independently checked.

Repeated reachability of the same global module through a valid diamond must not introduce inference ambiguity or duplicate construction. Existing ownership and duplicate-output rules remain authoritative; this change must not permit conflicting provider owners. Globals with the same output and different owners are not a mechanism for selecting an implementation.

The verifier must prove dependencies for every registered provider and controller reachable from the root. Framework-provided `Config` is an explicit intrinsic capability matching the existing builder. Other runtime auto-configuration contributions must not be universally assumed present. A declared output may still be overridden with a native value at runtime, but an override does not erase that output's declared static constructor requirements in the checked contract.

## Engine mechanism and feasibility gate

Use additive, hidden public metadata and marker traits under `__private`:

```rust
pub trait ModuleContract: Cauldron { type Metadata: ModuleMetadata; }
pub trait ModuleMetadata { const KEY: &'static str; }
pub trait SlotAt<const INDEX: usize> { type Slot: SlotKey; }
pub trait ImportAt<const INDEX: usize> { type Metadata: ModuleMetadata; }
pub trait Represents<T> {}
pub trait Lookup<T, Proof> {}
pub trait CheckModule<Scope, Proof> {}
```

Each declared output has an opaque public marker implementing `Represents<ActualOutput>`. This keeps actual private types out of public associated metadata while retaining Rust type identity for dependency checks. Module check implementations read actual `Injector::Dependencies`; private constructors may appear in implementation bounds without becoming public associated types. Imports expose public metadata proxies through associated-type equality bounds, avoiding leakage of private imported module types.

Each declarative cauldron also supplies a same-name function-like metadata macro. Ordinary `use` aliases and re-exports carry both type and macro namespaces; `type` aliases do not, explaining the agreed beta restriction. An import's macro callback captures its description at the importing declaration's original scope. The description embeds only opaque metadata projections, and parent descriptions carry the reachable child records. Its public proxy is addressed through the importing metadata's `ImportAt` projection, so expansion at the root does not need access to private source modules or their import names.

A staged internal bootstrap collects child descriptions and canonical declaration IDs. Each parent removes duplicate child records only after emitting Rust type-equality guards for every merged metadata projection. The root's collector checks the resulting closed graph. No root invocation accesses a macro-expanded macro-export helper by an absolute same-crate path. The root generates separate scopes containing locals, direct imported exports, reachable global exports, and intrinsic Config. Visibility is evaluated using closed constant expressions, not generic const expressions. Repeated slots in a scope are disabled before trait lookup. The resulting balanced marker tree admits one membership proof for each valid output; local/global and direct/global access to one owner are normalized rather than presented as competing candidates.

Declaration/slot keys are indexing aids, not type-identity substitutes. Every repeated declaration ID requires an unconditional `SameWhen<true, A, B>` proof that its metadata proxy types are equal. Equal normalization keys likewise require equal marker types. Rust type equality therefore protects deduplication against identifier collisions; `Represents<ActualOutput>` establishes output identity. No type-name hashing, unsafe TypeId conversion, source-tree scanning, or process-global proc-macro registration state is used.

Generated runtime registrations and static metadata come from one declaration. Public burn/builder signatures stay unchanged; the root emits unconditional checks for every reachable module, including unused roots. Hidden proof arguments remain generated implementation details.

Static import cycles and constructor cycles are not promised new structured compile-time diagnostics. Legacy runtime validation retains its structured diagnostics for those cases; declarative import cycles may be rejected during metadata expansion. Balanced scope trees avoid linear-depth membership proofs; the callback protocol still operates within Rust's compiler expansion limits and must be covered by representative multi-module tests.

### Task 1 feasibility result (2026-10-11)

The approved documentation was committed as `a83c803`. A dependency-free throwaway Cargo project at `/tmp/furnace-di-contract-probe/` tested the proposed inferred proof selection on Rust 1.94.0 and stable Rust 1.96.0. Both compilers produced the same outcomes:

| Probe | Required outcome | Actual outcome |
| --- | --- | --- |
| Local provider | Compile | Compiles |
| Direct imported export | Compile | Compiles |
| One reachable nested global | Compile | Compiles |
| Intrinsic Config | Compile | Compiles |
| Missing provider | Reject | E0277 |
| Global reachable only from another root | Reject | E0277 |
| Two-root isolation | Reject the missing root | E0277 |
| Diamond reaching the same global | Compile | E0283: ambiguous proof |
| Local/global access to the same owner | Compile | E0283: ambiguous proof |
| Direct-import/global access to the same owner | Compile | E0283: ambiguous proof |
| Two globals with different exported outputs | Compile | E0283: ambiguous proof |

The reduced model uses these implementations:

```rust
impl<M, R, T> Resolves<R, T, Local> for M
where M: Provides<T> {}

impl<M, R, T, I, P> Resolves<R, T, Direct<I, P>> for M
where M: Imports<I, P>, I: Exports<T> {}

impl<M, R, T, I, P> Resolves<R, T, Global<I, P>> for M
where R: Reachable<I, P>, I: GlobalExports<T> {}

fn assert_dependency<M, R, T, P>()
where M: Resolves<R, T, P> {}
```

The intended assertion supplies `P = _`. Rust does not choose one of several valid proof types, and does not uniquely select a reachable module by searching all its nested export bounds. The two-global case fails even without duplicate outputs or diamond imports. Associated module lists and a graph-validation wrapper cannot by themselves remove this ambiguity at dependency selection.

Control experiments explicitly selecting `Local`, `Direct<DatabaseModule, First>`, or `Global<DatabaseModule, Left>` compile on both toolchains. This confirms the modeled dependency is present and isolates inference as the blocker; requiring those types from application authors would violate the approved API. This result rejects this candidate mechanism, not compile-time DI in general.

An alternative using safe constant equality of `TypeId::of::<T>()` also fails on the MSRV: `PartialEq` is not a stable const trait (E0658). Do not rely on unsafe transmutation of TypeId's unspecified representation, nightly features, stringified type names, or source-tree scanning to bypass that boundary.

Reproduction while the throwaway project exists:

```sh
cargo +1.94.0 check --offline --manifest-path /tmp/furnace-di-contract-probe/Cargo.toml --bin nested_global
# Expected exit 0.
cargo +1.94.0 check --offline --manifest-path /tmp/furnace-di-contract-probe/Cargo.toml --bin two_global_outputs
# Observed exit 101, E0283.
cargo +1.94.0 check --offline --manifest-path /tmp/furnace-di-contract-probe/Cargo.toml --bin direct_global_explicit
# Expected exit 0, diagnostic control only; not an acceptable public API.
```

Next design requirement: generate or otherwise establish a canonical output-to-owner lookup before dependency verification, deduplicate repeated paths to one module, and choose visibility without existential inference among competing witnesses. Cross-crate metadata, private types, aliases, and root-specific globals must remain covered. A macro metadata/callback approach is a candidate for investigation, not a verified replacement. Revise the mechanism and repeat the feasibility matrix before Tasks 2–6; all application syntax and infrastructure requirements above remain the intended contract.

No engine, infrastructure, version, or example files were changed by Task 1. Its gate deliberately prevents shipping the rejected checker or declaring a weaker runtime-only migration complete.

### Revised feasibility result (2026-10-11)

A separate throwaway workspace at `/tmp/furnace-di-metadata-probe/` implements the marker tree, constant visibility/normalization, compiler identity guards, cross-crate same-name metadata callbacks, and generated root assertions. It has separate core, macro, infrastructure, and application crates; the application renames its infrastructure dependency. It models the infrastructure contract; actual Furnace database/logger migration remains Task 5.

Thirty checks across Rust 1.94.0 and stable 1.96.0 matched their expected outcomes:

- Valid: two global outputs, `use` aliases, nested global reachability, diamond with direct/global overlap, private providers and private imported modules, direct non-global exports.
- Invalid: missing database, missing logger, a linked but unreachable global, root isolation, a non-global output without a direct import, transitive non-global access, an unexported private global member, and a marker-key identity collision.
- Module `type` aliases were rejected as expected under the user-approved beta restriction. Dependency/output aliases, including a native `Arc<String>` output owned outside the infrastructure crate, worked.

Two additional negative controls deliberately reused a declaration ID for distinct database/logger metadata proxies. Both compilers rejected this with `SameWhen<true, A, B>` rather than silently merging different modules. Results and compiler logs are retained in the throwaway workspace, including `results.json`.

This passes the revised feasibility gate for the agreed beta contract. Production tasks must still test generated declarations, macro hygiene/diagnostics, HTTP controllers, actual infrastructure integration, and representative graph size; prototype results do not establish those features as implemented.

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
7. Prepare workspace version `2.0.0-beta.1`, synchronized internal pins and example lockfiles, with migration notes and explicit guarantee boundaries. No publishing, tagging, merging, or pushing is part of this task.

Production dependencies remain unchanged. `trybuild` may be added as a core dev dependency. Rust MSRV remains 1.94 and edition remains 2024. All public API examples must be tested in the appropriate feature configurations.

Implementation refinement (Task 3): same-crate diamond fixtures exposed Rust's crate-level macro-export absolute-path lint, which the initial external-crate prototype did not exercise. Capturing and embedding child descriptions at each parent declaration avoids that access entirely; no consumer lint suppression is added. The typed scope, visibility and membership mechanism stays unchanged.

Review refinement: explicit Config outputs (including aliases and imported exports) must not compete with intrinsic Config. The root constructs an inferred zero-sized scope tree, with autoref dispatch on concrete `Represents<Config>` slot markers selecting Nil for those redundant memberships. This uses Rust trait identity and leaves every declared constructor's dependency check intact. No type-name comparison or consumer syntax is added.

## 2026-10-12 approved 2.0 prerelease amendment

The user replaced the additive beta policy with a major release: managed burner, storage and controller dependency fields now require `#[inject]`. Unmarked fields are compile errors. Preserve unit structs, manual `Injector::Dependencies`, runtime construction, and fluent registrations. Synchronize workspace and local dependency pins to 2.0.0-beta.1, migrate examples/templates/fixtures, and review checker soundness before committing the release preparation. Historical benchmark reports retain their measured version labels; obsolete release notes are removed.
