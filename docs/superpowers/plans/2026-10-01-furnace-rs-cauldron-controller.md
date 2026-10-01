# furnace-rs, Cauldrons, Direct Controllers, and Seals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the approved furnace-rs package family, explicit cauldrons, inherent controller endpoints, and controller-wide typed seals without weakening dependency ownership or startup validation.

**Architecture:** Preserve the concrete-output DI graph and rename its module topology to cauldrons. Separate controller DI declarations from inherent endpoint descriptors, attach static seal declarations to the controller, and select all HTTP/security metadata through explicit cauldron membership. Add a framework-neutral preflight callback boundary so HTTP endpoint/security validation completes before any provider or default constructor executes.

**Tech Stack:** Rust 2024, existing Rust 1.94 minimum, syn/quote/proc-macro-crate, inventory, Tokio, Axum, serde, trybuild, and current workspace tooling.

**Spec:** [Approved design](../specs/2026-10-01-furnace-rs-cauldron-controller-design.md).

**Status:** Spec approved by the user. This execution plan awaits review; no product implementation is authorized by the plan's existence alone. Preserve the prior native execution method and user-authorized per-task local commits on `refactor/mads-declaration-module`.

## Global Constraints

- Project/facade package `furnace-rs`; internal packages `furnace-rs-core`, `furnace-rs-core-macros`, `furnace-rs-common`, `furnace-rs-common-macros`, `furnace-rs-persistence`, `furnace-rs-testing`, `furnace-rs-extra`, and `furnace-rs-cli`.
- Recommended facade dependency alias `furnace`; CLI executable `furnace`. Default imports such as `furnace_rs` and arbitrary renamed dependencies also work.
- Keep current package versions, Rust 2024, minimum Rust 1.94, feature names, and existing dependencies. No new dependency or unsafe code.
- Runtime `Furnace`/`FurnaceBuilder`/`FurnaceBurnExt`; module trait `Cauldron`/`CauldronRegistration`; graph/descriptor/node/ownership names use cauldron vocabulary.
- Keep burner/storage/element semantics, factory output normalization, lifecycle semantics, native SeaORM types, semantic provider-kind variants, and focused fixture isolation.
- One owner per rooted output; direct imports and reachable global exports govern access. Supplied registered values preserve ownership while bypassing unused constructors.
- Configuration `furnace.toml`, environment prefix `FURNACE_`, diagnostic prefix `FURNACE` with existing numeric suffixes, official identifiers `furnace.common.*` and `furnace.persistence.*`.
- Private inspection protocol and public CLI JSON schema both version 2. Historical documents remain historical; old product aliases/fallbacks are removed.
- Exactly one annotated inherent endpoint implementation per selected controller. HTTP verbs accept bare attributes or one string path; sync/async handlers support `&self` or no receiver.
- Every direct managed controller implements static `Sealable::seals()`. Empty registration means public; exactly one guard seals every endpoint. No endpoint skips/overrides and no multi-guard pipeline.
- HTTP-only supports unsealed controllers. Passport policies need HTTP+JWT; cookies additionally need cookies. Core-only remains HTTP/database independent.
- Registration/seal/preflight analysis performs no constructor, I/O, lifecycle hook, or bind. Selected duplicate endpoint/security failures occur before construction in builder and standard startup.
- No release/version bump, registry publication, remote repository rename, push, PR, or merge. Check package-name availability again before a future publish; the recorded name check does not reserve names.

## Review Focus

- Renamed/default dependency imports, a consumer with a local module named `furnace`, and renamed transitive macro dependencies must resolve actual package identities without relying on spelling or expansion order (Tasks 1, 3).
- Two independent roots and a diamond import must not share evaluated seal callbacks or admit unregistered malformed controller/strategy metadata (Task 4).
- Shared guard policy types attached to controllers in different cauldrons must use each controller's context, including private supplied JWT overrides and same-name custom strategies (Tasks 4, 5).
- Cfg-disabled endpoints and canonical colon/brace/wildcard paths must produce deterministic metadata, correct captures, and no spurious route conflict (Task 3).
- HTTP preflight must run for a low-level rooted builder and focused fixtures without accidentally validating unrelated inventory or applying an official default before a conflict (Task 4).

