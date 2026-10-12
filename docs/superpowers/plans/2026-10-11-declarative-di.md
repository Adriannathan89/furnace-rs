# Declarative DI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Use superpowers:subagent-driven-development only if the user chooses delegated execution. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add declarative cauldron registration, explicit injection markers, and root-scoped compile-time dependency checks, including the database/logger module contracts.

**Architecture:** Generate static metadata and the existing runtime registration from the same `#[cauldron(...)]` declaration. Preserve managed macros and runtime graph analysis. Prove global membership feasibility before migrating infrastructure modules.

**Tech Stack:** Rust 2024, MSRV 1.94, syn/quote procedural macros, typed dependency tuples, inventory, trybuild, existing Tokio/SeaORM integrations.

**Spec:** [Declarative DI design](../specs/2026-10-11-declarative-di-design.md).

## Global Constraints

- Rust MSRV remains 1.94 and edition remains 2024.
- Production dependencies remain unchanged. `trybuild` may be added as a core dev dependency.
- Preserve bare `#[cauldron]`, manual `Cauldron`, builder APIs, native outputs, asynchronous construction, and lifecycle.
- Root verification adds no public proof argument to burn/builder APIs and no separate checked-cauldron attribute.
- Unmarked managed fields remain injected; sixteen-dependency tuple limit remains unchanged.
- Globals are visible only inside the marked root's reachable graph and only through exported outputs.
- Declarative imports support cauldron declarations, qualified paths, re-exports, and `use` aliases; module `type` aliases retain support only in legacy fluent registration.
- Zero-argument cauldron attributes retain legacy behavior; empty declarative modules use `providers = []`.
- Beta target is `2.0.0-beta.1`; do not publish, tag, merge, or push.

## Review Focus

- Diamond imports and local/global or direct/global overlap must not create ambiguous proof inference: Task 1.
- A linked global outside the selected graph, including another root's globals, must not satisfy dependencies: Tasks 1 and 3.
- Renamed crates and private qualified types must resolve without exposing application-private types through generated public metadata: Task 3.
- Native connection/logger overrides must preserve instance selection and lifecycle without deleting static requirements: Task 5.
- Runtime auto-configuration must not be assumed universally available by the checker: Tasks 3 and 4.

## File map

- Extend `crates/furnace-rs-core-macros/src/cauldron.rs`; add `cauldron/declaration.rs` for parsing and `cauldron/contract.rs` for metadata/proof expansion.
- Add `crates/furnace-rs-core/src/di_contract.rs`; export generated-code helpers through existing `__private` in `src/lib.rs`.
- Extend `crates/furnace-rs-core-macros/src/managed.rs` and `crates/furnace-rs-common-macros/src/controller/managed.rs` for injection helpers.
- Add core `tests/declarative_di.rs`, `tests/declarative_di_ui.rs`, and `tests/ui/declarative_di/{pass,fail}/` fixtures.
- Extend common/logger and persistence module source/tests, facade UI/path-resolution tests, and feature checks.
- Add standalone `example/compile-time-di/`; update infrastructure guides, public macro docs, and release metadata.

## Task 1: Prove root-scoped globals without changing product behavior

**Execution status:** complete after mechanism revision and the user's beta-import restriction approval. The replacement marker/callback protocol passed 30 expected-outcome checks on Rust 1.94/stable and two additional declaration-ID collision controls. The original inference failure is retained in the spec as evidence; product tasks now consume the revised signatures and protocol.

**Files:** Create a throwaway `/tmp/furnace-di-contract-probe/` Rust project and record conclusions in the spec's feasibility section. Do not install production dependencies.

**Interfaces:** Consumes the proposed `ModuleContract` associated metadata and `CheckGraph<Root, Proof>` contract. Produces a verified witness mechanism and exact helper trait definitions for Task 3.

- [x] Write isolated cases for local resolution, direct exports, a nested global, and `(Config,)` intrinsic inputs; missing types must fail using Rust type identity.
- [x] Run `cargo +1.94.0 check --manifest-path /tmp/furnace-di-contract-probe/Cargo.toml` for each valid graph; record exit 0. Run invalid variants separately; record a missing trait-bound failure.
- [x] Add a diamond reaching the same global, local/global overlap, direct/global overlap, two roots with different imports, and a linked but unreachable global. Valid overlap cases must compile; the missing-global root must fail. Required overlap outcomes failed; this is a gate failure, not a passing implementation.
- [x] Run the same matrix on installed stable Rust. Inspect errors for trait solver ambiguity or overflow on valid graphs.
- [x] Document the exact working metadata/witness signatures, inference entry point, duplicate handling, and recursion behavior in the spec before Task 3.
- [x] If any required valid case cannot be represented without changing public syntax or semantics, stop product implementation and report the concrete failed case. Revise the spec before proceeding; do not substitute runtime checking for the promised compile check.

This task is a feasibility gate, not a distributable feature. Do not copy a failing prototype into the engine.

## Task 2: Add declarative registration with runtime equivalence

**Files:** Core macro cauldron parser/expansion files from the file map; core `tests/declarative_di.rs`; facade UI fixtures for parser failures.

