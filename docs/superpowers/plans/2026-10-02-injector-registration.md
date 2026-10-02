# Injector Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `#[element]` factories with typed asynchronous injectors and explicit cauldron output bindings without weakening DI validation or lifecycle cleanup.

**Architecture:** `Injector<T = Self>` receives a declared dependency tuple and returns `Result<T>` asynchronously. Static constructor descriptors bridge that contract into the existing graph and lifecycle engine; rooted selection takes the descriptor selected by each cauldron registration. Managed macros generate the same contract and retain catalog discovery for unrooted applications.

**Tech Stack:** Rust edition 2024, stable Rust and MSRV 1.94, existing inventory/syn/quote/Tokio/Axum/SeaORM dependencies. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-02-injector-registration-design.md` (approved).

## Global Constraints

- Work on the current assigned branch `refactor/furnance-injector`; commit each completed task locally. Do not push, publish, or bump versions.
- Preserve `.provide::<Service>()`, `.provide_with::<Output, Implementer>()`, `.controller::<Controller>()`, imports, exports, and `.global()`.
- `inject` returns a `Send` future with output `Result<T>`; dependencies are `()` or tuples of one through sixteen `Clone + Send + Sync + 'static` elements.
- Registration, metadata discovery, inspection, and invalid preflight must perform no construction, lifecycle attachment, or I/O.
- Preserve the existing single reachable owner per output, scope validation, auto-configuration, native output types, and lifecycle ordering/cleanup.
- Manual injector registrations require a cauldron root; unrooted catalog/focused builds discover macro-generated or deliberately authored integration descriptors.
- Remove `element` with no deprecated alias. Preserve HTTP controllers, seals, endpoint skips, crate aliases, and feature boundaries.
- Keep historical specifications/reports intact. Live database checks use only temporary resources and clean them up.

## Review Focus

1. Explicit trait binding when a different linked constructor has the same output: use the selected implementer without false ambiguity (Task 2).
2. Repeated inspection of generic descriptors: stable metadata, no heap leaks or constructor calls (Tasks 1–2).
3. An externally supplied registered output: skip both construction and lifecycle attachment, while retaining ownership checks (Task 2).
4. A connection acquired before a later constructor or hook fails: close the acquired resource exactly once using existing cleanup semantics (Tasks 4–6).
5. Managed self-type dependencies and cloned controller handles: preserve normalized dependency types, sharing, and route callbacks (Task 3).

---

## File responsibilities

- New `crates/furnace-rs-core/src/injector.rs`: public construction contract, typed tuple resolution, and static constructor bridge.
- Existing core `descriptor.rs`: dependency runtime names; `cauldron.rs`: selected constructor references; `graph/scope.rs`: authoritative registration selection. Keep graph ownership policy in existing graph files.
- Existing core/common managed macro files: generate injector implementations and reusable descriptors, without duplicating construction logic.
- Existing common/persistence integration files: injectors for native outputs and deliberately discoverable integration descriptors.
- New core/common integration tests and facade UI fixtures: behavior and compile-contract coverage. Existing test suites remain the regression gates.
- Active docs/examples: consumer migration. A final verification report records commands, results, review rulings, and any genuine limitations.

### Task 1: Typed Injector contract and static descriptor bridge

**Files:** Create core `src/injector.rs` and `tests/injector.rs`; modify core `src/lib.rs`, `src/descriptor.rs`; test existing `tests/descriptor.rs`.

**Interfaces:**
- Produce public `Injector<T = Self>` with `type Dependencies: InjectionDependencies`, `fn inject(Self::Dependencies) -> impl Future<Output = Result<T>> + Send`, and `fn lifecycle(T) -> LifecycleResource<T>` defaulting to `LifecycleResource::new`.
- Produce `InjectionDependencies::descriptors() -> &'static [DependencyDescriptor]` and `InjectionDependencies::resolve(&ConstructionContext<'_>) -> Result<Self>` for unit and tuples through arity sixteen. Seal dependency-shape implementations so the published shape limit is enforceable.
- Produce doc-hidden `injector_descriptor<T, I>() -> &'static ProviderDescriptor`, with `T: Send + Sync + 'static`, `I: Injector<T>`. Add a doc-hidden overridable `Injector::descriptor()` defaulting to this bridge so managed and integration providers can reuse their richer descriptor metadata.
- Extend `DependencyDescriptor` with an optional runtime-name callback while retaining its existing constructor API for low-level consumers.

- [x] Write `injector.rs` tests: zero dependencies, `(Config,)`, a two-handle tuple, sixteen dependencies, async error propagation, and a lifecycle attachment counter. Assert exact output type identity and typed dependency names.
- [x] Run `cargo test --locked --offline -p furnace-rs-core --test injector`; confirm missing API failures before implementation.
- [x] Implement tuple resolution and generic associated-constant descriptors, including ordinary and lifecycle constructor adapters. Static descriptors must not leak allocations; generated function pointers may refer to generic type parameters.
- [x] Run the new test on stable and `cargo +1.94.0 test --locked --offline -p furnace-rs-core --test injector`; include repeated descriptor identity checks and assert no construction during descriptor access. Run existing descriptor tests.
- [x] Commit: `feat(core): add typed asynchronous Injector construction`.