## File and interface map

Task 1 renames all `crates/mads*` directories to corresponding `crates/furnace-rs*` directories; subsequent paths below use those new names. Preserve git history through rename-aware moves; do not delete/recreate source trees unnecessarily.

- Core: rename `src/furnace.rs` to `src/cauldron.rs`; modify `descriptor.rs`, `catalog.rs`, `builder.rs`, `graph/`, `auto_configuration/`, `lib.rs`, configuration/runtime docs and tests. Add `src/preflight.rs` for the integration callback boundary.
- Core macros: rename `src/furnace.rs` to `src/cauldron.rs`; update `lib.rs`, `path.rs`, `main.rs`, managed/factory/configuration diagnostics and tests.
- Common macros: keep `controller.rs` as a dispatcher; create `controller/managed.rs` and `controller/endpoints.rs` for struct and inherent-impl responsibilities. Extract shared path/extractor/adapter utilities from `routes.rs` into `endpoint.rs`, then remove the legacy trait expander after migration. Adapt `guard.rs` for unit policy types and remove old trait/method guard parsing after migration.
- Common runtime: add `src/seal.rs` and `src/http_preflight.rs`; modify `route.rs`, `http_scope.rs`, `router.rs`, `passport/{guard,strategy,context}.rs`, `jwt/auto_configuration.rs`, `inspection.rs`, `server.rs`, and exports. Keep authentication, redaction, CORS, and lifecycle implementations intact.
- Facade/testing/persistence: update package identities, re-exports, native/focused consumers, examples, and named built-in cauldrons. Focused selection is an explicit preflight context, not a rootless complete-catalog fallback.
- CLI: update package selection/watch logic, version-2 DTOs and rendering, inspection handshake, generated controller/cauldron templates, and config template filename; delete the route-trait template.
- Scripts/workflows/docs: migrate package lists, versions/pins assertions, archive payload rules, PostgreSQL test env, command names, active guides, migration notes, and CI gates.

## Execution sequencing

Run sequentially on the existing user-assigned branch. Task 1 keeps the existing route declaration behavior under the new brand so the renamed workspace is testable. Tasks 2–5 add the new contracts beside staged legacy route support. Task 6 migrates all consumers and removes staged support in the same commit; never ship aliases or leave generated applications knowingly broken. Each task ends with its own relevant test gate and focused commit. Preserve low-debug local verification settings established in the previous task; do not clean artifacts unless disk pressure requires it.

### Task 1: Rename package identities, branding, runtime, and cauldron topology

**Files:** Move all nine workspace crate directories; modify root/consumer/example `Cargo.toml` and lockfiles, new core/core-macro `src/cauldron.rs`, core graph/builders/configuration, all crate-path resolvers/exports, common startup/inspection, CLI project/watch/output/scaffold config template, scripts/workflows, and owning fixture assertions. Tests live under the renamed `crates/furnace-rs/tests/consumers/`, `path_resolution.rs`, `feature_matrix.rs`, CLI inspection/config/watch tests, and core cauldron tests.

**Interfaces:**
- Produce `Cauldron::register(self) -> CauldronRegistration<Self>` and existing typed registration/access methods under the new names.
- Produce `CauldronDescriptor`, `CauldronGraph`, `CauldronNode`, `CauldronImportEdge`, `ProviderOwnership::cauldron_type_name()`, `GraphAnalysis::cauldron_graph()`, and `Furnace::cauldron_graph()`; rename topology-related accessors consistently.
- Produce `FurnaceBurnExt::burn<C: Cauldron>() -> impl Future<Output = Result<(), HttpRuntimeError>> + Send` and existing builder lifecycle APIs on `Furnace`.
- Macro path lookup uses the actual package names `furnace-rs-core`, `furnace-rs-common`, and `furnace-rs`, and honors Cargo aliases. Generated scaffold manifests use `furnace = { package = "furnace-rs", version = "=0.9.2", ... }`.
- Produce config/environment/diagnostic/identifier changes and version-2 topology fields. Keep legacy route-trait columns only during Tasks 1–5.