**Interfaces:** Consumes existing `CauldronRegistration::{provide,provide_with,controller,import,export,global}`. Produces generated `impl Cauldron` from `providers`, `controllers`, `imports`, `exports`, `global`, and `root` syntax; root proof emission is attached in Task 3.

- [x] Add tests comparing declarative and manual registration definitions for provider output/constructor pairs, import/export lists, controller membership, and global status. Add `#[cauldron(providers = [])]` empty-module coverage and bare/zero-argument attribute manual-impl legacy pass fixtures.
- [x] Run `cargo test -p furnace-rs-core --test declarative_di` and the targeted facade UI suite; verify the new syntax fails for the intended unsupported-arguments reason before implementation.
- [x] Parse declarations in `cauldron/declaration.rs`; retain explicit bare-attribute handling. Generate the same runtime registration and inventory descriptor in the declarative branch. Do not add a second descriptor or alter existing constructor metadata.
- [x] Add compile-fail fixtures for duplicate options/list entries, unknown options, malformed `Output => Injector`, generic/non-unit structs, and manual-impl conflicts. Check qualified paths and empty lists.
- [x] Run the new runtime and UI tests plus existing core cauldron registration/scope tests; assert equivalent runtime selection and preserved legacy acceptance.
- [x] Commit the tested registration extension with its regression tests. Preserve actual red/green command evidence; do not manufacture a failing historical commit.

## Task 3: Verify the complete declared root graph

**Files:** Core `src/di_contract.rs`, `src/lib.rs`, `Cargo.toml` if trybuild is needed; macro `cauldron/contract.rs`; core declarative UI harness and fixtures; facade `tests/path_resolution.rs` and its existing consumer fixtures.

**Interfaces:** Consumes the exact proven Task 1 witness API and Task 2 declaration model. Produces `__private::ModuleContract`, `__private::ModuleMetadata`, `__private::CheckModule<Scope, Proof>`, marker/tree/identity helpers established by the probe, and an unconditional generated assertion for `#[cauldron(root, ...)]`.

- [x] Write compile pass/fail fixtures for local injection, direct export access, hidden/non-exported dependencies, transitive non-global access, native `Output => Injector`, invalid exports, controller export rejection, and missing metadata on an imported legacy module.
- [x] Add global fixtures: nested global success, missing global import, unexported global member, unrelated linked global, independent roots, diamond, overlap cases, and rejection of a marked root imported by another marked root. Include an empty `main` case proving checks run even without burn/build calls.
- [x] Run the UI harness before implementing contracts; confirm failures arise from missing support, then establish snapshots only after implementing correct behavior.
- [x] Generate metadata, same-name macro callbacks, staged child-description bootstrap and the root collector, and per-module normalized balanced scope assertions using the proven mechanism. Include compiler identity guards before any declaration/slot deduplication. Validate actual `Injector::Dependencies` for every selected constructor/controller. Model `Config` as the existing intrinsic; do not treat arbitrary auto-configuration outputs as universal capabilities.
- [x] Add a fixture where a runtime native override is supplied but a declared constructor dependency is absent: the marked root must still fail. Add a fixture proving an arbitrary runtime auto-config output is not statically assumed.
- [x] Verify renamed core/facade dependencies, raw identifiers, private module types, duplicate authored paths, and qualified bindings. Adjust generated metadata visibility without requiring application types to become public.
- [x] Run `cargo test -p furnace-rs-core --test declarative_di_ui` on stable and Rust 1.94; follow the existing compiler-specific snapshot convention. Run facade path-resolution tests and core no-default-features checks.
- [x] Commit the compile-time contract and tests after the feasibility gate and all required pass cases succeed.

## Task 4: Accept explicit injection markers on managed types

**Files:** Core macro `src/managed.rs`; common macro `src/controller/managed.rs` and its tests; facade existing UI harness plus new injection fixtures; common HTTP preflight tests as needed.

**Interfaces:** Consumes parameterless field `#[inject]`; produces the same generated `Injector::Dependencies`, managed handle, descriptor, and lifecycle as the existing field injection model.

- [x] Add fixtures for marked burner/storage/controller dependencies, missing markers rejected by all three managed macros, repeated markers, and rejected `#[inject(token = ...)]` arguments.
- [x] Run the targeted facade UI suite and macro unit tests; confirm valid markers currently fail at field-attribute validation.
- [x] Accept and consume one parameterless marker per field. Keep every managed field in the constructor tuple; retain documentation attributes and existing diagnostics for other unsupported attributes. Preserve automatic controller dependencies and HTTP metadata.
- [x] Add runtime coverage checking marked handles, shared dependency values, and lifecycle behavior; retain HTTP preflight-before-construction tests. A controller dependency supplied only by runtime auto-configuration is not a static capability unless the root contract declares it.
- [x] Run core injector/lifecycle tests, facade UI tests with HTTP enabled, and common HTTP preflight tests. Verify the macro removes helpers before emitting field definitions.
- [x] Commit injection-marker support and compatibility tests.

## Task 5: Adopt the database and logger contracts across crates