### Task 2: Authoritative cauldron injector registrations

**Files:** Modify core `src/cauldron.rs`, `src/graph/{cauldron,scope}.rs`, `src/builder.rs`, core-macros `src/cauldron.rs`; create core `tests/injector_registration.rs`; extend `tests/cauldron_scope.rs`.

**Interfaces:**
- Consume Task 1's typed descriptor bridge.
- Produce `CauldronRegistration::provide_with<T, I>(self) -> Self` with `I: Injector<T>` and matching cauldron macro entry method.
- Intermediate compatibility: retain current legacy `provide<T>()` until managed and integration providers have migrated. Task 5 makes `T: Injector<T>` authoritative for ordinary registrations.
- Retain an optional `&'static ProviderDescriptor` in each registration member. Explicit descriptors take precedence over same-output linked descriptors; ownership, controller classification, and overrides still validate normally.

- [x] Add tests for a plain manually authored trait injector, private dependency in the owning cauldron, exported dependency via direct import, and inaccessible dependency. Assert registry resolution returns the authored trait result.
- [x] Add missing-dependency and cycle tests with atomic constructor counters remaining zero. Add duplicate output/owner cases using different implementers and an unrelated broken cauldron excluded from the selected root.
- [x] Run the new registration test and confirm failure before adding the registration method.
- [x] Integrate selected descriptors into rooted scope before auto-configuration and HTTP preflight; preserve override suppression and registration locations in diagnostics. Do not collect registrations from unreachable cauldrons into the root's provider catalog.
- [x] Test competing linked same-output metadata, repeated analysis, a supplied output that suppresses `inject` and `lifecycle`, and a trait binding that does not implicitly register its concrete implementer. Run core cauldron and auto-configuration tests.
- [x] Commit: `feat(core): bind cauldron outputs to explicit injectors`.

### Task 3: Managed services and controllers implement Injector

**Files:** Modify core-macros `src/managed.rs`, common-macros `src/controller/managed.rs`, their existing macro test support, facade `src/lib.rs`; create common `tests/injector_controller.rs`; extend core injector tests and facade `tests/ui/pass/injector.rs`.

**Interfaces:**
- Generated `Injector<Self>` resolves managed fields through the Task 1 tuple contract and constructs the same Arc-backed handle as today.
- `Injector::descriptor()` returns the managed descriptor, preserving role, namespace, source location, catalog submission, controller callbacks, and focused requirements. Generated descriptor constructors delegate to `inject` and `lifecycle`.
- Reexport `Injector` and `InjectionDependencies` through core and facade/prelude without enabling unrelated features.

- [x] Add behavior tests for a managed burner/storage, cloned shared handles, and a managed controller consuming a manual trait binding in a rooted cauldron. Assert direct endpoint requests return the injected implementation's output.
- [x] Add self-type normalization and focused unrooted managed construction regression assertions. Run the focused new tests before macro changes and capture the missing Injector failures.
- [x] Generate dependency tuples and the injector contract in both managed expansions. Reject fields beyond sixteen with a focused macro diagnostic rather than incidental Rust tuple errors. Preserve generated handle and route APIs.
- [x] Run core/common macro tests, new controller/injector tests, direct-controller and optional-seal tests, and the facade pass fixture on stable/MSRV.
- [x] Commit: `feat(macros): generate injectors for managed providers and controllers`.

### Task 4: Native integrations and lifecycle migration

**Files:** Modify common `src/logger/mod.rs`, common auto-configuration provider descriptors as required, persistence `src/sea_orm/mod.rs`, existing persistence tests; create core `tests/injector_lifecycle.rs`. Migrate other official element declarations identified by `rg -n 'element' crates/*/src`.

**Interfaces:**
- Consume `Injector<NativeOutput>` and `provide_with<NativeOutput, Constructor>()`.
- A local SeaORM constructor declares `(DatabaseFactory, SeaOrmPostgres)` and returns the native `DatabaseConnection`; its lifecycle override contributes the existing `SeaOrmLifecycle` infrastructure hook.
- Official discoverable provider descriptors delegate to injectors and preserve conditional/focused integration requirements. Keep ordinary helper functions when existing direct callers need them.

- [ ] Add lifecycle event assertions: inject once, attach once, infrastructure before application hooks, reverse shutdown, cleanup after a later construction failure, rollback after startup failure, and skip when overridden.
- [ ] Run the lifecycle test and relevant persistence tests before migration; capture missing/new contract failures.
- [ ] Migrate logger and persistence constructors, bindings, and catalog bridges without changing native output identities or activation rules. Retain current redaction and hook ownership strings.
- [ ] Run core lifecycle/construction-failure and persistence factory/connector/module tests, common auto-configuration/focused/scoped tests. Assert native resource values resolve under their existing output types.
- [ ] Commit: `refactor(integrations): construct native resources through Injector`.

