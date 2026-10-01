# Explicit Furnace Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace namespace-inferred module registration with explicit furnace ownership, scoped exports, the agreed declaration vocabulary, and `Mads::burn`.

**Architecture:** Keep inventory as a declaration catalog and collect registration callbacks from one selected root into an explicit membership graph. Core owns registration, reachability, exports, and dependency access; HTTP consumes that graph for controller/Passport selection. Preserve existing constructors, lifecycle resources, configuration, inspection wire formats, and feature boundaries.

**Tech Stack:** Rust 2024, Rust 1.94 minimum, existing syn/quote proc macros, inventory, Tokio, Axum, trybuild, and workspace tooling.

**Spec:** [Furnace registration design](../specs/2026-10-01-furnace-registration-design.md)

**Status:** Draft execution plan, pending review of both documents. Registration through a `register` method is confirmed; generic chaining, export rules, and final removal of old names still require design review. Do not implement merely because this plan exists.

## Global Constraints

- Rust edition 2024; minimum supported Rust version 1.94.
- No new dependencies and no unsafe code.
- Core-only builds must not acquire HTTP, JWT, or database dependencies.
- Rooted ownership is explicit and unique within one selected application graph.
- Global furnaces expose only explicit exports and must be reachable from the selected root.
- Registration and analysis execute no provider constructors or lifecycle hooks.
- Existing standard configuration, lifecycle, shutdown, and inspection behavior is preserved under `burn`.

## Review Focus

- Named-field services and controllers must register without constructing an instance; Task 1 pins generic chaining.
- Multiple element functions returning the same normalized output must fail deterministically rather than silently choose a constructor; Task 2 pins factory ambiguity.
- Builder overrides and official auto-configuration contributions must not bypass furnace ownership; Tasks 3 and 5 pin those paths.
- Route traits, guards, and Passport strategies in different Rust namespaces must use the selected controller's explicit scope; Task 4 pins that path.
- Two independent application roots and diamond imports must not leak membership/global state through cached registration; Task 2 pins isolation and callback evaluation.

## File and interface map

- Create `crates/mads-core/src/furnace.rs`: public `Furnace`/`FurnaceRegistration`, doc-hidden erased metadata, and chain recording.
- Create `crates/mads-core-macros/src/furnace.rs`: unit-struct expansion and registration callback generation. `module_v2.rs` is removed after consumers migrate; `module.rs` is already inactive commented code and can be removed at the same time.
- Modify `crates/mads-core/src/descriptor.rs`, `catalog.rs`, `graph/module.rs`, `graph/scope.rs`, `builder.rs`: registration metadata, collection, ownership/access, and rooted selection.
- Modify macro `lib.rs`, `managed.rs`, `provider.rs` and their support tests: new attributes and current constructor semantics.
- Modify `crates/mads-common/src/http_scope.rs`, `router.rs`, `route.rs`, `passport/guard.rs`, `inspection.rs`, and `server.rs` where they consume scope: explicit controller and strategy access; `burn`.
- Modify `crates/mads-common/src/logger/mod.rs`, `jwt/auto_configuration.rs`, `crates/mads-persistence/src/sea_orm/mod.rs`, and core auto-configuration code: official contributions and explicit infrastructure furnaces.
- Modify core/common/facade `lib.rs`: exports/preludes. Update testing fixtures, consumer/UI fixtures, CLI templates, examples, and documentation with their owning task.

Use existing files unless a listed new file owns a distinct responsibility. Do not split unrelated subsystems or rename all graph/report types.

## Execution sequencing

Execute tasks in order in an isolated branch/worktree, respecting existing workspace instructions. Temporary old-name exports may remain while migrating internal consumers; remove them in Task 8. They are staging aids, not shipped compatibility promises. Each task includes its own test cycle and ends with a focused commit containing only that task's files. Any fixture broken by an interface change must be migrated with its owning task rather than knowingly leave that task untestable.

### Task 1: Declaration vocabulary and typed registration contract

