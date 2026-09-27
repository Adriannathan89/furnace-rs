# MADS Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a module-free `mads-testing` fixture for focused provider construction, SQLite-backed SeaORM mocks, and in-process controller assertions with reliable lifecycle shutdown.

**Architecture:** Add a focused provider selection path to core and a selected-controller route path to common HTTP. The testing crate owns the public fixture, mock validation, request/assertion API, and scoped lifecycle runner. `#[mads::test]` generates the supported fixture entry point in an inline test module.

**Tech Stack:** Rust 2024, Rust 1.94, MADS core/common/facade, SeaORM 2.0 mock, Axum 0.8, Tower 0.5, Tokio, serde_json, futures-util.

**Spec:** `docs/superpowers/specs/2026-09-27-mads-testing-design.md`

## Global Constraints

- `mads-testing` is added at `crates/mads-testing`, currently an untracked empty user directory; inspect it before edits and do not discard user files.
- SeaORM 2.0 `MockDatabase` with `DbBackend::Sqlite` is the only database fixture. No SQLite driver, live database, socket, or file is used.
- A selected `DatabaseConnection` dependency requires an explicit `.mock_database(...)`; production database providers must not run as fallback. Direct `.provide::<DatabaseConnection>(...)` fails setup.
- Focused tests may cross MADS module import and private-provider boundaries. Rust language privacy remains enforced. Normal rooted and complete-catalog behavior is unchanged.
- `#[mads::test]` accepts inline modules, applies `#[cfg(test)]`, and generates module-local `test_fixture()`. File-backed modules are rejected.
- `run` starts hooks and awaits shutdown after normal completion or an unwinding assertion panic. Startup uses existing rollback. Cancellation and process abort are outside the cleanup guarantee.
- Public APIs have documentation; workspace `missing_docs = deny` and `unsafe_code = forbid` remain in force. Preserve crate dependency layering and avoid cycles.

## Review Focus

These five cases deserve explicit tests in their owning tasks:

1. Two supplied values of the same concrete type: fixture setup reports a duplicate and constructs no subject (Task 3).
2. A selected type has ambiguous provider descriptors: focused analysis retains `MADS002` rather than choosing one (Task 1).
3. An unrelated controller has malformed route metadata: the selected controller still serves (Task 2).
4. The body panics and a lifecycle stop hook fails: the original panic stays primary and shutdown failure is reported (Task 3).
5. A response body is malformed JSON: `assert_json` fails with the body and parse cause, rather than a vague mismatch (Task 4).

---

## File map

| File | Responsibility |
| --- | --- |
| `crates/mads-core/src/graph/focus.rs` | Traverse one provider's dependency closure and apply required-supply barriers. |
| `crates/mads-core/src/graph/mod.rs`, `src/builder.rs` | Connect focused analysis to existing graph validation and construction. |
| `crates/mads-common/src/http_scope.rs`, `src/route.rs`, `src/router.rs`, `src/lib.rs` | Select, validate, and register only one controller through the existing typed path. |
| `crates/mads-testing/src/error.rs` | Fixture errors and source preservation. |
| `crates/mads-testing/src/fixture.rs` | Subject selection, supplied values, mock check, and lifecycle runner. |
| `crates/mads-testing/src/http.rs` | In-process request construction and dispatch. |
| `crates/mads-testing/src/response.rs` | Buffered response and chainable assertions. |
| `crates/mads-testing/src/lib.rs` | Documented public exports and hidden macro entry. |
| `crates/mads-core-macros/src/test_module.rs` | Inline test-module attribute expansion. |
| `crates/mads-core-macros/src/lib.rs`, `src/path.rs`; `crates/mads-core/src/lib.rs`; `crates/mads/src/lib.rs` | Export macro and resolve a renamed `mads-testing` dependency. |
| `Cargo.toml`, `crates/mads-testing/Cargo.toml`, `Cargo.lock` | Workspace membership and dependencies. |
| `crates/mads-testing/README.md`, `README.md` | Public controller and service usage. |

### Task 1: Focused provider graph and construction