- [ ] Write external consumers for default `furnace_rs`, alias `furnace`, arbitrary alias `framework`, and a shadowing local `mod furnace`; assert new declarations/runtime compile. Add config tests loading `furnace.toml`/`FURNACE_SERVER_PORT`, ignoring implicit `mads.toml`/`MADS_SERVER_PORT`, and preserving `.env`/file/env precedence. Add JSON/protocol tests asserting both version constants equal 2 and ownership fields use `root_cauldron`/`cauldron`.
- [ ] Run the new consumers/config tests against the current checkout; confirm missing new package/API/config behavior is the RED cause, recording it before rename.
- [ ] Perform the rename and exact public-name/brand mappings from the spec; migrate existing fixtures required to run the new package names. Update diagnostic strings/snapshots after inspecting actual differences. Update remote branding references in files without mutating Git remote settings or rewriting historical artifacts.
- [ ] Run `cargo test --locked -p furnace-rs-core -p furnace-rs-core-macros`, facade `path_resolution`/`feature_matrix`, and CLI inspection/watch/configuration tests; run `cargo check -p furnace-rs --no-default-features`. Confirm `cargo metadata --locked --no-deps` lists exactly the nine new names and unchanged versions.
- [ ] Commit `feat!: rename the framework to furnace-rs and cauldrons` with this task's fixture changes.

### Task 2: Typed static seal declarations and guard policy types

**Files:** Create `crates/furnace-rs-common/src/seal.rs` and `tests/seal_registration.rs`; modify common `passport/guard.rs`, common-macro `guard.rs`/`lib.rs`, common/facade exports, and facade UI guard policy fixtures.

**Interfaces:**
- Produce HTTP `Sealable: Send + Sync + Sized + 'static { fn seals() -> SealRegistration<Self>; }`.
- Produce `SealRegistration<C: Sealable>::new() -> Self`, `seal<G: GuardPolicy>(self) -> Self`, and doc-hidden `into_definition(self) -> SealDefinition`.
- `SealDefinition::entries() -> &[SealEntry]` exposes ordered records with `guard_type_id() -> TypeId`, `guard_type_name() -> &'static str`, `location() -> SourceLocation`, and `descriptor() -> &'static GuardDescriptor` backed by a static callback. Zero/one entry is valid; more than one is rejected during selected preflight, not while recording.
- Produce doc-hidden public `GuardPolicy: Send + Sync + 'static { fn descriptor() -> &'static GuardDescriptor; }`, implemented by `#[guard(...)]` on a non-generic unit policy struct. Its descriptor retains existing typed principal/adapters/source/authorization policy without an endpoint skip flag.
- `Sealable`, empty `SealRegistration::new()`, and `SealDefinition` are HTTP-only APIs. Gate `GuardPolicy`, `SealEntry`, `entries()`, and typed `.seal::<G>()` methods on HTTP+JWT; the controller macro emits its typed helper only when the owning macro/runtime Passport feature is enabled, without relying on a downstream application feature named `jwt`. Keep the existing route-trait guard grammar only for staged legacy consumers until Task 6. Do not inject/construct guard policy structs or make them DI members.

- [ ] Add `seal_recording_does_not_construct_values`: erase empty/one/two declarations, assert 0/1/2 entries with authored identities/locations and constructor counters still 0. Add unit policy UI pass/fail cases for JWT/cookie gating, roles/permissions/predicates, invalid principal/source, generics, named fields, and `skip`.
- [ ] Run `cargo test -p furnace-rs-common --all-features --test seal_registration` and the new UI cases; record RED missing Sealable/policy type support.
- [ ] Implement the interfaces and unit-policy expansion using the current typed Passport adapters. Keep Sealable/empty recording HTTP-only, and gate actual nonempty Passport policies on HTTP+JWT. Preserve grammar validation at the offending attribute token.
- [ ] Run the focused tests/UI cases plus `cargo check -p furnace-rs --no-default-features --features http` and common JWT-only/cookies feature checks; assert direct policy factory callbacks are never invoked during recording.
- [ ] Commit `feat(http): declare static controller seals and guard policies`.

### Task 3: Direct inherent controller endpoints and typed adapters