**Files:** Create `crates/mads-core/src/furnace.rs`, `crates/mads-core-macros/src/furnace.rs`, `crates/mads-core/tests/furnace_registration.rs`, `crates/mads-core-macros/tests/support/furnace.rs`; modify core/macro/facade `src/lib.rs`, core `descriptor.rs`, macro `managed.rs` and `provider.rs`; add furnace UI fixtures under `crates/mads/tests/ui/pass/` and `ui/fail/`.

**Interfaces:**
- Produces `Furnace::register(self) -> FurnaceRegistration<Self>` and all generic chain methods from the spec.
- Produces doc-hidden `FurnaceDefinition` with ordered `Vec<FurnaceMember>`, ordered `Vec<FurnaceImport>`, ordered export TypeIds/locations, and a global flag. Members contain output TypeId, `type_name::<T>()`, role (`Provider` or `Controller`), and caller location; imports contain furnace TypeId/name and caller location. `FurnaceRegistration<M>::into_definition(self) -> FurnaceDefinition` erases the type.
- Produces `ModuleDescriptor::with_registration(self, callback: fn() -> FurnaceDefinition) -> Self` and `registration(&self) -> Option<fn() -> FurnaceDefinition>` for staged migration.
- Furnace macro callback is `|| <DeclaredType as Furnace>::register(DeclaredType).into_definition()`. Callback imports are recorded, never recursively invoked by `.import`.
- Produces `furnace`, `burner`, `storage`, `element`, and `element(lifecycle)` macro entry points. Existing provider kind variants remain.

- [ ] Add `named_field_types_register_without_construction`: a furnace chain registers a dependency-bearing burner and controller by type; erasing the chain yields their exact TypeIds/roles, import order, export list, and global flag. Constructor counters stay at zero.
- [ ] Add UI pass fixtures for `FurnaceRegistration::new(self)`, a named-field burner/storage/controller, renamed facade/core dependencies, and async `element(lifecycle)` syntax. Add UI failures for furnace arguments, named fields, generics, and missing `impl Furnace`; expected diagnostics use `furnace`.
- [ ] Run `cargo test -p mads-core --test furnace_registration` and `cargo test -p mads --test ui --test path_resolution`; verify failure is missing new APIs, not an unrelated fixture/environment failure.
- [ ] Implement the interfaces above. Use `#[track_caller]` on chain recording and generated forwarding methods so errors identify registration sites. Reuse existing managed/provider expansions and crate-path resolution; do not change factory callable signatures or outputs.
- [ ] Run `cargo test -p mads-core --test furnace_registration`, `cargo test -p mads-core-macros`, and `cargo test -p mads --test ui --test path_resolution`; inspect newly generated trybuild stderr rather than blindly accepting snapshots.
- [ ] Commit: `feat(core): add furnace declarations and typed registration`.

### Task 2: Explicit graph collection and unique ownership

**Files:** Modify core `src/catalog.rs`, `src/descriptor.rs`, `src/graph/module.rs`, `src/graph/model.rs`, `src/graph/inspection.rs`; create `crates/mads-core/tests/furnace_graph.rs`; migrate existing `tests/module_graph.rs`, `module_scope.rs`, `descriptor.rs`, and graph fixtures as needed.

**Interfaces:**
- Consumes Task 1 registration callbacks and erased definition records.
- Existing `build_module_graph(root: TypeId, descriptors: &[&'static ModuleDescriptor]) -> Result<ModuleGraph>` collects callbacks and owns their runtime records.
- Produces `ModuleGraph::owner_of(TypeId) -> Option<&ModuleNode>`, `exports(TypeId, TypeId) -> bool`, `is_global(TypeId) -> bool`, `is_controller(TypeId) -> bool`, and `can_access(TypeId, TypeId) -> bool` with spec semantics. Retains existing root/import/node/report APIs.
- Provider descriptor selection continues to use output TypeId; only selected outputs undergo missing/ambiguous constructor lookup.