**Files:** Create `crates/mads-core/src/graph/focus.rs`, `crates/mads-core/tests/focused_selection.rs`; modify `crates/mads-core/src/graph/mod.rs`, `crates/mads-core/src/builder.rs`.

**Interfaces:** Produce `MadsBuilder::__test_focus<T: Send + Sync + 'static>(&mut self) -> mads_core::Result<&mut Self>` and `MadsBuilder::__test_require_provided<T: Send + Sync + 'static>(&mut self) -> &mut Self`, both document-hidden. Focus and `root` are mutually exclusive (`MADS008`). Focused analysis selects exactly `T` and its transitive registered dependencies, stops at provided values, and excludes required-supply types until supplied. Existing `analyze_descriptors`, construction ordering, diagnostics, and selected auto-configuration semantics are reused; unrelated auto-configurations do not run.

- [ ] **Step 1: Write failing integration tests** in `focused_selection.rs`: `focus_builds_private_transitive_chain_without_module` asserts selected service → repository and an unrelated panic constructor is never called; `focus_uses_supplied_value` asserts its registered constructor is skipped; `focus_requires_external_type` asserts missing required supply is `MADS003` and its constructor is not called; `focus_reports_ambiguous_selected_type` asserts `MADS002`; `focus_reports_missing_selected_type` asserts `MADS003`; `focus_reports_selected_cycle` asserts `MADS005`; `focus_rejects_root_mix` asserts `MADS008`; `focus_ignores_unrelated_auto_configuration` asserts its evaluator is not called.
- [ ] **Step 2: Run** `cargo test -p mads-core --test focused_selection`; expect failure because `__test_focus` is absent.
- [ ] **Step 3: Implement** the two methods and focused traversal. Keep the normal `root == None` complete-catalog path intact. Validate the selected descriptor even when its type is supplied, then let the supplied value stop construction of that descriptor. Ensure a required-supply type cannot be satisfied by a static provider or auto-configuration.
- [ ] **Step 4: Run** `cargo test -p mads-core --test focused_selection` and `cargo test -p mads-core --test auto_configuration_builder --test module_scope`; expect all to pass.
- [ ] **Step 5: Commit** the focused core change as `feat(testing): select one provider dependency chain`.

### Task 2: Selected-controller router

**Files:** Create `crates/mads-common/tests/focused_router.rs`; modify `crates/mads-common/src/http_scope.rs`, `src/route.rs`, `src/router.rs`, `src/lib.rs`.

**Interfaces:** Consume `MadsBuilder::__test_focus<T>()`. Produce document-hidden `mads_common::__private::build_test_router_for<T: Send + Sync + 'static>(&mads_core::Mads) -> mads_core::Result<axum::Router>`. Look up exactly one `ControllerRouteDescriptor` by `TypeId`; absent or ambiguous controller metadata is an error. Reuse selected route validation, typed registrar, and selected Passport guard preflight when enabled. Only selected routes are installed. The normal `build_router` path stays unchanged.

- [ ] **Step 1: Write failing tests** in `focused_router.rs`: `builds_only_selected_controller_routes` asserts selected `GET` returns 200 and unrelated path returns 404; `ignores_invalid_unselected_controller` asserts selected router builds despite malformed unrelated route; `rejects_invalid_selected_route` asserts `MADS030`; `rejects_absent_controller_metadata` asserts an error; `rejects_ambiguous_selected_controller_metadata` asserts an error. Add a guarded selected-route case under `cfg(feature = "jwt")` using existing Passport fixtures or a minimal local fixture.
- [ ] **Step 2: Run** `cargo test -p mads-common --features http --test focused_router`; expect failure because `build_test_router_for` is absent.
- [ ] **Step 3: Implement** descriptor lookup and selected scope/validation in the mapped files. Use the existing registrar and `RouterBuildContext`; avoid a second route execution path. Under JWT, preflight only guards attached to selected routes.
- [ ] **Step 4: Run** `cargo test -p mads-common --features http --test focused_router` and `cargo test -p mads-common --all-features --test focused_router`; expect all selected-route cases to pass.
- [ ] **Step 5: Commit** as `feat(testing): build a focused controller router`.