**Files:** Modify common-macro `controller.rs`, create `controller/{managed,endpoints}.rs` and `endpoint.rs`, modify `verb.rs`; modify common `route.rs`, `http_scope.rs`, and facade exports. Add common `tests/direct_controller.rs`, facade UI/renamed consumer cases, and macro expansion tests.

**Interfaces:**
- Struct `#[controller]` emits provider/role metadata, a `ControllerDescriptor` with type identity/location and seal callback `fn() -> SealDefinition`, and associated `Controller::seal<G: GuardPolicy>() -> SealRegistration<Controller>` forwarding with caller tracking.
- Impl `#[controller(route = "/user")]` emits `ControllerEndpointDescriptor`: controller TypeId/name, implementation location, static `&[RouteDescriptor]`, and the existing Axum registrar function contract adapted to call inherent methods.
- Produce `ControllerDescriptor::new(type_name: &'static str, type_id: fn() -> TypeId, location: SourceLocation, seals: fn() -> SealDefinition) -> Self` and `ControllerEndpointDescriptor::new(type_name: &'static str, type_id: fn() -> TypeId, location: SourceLocation, endpoints: &'static [RouteDescriptor], registrar: ControllerRegistrar) -> Self`, each with the existing optional namespace builder for report metadata. `ControllerRegistrar` retains `fn(axum::Router, &RouterBuildContext<'_>, &mut ValidatedRouteIter<'_>) -> furnace_core::Result<axum::Router>`; aliases resolve the actual core package.
- `RouteCatalog::controllers() -> Vec<&'static ControllerDescriptor>` collects DI/controller declarations; `RouteCatalog::endpoint_sets() -> Vec<&'static ControllerEndpointDescriptor>` collects inherent implementation descriptors. HTTP selection joins by TypeId and validates exactly one endpoint set for each selected controller.
- Canonical RouteDescriptor paths normalize `/:id` to `/{id}`, preserve native brace/wildcard forms, and join prefixes without double slashes. Bare GET/POST use the base path. Endpoint identity uses controller TypeId, HTTP method, canonical full path, and method name, with no route-trait identity.
- Adapt reusable typed handler generation to call `controller.method(...)` or `Controller::method(...)`, await only async methods, preserve native extractors/IntoResponse, and suppress cfg-disabled metadata/adapters together.

- [ ] Add a dependency-bearing direct controller with `GET /user/{id}`, base GET/POST, sync/async methods, a helper without an HTTP verb, and native extractors. Assert responses, captured ID, prefix joining, and constructor dependency order. Add colon/brace-equivalent conflicts, parameterized paths with the same structural pattern but different capture names, wildcard conflicts, and cfg-disabled endpoint tests. Assert conflicts fail preflight rather than panicking during Axum registration; reject inconsistent capture names across verbs sharing a structural path while preserving legitimate static-vs-parameter routing. Add UI cases for missing Sealable, non-inherent/generic impls, unsupported receivers/parameter layouts, multiple verbs, bare verbs outside an annotated impl, malformed parameter/wildcard paths, and reserved method `seal`.
- [ ] Run the direct controller tests and new UI consumers; record RED because empty-argument struct controllers/inherent endpoint blocks are unsupported.
- [ ] Dispatch `#[controller]` by ItemStruct/ItemImpl. Extract shared parser/adapter routines instead of duplicating the legacy route expander. Emit static descriptor sets and a seal callback without reading source files or sharing mutable proc-macro state. Stage the old argument-bearing struct controller path separately until Task 6.
- [ ] Run macro tests, `direct_controller`, facade path resolution/UI, and existing native extractor/input validation tests; verify default/aliased imports and local namespace shadowing for struct, endpoint, and guard expansions.
- [ ] Commit `feat(http): declare endpoints on controller implementations`.

### Task 4: Selected endpoint/security preflight before construction

**Files:** Create core `src/preflight.rs` and `tests/integration_preflight.rs`; modify core `builder.rs`/`lib.rs`/graph analysis. Create common `src/http_preflight.rs` and `tests/cauldron_http_preflight.rs`; modify common `http_scope.rs`, `route.rs`, `passport/strategy.rs`, `jwt/auto_configuration.rs`, and focused fixture selection.