- [ ] Add `same_namespace_furnaces_have_distinct_owners` and assert two distinct furnace TypeIds can share a namespace while each output has its authored owner.
- [ ] Add `duplicate_member_and_invalid_export_are_rejected`: duplicate provider/controller membership, duplicate exports, exports of nonlocal members, and controller exports yield `MADS008` with registration locations. Define duplicate `.global()` as idempotent, since it is a boolean flag.
- [ ] Add `imports_validate_cycles_and_diamonds`: duplicate direct imports and cycles fail with `MADS008`; a diamond contributes one shared furnace, preserves authored direct edges, and evaluates its callback once per pass.
- [ ] Add `independent_roots_do_not_share_registration_state`: analyze two roots sharing declarations in separate graphs; owners/global flags and errors remain root-specific. No process-global registration result cache is allowed.
- [ ] Add `registered_output_requires_unique_constructor`: missing normalized output reports `MADS003`, duplicate selected output descriptors report existing duplicate/ambiguous codes; unregistered unrelated output descriptors do not invalidate the rooted graph.
- [ ] Run `cargo test -p mads-core --test furnace_graph`; verify the new behavior tests fail against namespace ownership before changing graph code.
- [ ] Collect definitions during rooted traversal, build a TypeId membership map, validate local membership/export/import rules, and remove namespace collision/longest-prefix ownership logic. Resolve constructor metadata after membership collection; include role validation against provider/controller catalogs in the owning layer. Retain namespace fields as report metadata only.
- [ ] Run `cargo test -p mads-core --test furnace_graph --test module_graph --test descriptor --test inspection_snapshot --test catalog` and confirm deterministic diagnostics/ownership.
- [ ] Commit: `feat(core): collect explicit furnace membership graphs`.

### Task 3: Scoped dependencies and rooted builder overrides

**Files:** Modify core `src/graph/scope.rs`, `src/builder.rs`, `src/graph/analysis.rs`, `src/auto_configuration/analysis.rs` as required; create `crates/mads-core/tests/furnace_scope.rs`; migrate existing scoped/architecture fixtures.

**Interfaces:**
- Consumes Task 2 authoritative membership/accessors; rooted builders use `root<M: Furnace>(&mut self) -> Result<&mut Self>`.
- Scope selection returns existing selected descriptors, ownership, and diagnostics, using membership rather than namespace discovery.
- `MadsBuilder::provide<T>(value)` remains a value override. Rooted analysis rejects outputs not in the graph and retains registered ownership/access for valid overrides.

- [ ] Add a dependency access matrix: local private provider succeeds; public unexported imported provider fails `MADS009`; direct exported provider succeeds; transitive export alone fails; reachable global export succeeds; a reachable global private member fails; an unreachable global contributes nothing. Assert zero constructor calls on every invalid case.
- [ ] Add `dependency_is_not_implicitly_registered`: a linked constructor with no reachable `.provide::<T>()` fails as missing registration even when its namespace matches the requesting furnace.
- [ ] Add `namespace_move_preserves_membership`: equivalent declarations in different namespaces have the same selection/access behavior.
- [ ] Add `rooted_override_retains_scope`: overriding a registered exported output succeeds without its constructor; overriding a registered private output does not grant another furnace access; an unregistered override fails before startup.
- [ ] Run `cargo test -p mads-core --test furnace_scope`; confirm failures demonstrate namespace admission or visibility bypass.
- [ ] Replace scope admission with explicit graph membership, apply export checks before satisfying dependencies through supplied values, and retain the framework `Config` exception. Preserve dependency paths and structured diagnostics. Keep unrooted complete-catalog and focused test modes separate from rooted membership enforcement.
- [ ] Run `cargo test -p mads-core --test furnace_scope --test module_scope --test builder --test focused_selection --test invalid_graph --test lifecycle --test provider_lifecycle --test construction_failure`.
- [ ] Commit: `feat(core): enforce furnace exports and scoped overrides`.

### Task 4: Explicit controller routing and Passport scope

**Files:** Modify common `src/http_scope.rs`, `src/router.rs`, `src/route.rs`, `src/passport/guard.rs`, `src/inspection.rs` where scope is consumed; create `crates/mads-common/tests/furnace_http_scope.rs`; migrate `tests/scoped_router.rs`, `scoped_passport.rs`, `focused_router.rs`, and facade route fixtures.