### Task 5: Remove element and migrate public consumers

**Files:** Remove core-macros `src/provider.rs` and obsolete factory-only macro tests; modify core/common/facade reexports, core `src/cauldron.rs`, examples `protected-route`, `posts-crud`, `basic`, CLI scaffold fixtures, active README/architecture/changelog/migration guide. Migrate existing provider/factory integration tests and UI fixtures; preserve tests exercising equivalent behavior.

**Interfaces:**
- Final `provide<T>()` requires `T: Injector<T>` and records `T::descriptor()`.
- Public `element` no longer exists. Manual factories are local constructor structs implementing `Injector<Output>`; managed providers use their generated injector.
- Document manual rooted discovery, tuple arities, trait output conversion, async construction, external native types, lifecycle overrides, and preserved public/sealed controller behavior.

- [ ] Create UI failures for wrong bound output, seventeen manual dependencies, non-clone dependencies, non-Send futures, and removed `element`. Add pass cases for plain services, async trait bindings, one dependency, and lifecycle overrides.
- [ ] Run UI cases to establish expected new failures; migrate obsolete element-specific fixtures instead of retaining tests for removed behavior. Preserve compile coverage for concrete outputs and constructor errors.
- [ ] Switch ordinary registrations to the typed contract; migrate all product/test/example factory consumers, remove the macro exports and implementation, and update external-alias consumers. Ensure non-registration helper functions remain callable where needed.
- [ ] Run full facade UI/provider tests, CLI scaffold-consumer tests, external aliases, and three standalone examples. Search active Rust source/docs for leftover macro usages; exclude historical records and unrelated serde sequence elements.
- [ ] Stage new package files, then run `bash script/verify-package-contents.sh`. Commit: `refactor!: replace element factories with Injector registrations`.

### Task 6: Full verification, live database checks, and branch review

**Files:** Update this plan's completion checkboxes and the approved spec's implementation status; create `docs/superpowers/reports/2026-10-02-injector-registration-verification.md`.

**Interfaces:** No additional public API. Produce reviewable evidence for the complete change and document any remaining limitations.

- [ ] Finish all source edits before CLI watcher/full-suite tests. Run `cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings`; require exit zero.
- [ ] Run stable and MSRV workspace suites sequentially: `cargo test --locked --offline --workspace --all-features` and `cargo +1.94.0 test --locked --offline --workspace --all-features`. Preserve logs and report actual failures/ignored database cases rather than assuming counts from prior runs.
- [ ] Run strict docs with `RUSTDOCFLAGS='-D warnings' cargo doc --locked --offline --workspace --all-features --no-deps`, `cargo test --locked --offline --workspace --all-features --doc`, and feature checks listed below. Repeat standalone example/package payload gates only if later changes invalidate Task 5's evidence.
- [ ] Run four live PostgreSQL tests on both stable and MSRV with `--features sea-orm-postgres --test postgres -- --ignored --test-threads=1`. Reuse the documented temporary-cluster procedure from `docs/superpowers/reports/2026-10-02-postgres-verification.md`; preserve logs, stop the cluster, and remove only its owned temporary directory.
- [ ] Run `python3 -m unittest discover -s benchmark/tool -p 'test_*.py'` with the documented dedicated database and local posts-crud app environment; require the database recovery cases to run rather than skip. Stop all resources created for verification.
- [ ] Request one fresh whole-branch review of the implementation against the spec and plan. For native execution, follow executing-plans requirements for reviewer dispatch, model selection, rulings, and scratch cleanup. Fix accepted findings, rerun affected checks, and record every finding's disposition.
- [ ] Update the verification report and completion status, commit `docs: record Injector migration verification`, and report final commits, checks, limitations, review rulings, and working-tree state. Do not merge or push.

## Execution handoff

Feature checks (all must exit zero):

```bash
cargo check --locked --offline -p furnace-rs-core --no-default-features
cargo check --locked --offline -p furnace-rs-persistence --no-default-features
cargo check --locked --offline -p furnace-rs-persistence --no-default-features --features sea-orm-postgres
cargo check --locked --offline -p furnace-rs-common --no-default-features --features http
cargo check --locked --offline -p furnace-rs-common --no-default-features --features jwt
cargo check --locked --offline -p furnace-rs-common --no-default-features --features cookies
cargo check --locked --offline -p furnace-rs --no-default-features
cargo check --locked --offline -p furnace-rs --no-default-features --features http,runtime-tokio
cargo check --locked --offline -p furnace-rs --no-default-features --features cookies
```

Recommended method: native execution in this session, with one fresh final reviewer. These six tasks depend on the same constructor/descriptor interfaces; a single implementer can keep that contract consistent while migrating consumers incrementally.

The written plan requires user review before implementation. No product code has been changed in this planning stage.