**Interfaces:**
- Produce core doc-hidden `PreflightValidator = for<'a> fn(&PreflightContext<'a>) -> Vec<Diagnostic>` and inventory `PreflightDescriptor::new(identifier: &'static str, location: SourceLocation, validator: PreflightValidator) -> Self`.
- `PreflightContext<'a>` exposes `config() -> &Config`, `cauldron_graph() -> Option<&CauldronGraph>`, `focus_type_id() -> Option<TypeId>`, and `has_output<T: Send + Sync + 'static>() -> bool`; construct it from the selected virtual analysis, not from a built application.
- Invoke collected validators in identifier order during builder analysis after official-default condition evaluation and virtual-output selection, before applying contributions or constructing providers. Aggregate/deduplicate structured diagnostics in GraphAnalysis; validators never own construction or lifecycle state.
- Common validator identifier `furnace.common.http.preflight` selects rooted controller descriptors through graph membership, focused descriptors through exact fixture TypeId, or the complete controller catalog when unrooted. It evaluates selected seal callbacks once per analysis and validates endpoint sets/canonical conflicts and guard/strategy/JWT access.
- Scoped endpoint occurrences carry controller identity, endpoint identity, optional static policy descriptor, registration location, and controller owner. Policy declaration TypeId/namespace does not grant provider access; use the controller's cauldron for actual managed JWT/strategy dependencies.
- Produce `furnace_common::__private::preflight_http(&PreflightContext<'_>) -> Vec<Diagnostic>` for the registered callback and tests. Keep core independent of HTTP types.

- [ ] Add a preflight rejection with both provider/default constructor counters; assert invalid analysis/build, `FURNACE030` for duplicate endpoints or `FURNACE008` for missing/duplicate endpoint sets/multiple seals, and counters remain 0. Pin rooted and low-level builder paths. Add two independent roots/diamond callback counters and an unrelated malformed unregistered strategy/controller descriptor; assert only the selected scope is validated. Add focused HTTP tests beside unselected conflicting inventory.
The conflict fixture in `cauldron_http_preflight.rs` pins the preconstruction requirement explicitly:

```rust,ignore
let analysis = builder.analyze();
assert!(!analysis.is_valid());
assert!(analysis.diagnostics().iter().any(|d| d.code() == FURNACE030));
assert_eq!(PROVIDER_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
assert_eq!(DEFAULT_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
assert!(builder.build().await.is_err());
assert_eq!(PROVIDER_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
assert_eq!(DEFAULT_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
```

- [ ] Add shared-policy controllers with private/local/direct exported/global exported JwtService and custom strategies; assert private supplied JWT and inaccessible strategies fail with `FURNACE009`/existing strategy diagnostic before constructors. Official unowned defaults remain conditional/ambient. Registered invalid strategies still fail, matching the prior Passport regressions.
- [ ] Run `integration_preflight` and `cauldron_http_preflight`; record RED showing route conflicts run too late or seal metadata is not selected/validated.
- [ ] Implement the generic callback boundary and selected common validator. Refactor current JWT guard requirements to consume the selected seals and controller contexts; preserve default precedence and aggregate safe diagnostics without duplicate copies from evaluator/preflight. Build root/focus contexts explicitly rather than treating focused tests as complete inventory.
- [ ] Run core tests and focused/common scope, auto-configuration, Passport preflight/boundary tests. Verify `cargo tree -p furnace-rs-core --edges normal` contains no Axum/HTTP/JWT/SeaORM additions.
- [ ] Commit `feat(core): preflight selected endpoints and seals before construction`.

### Task 5: Controller-wide request protection and principal delivery

**Files:** Modify common `router.rs`, `route.rs`, `passport/{guard,strategy,context}.rs`, and common-macro endpoint adapters. Add `tests/controller_seals.rs`; migrate relevant unit tests for static policy bindings.

**Interfaces:**
- RouterBuildContext and Passport strategy bindings index by the scoped endpoint occurrence from Task 4, including controller context. Sharing a GuardPolicy descriptor does not share a cauldron strategy decision or erase endpoint identity.
- Every registrar for a sealed controller invokes the existing authentication/authorization pipeline with the selected policy before invoking its inherent method. Public empty-seal controllers install ordinary handlers.
- Preserve typed PassportGuard/principal extraction, verified-claims caching, token source handling, policy order (roles, permissions, predicates), status normalization, native response escape hatches, and server-side-only failure sources.