**Interfaces:**
- Consumes `ModuleGraph::owner_of`, `is_controller`, and `can_access`.
- Retains `HttpApplicationScope` entry points and route/controller/guard descriptor APIs. Rooted controllers are chosen by registered TypeId, not descriptor namespace.
- A `.controller::<T>()` entry requires corresponding controller metadata; `.provide::<Controller>()` does not implicitly install routes.

- [ ] Add `only_registered_controllers_install_routes`: register one of two same-namespace controllers; inspect/build the router and assert only the selected route exists. A missing controller descriptor reports a registration error before listening.
- [ ] Add `controller_routes_ignore_rust_namespace_ownership`: controller, route trait, and guard in separate namespaces retain the controller's explicit furnace context.
- [ ] Add `passport_strategy_uses_explicit_exports`: local strategy works, a direct imported/exported strategy works, a reachable global/exported strategy works, and an unregistered or private foreign strategy fails even if namespaces previously admitted it.
- [ ] Add `provider_exports_do_not_copy_routes`: importing/exporting ordinary providers changes DI access without duplicating controller routes. Shared imported furnaces contribute each registered controller once.
- [ ] Run `cargo test -p mads-common --all-features --test furnace_http_scope`; confirm behavior failures before updating selectors.
- [ ] Remove namespace owner inference from rooted HTTP/Passport paths. Resolve controller owners from graph membership and strategy access from the requesting controller context; retain attached route/guard metadata and existing route conflict validation. Preserve focused fixture and unrooted modes.
- [ ] Run `cargo test -p mads-common --all-features --test furnace_http_scope --test scoped_router --test scoped_passport --test focused_router --test passport_preflight --test inspection` and `cargo test -p mads --all-features --test typed_dispatch --test controller_conflicts --test validated_routes`.
- [ ] Commit: `feat(http): select controllers through furnace registration`.

### Task 5: Explicit official infrastructure furnaces

**Files:** Modify common `src/logger/mod.rs`, `src/jwt/auto_configuration.rs`, core `src/auto_configuration/analysis.rs` and `descriptor.rs`, persistence `src/sea_orm/mod.rs`; migrate logger/JWT/persistence test declarations and add cases to common `tests/scoped_auto_configuration.rs` and persistence `tests/module.rs`.

**Interfaces:**
- `LoggerModule` implements `Furnace`, registers `Logger`, exports `Logger`, and calls `.global()`.
- `DatabaseModule` implements `Furnace`, registers `DatabaseFactory`, `SeaOrmPostgres`, and `DatabaseConnection`, exports only `DatabaseConnection`, and calls `.global()`; factory functions use `element`/`element(lifecycle)`.
- Official auto-configuration evaluator/applier signatures stay unchanged. Scope integration consumes explicit graphs, grants only official contributed outputs, and preserves application-controlled override precedence.

- [ ] Add `logger_global_exports_only_when_imported`: a reachable logger furnace satisfies foreign logger consumers; an unimported logger furnace does not.
- [ ] Add `database_furnace_keeps_factory_and_connector_private`: a foreign consumer can depend on exported `DatabaseConnection` but not the private factory/connector; invalid access fails before connector side effects.
- [ ] Add `official_defaults_do_not_admit_unregistered_factories`: a registered JWT consumer may receive its official default, while unrelated unregistered user factories cannot satisfy dependencies. A valid registered application override wins and reports redacted override evidence.
- [ ] Run the affected logger, scoped auto-configuration, JWT, and persistence module tests; confirm new cases fail on the old admission rules.
- [ ] Implement explicit registration/export chains and adapt official contribution ownership/access without changing integration identifiers, feature gates, configuration keys, or native output types.
- [ ] Run `cargo test -p mads-common --all-features --test logger --test scoped_auto_configuration --test jwt_auto_configuration --test jwt_auto_configuration_override` and `cargo test -p mads-persistence --features sea-orm-postgres --test module --test factory`.
- [ ] Commit: `feat(integrations): register infrastructure through furnaces`.