### Task 3: Subject fixture, mock database, and lifecycle runner

**Files:** Fill `crates/mads-testing/Cargo.toml`; create `src/lib.rs`, `src/error.rs`, `src/fixture.rs`, `tests/subject_fixture.rs`, `README.md`; modify workspace `Cargo.toml` and `Cargo.lock`.

**Interfaces:** Produce `TestFixtureBuilder`, `SubjectFixture<T>`, `TestContext`, `TestError`, `TestResult<T>`, and `mads_testing::sea_orm` re-export. `TestFixtureBuilder::mock_database(MockDatabase) -> Self`, `provide<T>(T) -> Self`, and `subject<T>() -> SubjectFixture<T>` follow the spec. `SubjectFixture<T>::run<F, Fut>(self, F) -> TestResult<()>` receives `FnOnce(TestContext) -> Fut` with `Fut: Future<Output = ()>`. `TestContext::resolve<T>() -> mads_core::Result<Arc<T>>`. `TestError` identifies `Mads`, `MissingMockDatabase`, `UnsupportedMockBackend`, `DuplicateSupply`, `DirectDatabaseSupply`, `Request`, `Serialization`, `Service`, and `ResponseBody` cases; `TestResult<T> = Result<T, TestError>`. A document-hidden constructor exists only for macro expansion. Use `futures-util::FutureExt::catch_unwind` with `AssertUnwindSafe` around the body, await shutdown, then resume the body panic. Return shutdown errors on normal completion.

- [ ] **Step 1: Add the crate manifest, workspace membership, and an empty `src/lib.rs`, then write failing tests** in `subject_fixture.rs`: declare `mads-core`, `mads-common` with `http`, SeaORM 2.0 with `mock` and Tokio runtime, `serde`, `serde_json`, `tower`, `futures-util`, and dev dependencies `tokio` and `mads`; add workspace `futures-util` if absent. Tests cover registered service → repository resolving with a SQLite mock and executing a queued SeaORM query through native `DatabaseConnection`; a private cross-module provider; `missing_mock_never_connects` returning `MissingMockDatabase` before any production connector; `postgres_mock_is_rejected`; `direct_database_supply_is_rejected`; `duplicate_supply_is_rejected_before_construction`; `start_and_stop_run_once`; `panic_still_stops`; `panic_and_stop_failure_preserves_panic` in a subprocess that inspects panic and shutdown diagnostic output; and `startup_failure_rolls_back`.
- [ ] **Step 2: Run** `cargo test -p mads-testing --test subject_fixture`; expect failure because the crate and fixture API are not yet defined.
- [ ] **Step 3: Implement** the fixture and errors. Keep `mads-testing` independent of `mads` and `mads-persistence` as normal dependencies to avoid a cycle. Check the mock backend via SeaORM's `MockDatabaseTrait::get_database_backend` before `into_connection`. Call core's required-supply barrier for `DatabaseConnection`. Preserve underlying MADS errors as `TestError` sources. Keep the fixture constructor document-hidden.
- [ ] **Step 4: Run** `cargo test -p mads-testing --test subject_fixture` and `cargo check -p mads-testing`; expect both to pass. Add a missing-doc lint check for the new public API.
- [ ] **Step 5: Commit** as `feat(testing): add focused subject fixture and lifecycle`.

### Task 4: In-process HTTP client and assertions

**Files:** Create `crates/mads-testing/src/http.rs`, `src/response.rs`, `tests/http_fixture.rs`; modify `src/fixture.rs`, `src/lib.rs`, `src/error.rs`.

**Interfaces:** Consume `mads_common::__private::build_test_router_for<T>()`. Produce `ControllerFixture<T>::run<F, Fut>(self, F) -> TestResult<()>` with `FnOnce(TestClient) -> Fut`, `Fut: Future<Output = ()>`. `TestClient` provides `resolve<T>()`, `request(Method, &str)`, and `get/post/put/patch/delete`. `TestRequest` provides `header(HeaderName, HeaderValue)`, `json<S: Serialize>(&S) -> TestResult<Self>`, and async `send() -> TestResult<TestResponse>`. `TestResponse` buffers status, headers, and body; `assert_status(StatusCode)`, `assert_json(Value)`, `assert_text(&str)`, and `assert_header(HeaderName, HeaderValue)` each return `Self` and panic with expected/actual values on mismatch.