- [ ] Add requests covering GET-by-ID, base GET, and POST on one sealed controller: missing/invalid credentials return 401, failed policy returns 403, and handler counters remain 0; authorized requests call handlers and expose typed principals. Add an unsealed controller's public request and cookie-source cases.
- [ ] Add the same policy on controllers in two disjoint cauldrons, each with a same-named locally registered custom strategy. Assert each request invokes its own context binding and no global cache leaks across applications. Add direct-import and global-exported strategy success cases.
- [ ] Run `controller_seals`; record RED missing policy attachment/request binding before adapting handler dispatch.
- [ ] Adapt request protection to direct endpoint descriptors and static seal selection; keep one guard pipeline per controller and reject route-level policy attributes during macro parsing.
- [ ] Run `controller_seals`, native Passport, bearer/cookie, authorization/redaction, principal, and router/server tests with loopback permission where needed.
- [ ] Commit `feat(http): protect every controller endpoint with its declared seal`.

### Task 6: Migrate all consumers, scaffold/inspection, and remove route traits

**Files:** Modify all common/facade/testing/persistence tests and examples, facade compile consumers/UI fixtures, CLI fixtures/templates/output DTOs/rendering/inspection, and active examples. Delete macro `routes.rs`, old route-contract marker exports/runtime public types that require traits, CLI `templates/routes.rs.txt`, and obsolete trait-only UI fixtures; retain parser utilities owned by `endpoint.rs`.

**Interfaces:**
- Final public surface exposes direct `controller`, verbs, `Sealable`, `SealRegistration`, typed `guard`, and cauldron APIs, with no `routes` macro, `controller(routes = ...)`, trait/method guard inheritance, `skip`, or old brand/module aliases.
- Version-2 endpoint reports contain controller, handler, method, canonical path, and selected guard evidence; no fabricated `route_trait` field. Graph reports contain explicit cauldron ownership. Private child protocol uses renamed environment keys and version 2.
- Scaffold emits exactly `Cargo.toml`, `furnace.toml`, `src/main.rs`, and `src/app/{mod,controller,service}.rs`, with an AppCauldron, explicit service/controller registration, inherent base GET, and empty seals. No `routes.rs`/`mod routes`.
- Focused HTTP fixtures select their subject controller's inherent methods/seals/dependency chain; supplied fixture values remain isolated. Infrastructure and application examples use explicit cauldrons and correctly typed factory outputs.

- [ ] Add compile-fail cases proving old names/routes/guard targets/skip are rejected, and external pass cases for the complete spec example. Add scaffold exact-file/content assertions and offline generated consumer inspection/HTTP smoke tests. Assert schema/protocol 2, missing `route_trait`, correct `root_cauldron`, safe partial failures, and inspection constructor/bind counters 0. Reject protocol 1 before app work.
- [ ] Run these acceptance cases while staged legacy APIs still exist; record legacy compile-fail RED and obsolete scaffold/report output RED.
- [ ] Migrate each consumer preserving its intended behavior, replacing route-trait calls with inherent methods and explicit Sealable. Update all templates/CLI DTOs together, then remove legacy macro/runtime trait support and obsolete diagnostics. Inspect UI snapshot changes, including separate stable/MSRV compiler wording, before accepting snapshots.
- [ ] Run full common/facade/testing package tests, CLI scaffold/inspection/protocol/dev tests, all standalone examples and persistence standard startup checks; scan active code for old names/macros/files/implicit env prefixes. Classify historical and intentional failure matches separately.
- [ ] Commit `feat!: migrate consumers to cauldrons and sealed direct controllers`.

### Task 7: Active documentation, migration, and release tooling

**Files:** Root/crate READMEs, rustdoc/preludes, `CHANGELOG.md`, `CONTRIBUTING.md`, active `docs/{CLI,ARCHITECTURE}.md`, example READMEs, script release/archive docs/tests, and workflows. Create `docs/importance/furnace-rs-migration.md`.