### Task 6: Burn startup, facade exports, and inspection

**Files:** Modify common `src/server.rs`, `src/inspection.rs`, `src/lib.rs`, facade `src/lib.rs`, core `src/lib.rs`; migrate standard-startup unit tests, facade `tests/consumers/automatic_run/src/main.rs`, inspection CLI fixtures, and facade `tests/facade.rs`/`path_resolution.rs`.

**Interfaces:**
- Produces `MadsBurnExt::burn<M: Furnace>() -> impl Future<Output = Result<(), HttpRuntimeError>> + Send`, implemented for `Mads`, replacing the standard run contract.
- Core/facade preludes export `Furnace`, `FurnaceRegistration`, and the new declaration macros; HTTP-enabled common/facade preludes export `MadsBurnExt`.
- Retains private inspection protocol version 1 and current configuration/listener/shutdown semantics.

- [ ] Add startup cases asserting `burn` reaches registration, conventional configuration precedence/default address, lifecycle preparation, and no-route failure before binding. Use existing injectable bind/shutdown unit-test seams rather than launch a blocking server.
- [ ] Migrate the inspection standard fixture to `burn`; assert route/graph/doctor inspection handshakes succeed without provider construction or listener startup and derive explicit ownership.
- [ ] Add renamed-dependency consumer compilation for `Furnace`, new attributes, `MadsBurnExt`, and `Mads::burn`; core-only consumer must use a builder without acquiring HTTP.
- [ ] Run affected startup/facade/inspection tests and confirm missing `burn`/exports fail.
- [ ] Implement the renamed extension by retaining the current standard preparation and inspection branch. Keep build/start/shutdown APIs intact.
- [ ] Run `cargo test -p mads-common --all-features --lib server`, `cargo test -p mads --all-features --test facade --test path_resolution`, and `cargo test -p mads-cli --test inspection_cli --test inspection_protocol`.
- [ ] Commit: `feat(runtime): start registered applications with burn`.

### Task 7: Generated apps, examples, and test fixtures

**Files:** Modify CLI `src/scaffold/templates/main.rs.txt`, `app_mod.rs.txt`, `service.rs.txt`, template tests and scaffold consumer fixtures; migrate `example/hello-world`, `example/posts-crud`, `example/protected-route`, persistence `examples/standard_run.rs`, facade UI/consumer tests, common/core integration fixtures, and testing `tests/http_fixture.rs`, `subject_fixture.rs`, `src/fixture.rs` where bounds change.

**Interfaces:**
- Generated app furnace registers its service and controller explicitly; generated main calls `Mads::burn::<AppModule>()`.
- Posts example furnace registers service/repository/controller and imports the database furnace where access is needed; root imports feature furnaces and logger.
- Protected-route example explicitly registers concrete services, trait-object factory output wrappers, controllers, and Passport strategies in their intended furnace contexts.
- Focused `mads-testing` APIs keep their function-local isolation and supplied-value behavior.

- [ ] Update scaffold acceptance assertions to require `#[furnace]`, `impl Furnace`, generic explicit registrations, `#[burner]`, and `Mads::burn`. Compile the generated consumer against workspace crates.
- [ ] Add an acceptance test using two imported feature furnaces plus one shared exported provider; assert HTTP routes are selected and private foreign storage access fails.
- [ ] Run affected scaffold/acceptance tests to expose missing registrations or stale generated APIs.
- [ ] Migrate the listed consumers and examples. Use normalized factory output types and explicit exports; do not mechanically expose every previously public provider. Update focused fixture tests only for renamed attributes/contracts, retaining their original isolation semantics.
- [ ] Run `cargo test -p mads-cli --test scaffold_consumer --test scaffold_http --test scaffold_cli`, `cargo test -p mads --all-features --test ui --test provider --test v090_acceptance`, and `cargo test -p mads-testing --all-features`.
- [ ] Check each example manifest listed by `rg --files example -g Cargo.toml` using `cargo check --manifest-path <listed manifest>`, and `cargo check -p mads-persistence --features sea-orm-postgres --example standard_run`.
- [ ] Commit: `feat(cli): scaffold apps with explicit furnace registration`.