- [ ] **Step 1: Write failing tests** in `http_fixture.rs`: selected controller serves its chain through GET and POST; unrelated route returns 404; PUT/PATCH/DELETE and custom `request` dispatch with headers; JSON request encoding and response comparison; status, text, and header assertions; malformed JSON includes parse cause and body; each mismatch reports expected and actual; request construction and response-body failures return `TestError`; lifecycle shuts down after an HTTP assertion panic.
- [ ] **Step 2: Run** `cargo test -p mads-testing --test http_fixture`; expect failure because `TestClient` and assertion methods are absent.
- [ ] **Step 3: Implement** `ControllerFixture` by reusing Task 3's runner and Task 2's router. Dispatch with Tower `ServiceExt::oneshot`; collect the Axum body once. Keep request construction and body reading fallible and assertions infallible except for deliberate panic. Use native HTTP types from Axum's re-export.
- [ ] **Step 4: Run** `cargo test -p mads-testing --test http_fixture` and `cargo test -p mads-testing`; expect all to pass.
- [ ] **Step 5: Commit** as `feat(testing): add in-process HTTP assertions`.

### Task 5: Module macro, public acceptance, and documentation

**Files:** Create `crates/mads-core-macros/src/test_module.rs`, `crates/mads-testing/tests/public_contract.rs`, `crates/mads/tests/test_attribute.rs`; modify `crates/mads-core-macros/src/lib.rs`, `src/path.rs`, `crates/mads-core/src/lib.rs`, `crates/mads/src/lib.rs`, `crates/mads-testing/README.md`, and root `README.md`. Add focused macro expansion tests beside the existing macro support tests.

**Interfaces:** Produce `#[mads::test]` from `mads-core-macros`, re-export through `mads-core` and `mads`. It accepts only an inline `ItemMod` with no attribute arguments, applies `#[cfg(test)]`, preserves existing module attributes/items, and inserts a private `fn test_fixture() -> mads_testing::TestFixtureBuilder` that calls the hidden constructor. Resolve renamed `mads-testing` dependencies with `proc-macro-crate`, following existing path helpers. Reject external modules and non-module items with `syn::Error`.

- [ ] **Step 1: Write failing macro and consumer tests**: inline module exposes `test_fixture()` and runs a service and controller fixture; external `mod tests;` and a function target produce the specified compile errors; a call to `test_fixture()` outside an annotated module fails to compile; renamed `mads-testing` dependency expands correctly; production `cargo check` does not compile the annotated test module. The consumer test must use public `mads` and `mads-testing` paths. Add `mads-testing` as a dev dependency of `mads` for that test.
- [ ] **Step 2: Run** `cargo test -p mads --test test_attribute` and the macro crate's focused test; expect failure because `mads::test` is absent.
- [ ] **Step 3: Implement** the macro and exports. Document the final controller and service examples, the explicit SQLite mock requirement, assertion behavior, and the scoped cleanup guarantee. State that hidden macro plumbing is outside the supported compile-time gate.
- [ ] **Step 4: Run** `cargo test -p mads --test test_attribute`, `cargo test -p mads-testing --test public_contract`, and `cargo check -p mads --lib`; expect all to pass.
- [ ] **Step 5: Commit** as `feat(testing): expose annotated test modules`.

## Final verification

- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`.
- [ ] Run the repository coverage gate from `.codex/AGENTS.md`: `cargo llvm-cov --workspace --all-features --ignore-filename-regex '(^|/)tests/ui/' --fail-under-lines 85`.
- [ ] Review `git diff` against the spec, including normal rooted/rootless behavior and the untouched pre-existing user directory content; report any environment-only test limitations with the exact failed command.