**Interfaces:**
- Migration docs use default `furnace_rs` and recommended facade alias correctly, typed cauldron registration, direct controller methods, canonical paths, public/protected seals, explicit infrastructure exports, renamed config/env/CLI, and schema 2.
- CI/script package order lists all nine `furnace-rs` packages, new command/archive/config payloads, unchanged versions/features/MSRV, and `FURNACE_TEST_DATABASE_URL` for ignored live tests. No upload or publishing is executed locally.

- [ ] Add release/tooling assertions that all package/version pins, archive policy groups, PostgreSQL env, schema version, and generated file count match the final spec. Verify active guides contain the complete protected/public examples and no claim that Rust `pub` grants DI access.
- [ ] Run affected CLI release/documentation tests and rustdoc; record stale expectations and the previously deferred facade `pub` sentence as failures/mismatches to correct.
- [ ] Update active docs and tooling; add an unreleased breaking entry without choosing a release version. Keep historical specs/plans/reports intact and link the new migration guide where historical APIs are mentioned in active navigation.
- [ ] Run release-automation tests, strict rustdoc, and archive payload checks. New unpublished package identities may prevent registry-dependent `cargo package --no-verify`; distinguish that external registry resolution limitation from a payload failure, inspect `cargo package --list` and local archives when possible, and report exact unavailable validation rather than changing publish identities to make checks pass.
- [ ] Commit `docs!: document furnace-rs controllers, seals, and migration`.

### Task 8: Full verification, final review, and local handoff

**Files:** Only fixes required by demonstrated failures; update this plan's task status and create `docs/superpowers/reports/2026-10-01-furnace-rs-verification.md` with actual results, fresh-review findings/fixes, rulings, deferred minors, and external limitations.

**Interfaces:** Final branch has no staged compatibility paths, no uncommitted product edits, and the complete spec acceptance matrix. Preserve assigned branch and local task commits; do not push/merge/publish.

- [ ] Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --locked --workspace --all-features --no-fail-fast` and separate locked workspace doctests; use loopback permission for HTTP/CLI integrations.
- [ ] Run core-only facade, HTTP-only facade, JWT-only common, cookies, persistence without features and with sea-orm-postgres checks. Inspect normal dependency trees to confirm boundaries.
- [ ] Run strict rustdoc, every standalone example manifest, native persistence example, and archive checks. Record registry resolution limitations for unpublished renamed package dependencies without publishing them.
- [ ] Run installed Rust 1.94 full workspace tests; run the ignored real database tests if `FURNACE_TEST_DATABASE_URL` is available, otherwise record the existing CI PostgreSQL job as required external verification. Do not install a toolchain or provision a database merely to claim coverage.
- [ ] Produce a whole-branch review package from this task's implementation base. Dispatch exactly one fresh read-only final reviewer under executing-plans, with this spec/plan, Review Focus, verification evidence, and rulings ledger. Reproduce important findings with failing tests and fix them in one pass; rerun full affected/stable/MSRV gates. Record minor findings for the user rather than silently expanding scope.
- [ ] Commit demonstrated verification fixes and the final report/task-status update. Preserve local branch and clean working tree; remove only this plan's disposable execution artifacts after their record is committed.

## Self-review and handoff

Spec coverage: Task 1 owns all branding/cauldron/config/diagnostic mappings; Task 2 owns static guard/seal declarations; Task 3 owns direct method syntax/adapters/canonical paths; Task 4 owns selected preconstruction validation and core separation; Task 5 owns actual request protection/context isolation; Task 6 owns removal and every executable consumer; Task 7 owns active docs/release payload contracts; Task 8 owns full verification and one fresh whole-branch review.

Type consistency: all later tasks consume `SealDefinition`/`GuardPolicy` from Task 2, the two descriptor catalogs from Task 3, and the scoped endpoint/context selection from Task 4. Core callbacks carry only core types/TypeIds/diagnostics; common supplies HTTP interpretation. No task requires a controller instance to obtain endpoint or seal metadata.

The scope is one coordinated public API migration with sequential dependencies, not independent implementation work suitable for parallel edits. Continue with native execution after the user reviews this plan; retain per-task commits and a single final fresh reviewer as used for the prior implementation.