### Task 8: Complete the breaking migration and document it

**Files:** Remove old macro entry points and exports in core/macro/common/facade `src/lib.rs`, remove inactive/obsolete module expansion files; modify root/crate README files, `CHANGELOG.md`, examples' README files, rustdoc, `CONTRIBUTING.md` where relevant; create `docs/importance/furnace-registration-migration.md`. Update remaining fixtures found by the stale-API scan.

**Interfaces:** Final public API has no `module`, `service`, `repository`, or `provider` attribute aliases, no `Module` marker, no `MadsRunExt`, and no `Mads::run`. Graph/report types and built-in service/module type names retained by the spec remain.

- [ ] Add compile-fail consumers proving the old attributes and `run` no longer resolve; add compile-pass consumers for their replacements. Replace obsolete UI cases with corresponding new-API shape/argument tests.
- [ ] Run the new consumers/UI tests and confirm obsolete aliases are currently still accepted before removal.
- [ ] Remove transitional aliases/old marker/startup names and finish fixture migration. Rename user-facing macro diagnostics to the new attributes while retaining semantic inspection kind values.
- [ ] Write a before/after migration example covering named-field services, normalized factory outputs, direct exports, global exports, imported shared ownership, and `burn`. Explain that Rust `pub` no longer exports DI bindings. Add an unreleased breaking-change entry without choosing a release version.
- [ ] Scan `crates`, `example`, and `tests` for `#\[.*(module|service|repository|provider)`, `Mads::run`, `MadsRunExt`, and old `Module` bounds; classify remaining matches. Historical specs/plans and intentional compile-fail fixtures remain valid; shipped runnable code must use the new API.
- [ ] Run `cargo test -p mads --all-features --test ui --test path_resolution --test facade` and `cargo test --locked --workspace --all-features --doc`.
- [ ] Commit: `feat!: remove legacy registration and startup APIs`.

### Task 9: Workspace verification and release readiness

**Files:** Modify only files needed to resolve a demonstrated verification failure. No version bump, publishing, push, or merge is part of this task.

**Interfaces:** Final validation exercises the spec's complete public contract and workspace feature matrix.

- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --locked --workspace --all-features` and `cargo test --locked --workspace --all-features --doc`.
- [ ] Run `cargo check -p mads --no-default-features`, `cargo check -p mads-common --no-default-features --features jwt --locked`, `cargo check -p mads-persistence --no-default-features`, and `cargo check -p mads-persistence --no-default-features --features sea-orm-postgres`.
- [ ] Run `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps` and `cargo package --locked --workspace --no-verify`.
- [ ] Run `cargo +1.94.0 test --locked --workspace --all-features` if that toolchain is installed; otherwise record that MSRV verification requires the existing CI job. Do not install a toolchain without checking environment permissions.
- [ ] Run real database checks under the existing CI PostgreSQL service: `cargo test -p mads-persistence --features sea-orm-postgres --test postgres`. Locally, document missing `MADS_TEST_DATABASE_URL` rather than claim database coverage from a skipped test.
- [ ] Compare graph/route/doctor results against the spec: explicit ownership, hidden private members, unchanged redaction/wire fields, deterministic errors, and selected-only controller routes.
- [ ] If fixes were required, commit only those fixes and rerun affected checks. Record actual passing commands and any unavailable external verification; request review before integration.

## Self-review and execution handoff

Spec coverage: Tasks 1–3 own declarations, membership, exports, globals, factories, and overrides; Task 4 owns controllers/Passport; Task 5 owns official integrations; Task 6 owns startup/inspection; Tasks 7–8 own all public migrations; Task 9 owns full compatibility checks. Review Focus cases are assigned explicit tests above.

Review both documents before execution. The tasks share one graph contract and should run sequentially. Native execution is a reasonable default for implementation; subagent-driven execution is available if the user chooses it. This documentation task does not select an execution method or begin implementation.