**Files:** `crates/furnace-rs-persistence/src/sea_orm/mod.rs`, `tests/module.rs`, `tests/postgres.rs`; common `src/logger/mod.rs`, `tests/logger.rs`; facade `tests/logger.rs` and new `tests/declarative_infrastructure_ui.rs`/fixture directory.

**Interfaces:** Consumes declarative metadata, `global`, `Output => Injector`, intrinsic `Config`, and field markers. Produces static metadata for existing public `DatabaseCauldron` and `LoggerCauldron` paths with unchanged runtime registrations.

- [x] Add cross-crate compile fixtures with a burner consuming both `DatabaseConnection` and `Logger`: importing both global modules into the root must compile; removing either import must fail naming that dependency. Include feature gating so the database fixtures run only with the SeaORM feature enabled.
- [x] Verify these tests fail because the current infrastructure modules lack static metadata, rather than connecting to a database.
- [x] Replace each infrastructure module's manual `impl Cauldron` with the exact declarations in the spec. Keep injectors, inventory provider descriptors, connector configuration, native connection output, and lifecycle hooks intact.
- [x] Test that database factory/connector remain private, logger-only consumers work without persistence enabled, unrelated globals do not leak, and legacy roots can still import both migrated modules.
- [x] Extend runtime tests for supplied native database/logger values, default logger selection, connection readiness/shutdown, and one instance through diamond imports. Use existing SeaORM mocks for ordinary CI; real PostgreSQL tests remain conditional on the existing environment.
- [x] Run common/logger tests and persistence module tests with `sea-orm-postgres`, plus the cross-crate UI suite. Report real-database tests separately if their required service is unavailable.
- [x] Commit the infrastructure adoption and regression tests.

## Task 6: Deliver a tested example, migration guide, and beta metadata

**Files:** New `example/compile-time-di/{Cargo.toml,Cargo.lock,README.md,src/main.rs,tests/missing_dependency.rs,tests/ui/...}`; `example/README.md`; workspace/public crate READMEs; persistence example/guide; release metadata and existing version-policy tests.

**Interfaces:** Consumes the final macro grammar and infrastructure exports. Produces an independently runnable example and a reproducible compile-fail case invoked through `cargo test`.

- [x] Add a standalone example using burner, inject, local providers, global LoggerCauldron, and a marked root. Add trybuild fixtures where the logger import is absent. Document a database-root variant using DatabaseCauldron and its required feature/configuration without requiring PostgreSQL for the default run.
- [x] First run the compile-fail harness against a valid fixture to prove it rejects unexpected compilation; then remove the dependency registration/import and run it normally. A passing test means the invalid application failed compilation for the intended DI reason.
- [x] Run `cargo run --manifest-path example/compile-time-di/Cargo.toml` and `cargo test --manifest-path example/compile-time-di/Cargo.toml` on stable and Rust 1.94. Keep fixture failures out of ordinary example builds.
- [x] Update docs with `root`, global reachability, explicit markers, native bindings, compatibility, static override restrictions, and runtime failure boundaries. Test public examples and update the existing persistence example to show database/logger root imports where features permit.
- [x] Prepare `2.0.0-beta.1` workspace versions and exact internal pins, regenerate relevant lockfiles, and update existing release-policy tests only where they intentionally assert version alignment. Add beta release notes; do not publish.
- [x] Run `cargo fmt --all --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`.
- [x] Run the relevant MSRV UI/runtime suites with `cargo +1.94.0`, core `--no-default-features` build, existing facade feature matrix, renamed dependency tests, and existing standalone example checks. Distinguish environment failures from DI regressions.
- [x] Review the whole diff for graph parity, visibility changes, macro hygiene, proof ambiguity, beta pins, and unsupported guarantees. Use an independent reviewer only when explicitly authorized; fix confirmed findings and rerun affected checks.
- [x] Commit the example, docs, and beta preparation with verification evidence. Leave the branch ready for review without publishing or integration actions.

**Execution status:** Tasks 1–6 complete. See [beta verification and TDD history](../../releases/2.0.0-beta.1-verification.md) for final test evidence and review resolution. The branch remains local and separate; no publication or integration action was performed.

## Handoff and completion criteria

This plan describes implementation work; creating these documents does not execute it. The `root` syntax and the feasibility gate are explicit review items. All six tasks map to the spec's acceptance criteria; any change to graph semantics or application syntax must update both documents before proceeding.

Native execution is the default recommendation because tasks share one macro/type-contract implementation and should proceed sequentially. Delegated execution is optional if the user requests it. Completion requires recorded red/green evidence, passing relevant checks, reproducible negative examples, and a beta-ready branch; startup success under arbitrary external conditions is not a completion claim.

## 2026-10-12 approved 2.0 prerelease amendment

The user replaced the additive beta policy with a major release: managed burner, storage and controller dependency fields now require `#[inject]`. Unmarked fields are compile errors. Preserve unit structs, manual `Injector::Dependencies`, runtime construction, and fluent registrations. Synchronize workspace and local dependency pins to 2.0.0-beta.1, migrate examples/templates/fixtures, and review checker soundness before committing the release preparation. Historical benchmark reports retain their measured version labels; obsolete release notes are removed.
