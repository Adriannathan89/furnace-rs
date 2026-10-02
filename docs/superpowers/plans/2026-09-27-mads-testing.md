# MADS Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a module-free `mads-testing` fixture for focused provider construction, SQLite-backed SeaORM mocks, and in-process controller assertions with reliable lifecycle shutdown.

**Execution status (2026-10-01):** Tasks 1–5 implemented on `feat/mads-testing`.
The final review's focused Passport isolation issue was fixed. Workspace tests
passed (711 passed, 7 ignored); formatting, Clippy, documentation, and the
coverage gate passed (89.06% line coverage). Tests that bind localhost required
execution outside the restricted sandbox. Two minor review notes remain:
the duplicate-supply test lacks an explicit constructor sentinel, and the
malformed-JSON test does not specifically assert the parser cause.

**Architecture:** Add a focused provider selection path to core and a selected-controller route path to common HTTP. The testing crate owns the public fixture, mock validation, request/assertion API, and scoped lifecycle runner. `#[mads::test]` registers one async function with Cargo through Tokio and generates `test_fixture()` inside that function.

**Tech Stack:** Rust 2024, Rust 1.94, MADS core/common/facade, SeaORM 2.0 mock, Axum 0.8, Tower 0.5, Tokio, serde_json, futures-util.

**Spec:** `docs/superpowers/specs/2026-09-27-mads-testing-design.md`

## Global Constraints

- `mads-testing` is added at `crates/mads-testing`, currently an untracked empty user directory; inspect it before edits and do not discard user files.
- SeaORM 2.0 `MockDatabase` with `DbBackend::Sqlite` is the only database fixture. No SQLite driver, live database, socket, or file is used.
- A selected `DatabaseConnection` dependency requires an explicit `.mock_database(...)`; production database providers must not run as fallback. Direct `.provide::<DatabaseConnection>(...)` fails setup.
- Focused tests may cross MADS module import and private-provider boundaries. Rust language privacy remains enforced. Normal rooted and complete-catalog behavior is unchanged.
- `#[mads::test]` accepts one zero-argument, nongeneric `async fn`, applies `#[cfg(test)]` and Tokio's test attribute, and generates function-local `test_fixture()`. `cargo test` discovers it without a separate `#[tokio::test]`. Sync functions and non-function items are rejected.
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
| `crates/mads-core-macros/src/test_fn.rs` | Async test-function attribute expansion and function-local fixture entry. |
| `crates/mads-core-macros/src/lib.rs`, `src/path.rs`; `crates/mads-core/src/lib.rs`; `crates/mads/src/lib.rs` | Export macro and resolve a renamed `mads-testing` dependency. |
| `Cargo.toml`, `crates/mads-testing/Cargo.toml`, `Cargo.lock` | Workspace membership and dependencies. |
| `crates/mads-testing/README.md`, `README.md` | Public controller and service usage. |

### Task 1: Focused provider graph and construction

**Interfaces:** Produce document-hidden `MadsBuilder::__test_focus<T: Send + Sync + 'static>(&mut self) -> mads_core::Result<&mut Self>` and `MadsBuilder::__test_require_provided<T: Send + Sync + 'static>(&mut self) -> &mut Self`. Root and focus are mutually exclusive (`MADS008`). Focused analysis selects `T` and its transitive registered dependencies, stops at supplied values, and treats required-supply types as missing until supplied. It reuses `analyze_descriptors` and the existing construction plan.

**Implementation by file:**

| File | Required change |
| --- | --- |
| `crates/mads-core/src/graph/focus.rs` (create) | Walk `Catalog::providers()` from the requested `TypeId` in deterministic dependency order. Retain matching descriptors for ambiguity checks; traverse dependencies of the selected descriptor, stop at supplied values, and record missing required supplies. Do not inspect unrelated providers or module imports. |
| `crates/mads-core/src/graph/mod.rs` | Expose the focus selector to `builder.rs` as a crate-private function; keep `analyze_descriptors` as the common validation path. |
| `crates/mads-core/src/builder.rs` | Store a focus selector separately from `root`; implement the two hidden methods; dispatch focused builds to a focused-analysis branch, then use the current construction loop and lifecycle registration. Preserve the existing rooted and complete-catalog branches. |
| `crates/mads-core/tests/focused_selection.rs` (create) | Hold isolated descriptor fixtures and assertions for the focused graph; avoid shared mutable state across parallel tests unless guarded. |

**TDD cycle A — selection and overrides**

- [ ] **Step 1: Write failing tests** `focus_builds_private_transitive_chain_without_module` and `focus_uses_supplied_value`. Assert `graph.provider::<Service>()` and `graph.provider::<Repository>()` are present, an unrelated provider is absent and never invoked, and the supplied dependency's constructor counter remains zero.

  ```rust
  assert!(app.graph().provider::<Service>().is_some());
  assert!(app.graph().provider::<Repository>().is_some());
  assert!(app.graph().provider::<Unrelated>().is_none());
  assert_eq!(UNRELATED_CALLS.load(Ordering::SeqCst), 0);
  ```
- [ ] **Step 2: Run** `cargo test -p mads-core --test focused_selection`; expect compile failure for missing `__test_focus`.
- [ ] **Step 3: Implement** focus state in `builder.rs`, traversal in `focus.rs`, and the `graph/mod.rs` bridge. Use the existing `analyze_descriptors` result and construction plan.
- [ ] **Step 4: Run** `cargo test -p mads-core --test focused_selection`; expect cycle A tests to pass.

**TDD cycle B — fail-closed selection**

- [ ] **Step 5: Write failing tests** `focus_requires_external_type` (`MADS003`, no constructor call), `focus_reports_ambiguous_selected_type` (`MADS002`), `focus_reports_missing_selected_type` (`MADS003`), `focus_reports_selected_cycle` (`MADS005`), `focus_rejects_root_mix` (`MADS008`), and `focus_ignores_unrelated_auto_configuration` (evaluator counter zero). Include a supplied target type whose descriptor is still checked for ambiguity.

  ```rust
  assert_eq!(builder.analyze().diagnostics()[0].code(), MADS002);
  assert_eq!(UNRELATED_AUTO_EVALUATIONS.load(Ordering::SeqCst), 0);
  ```
- [ ] **Step 6: Run** `cargo test -p mads-core --test focused_selection`; expect the new cases to fail for their asserted reason.
- [ ] **Step 7: Implement** required-supply barriers and selection diagnostics. Prevent a static provider or auto-configuration from satisfying a required-supply type. Evaluate auto-configurations only against the focused chain.
- [ ] **Step 8: Run** `cargo test -p mads-core --test focused_selection` and `cargo test -p mads-core --test auto_configuration_builder --test module_scope`; expect all to pass. Run `cargo fmt --all --check` after formatting.
- [ ] **Step 9: Commit** as `feat(testing): select one provider dependency chain`.

### Task 2: Selected-controller router

**Interfaces:** Consume `MadsBuilder::__test_focus<T>()`. Produce document-hidden `mads_common::__private::build_test_router_for<T: Send + Sync + 'static>(&mads_core::Mads) -> mads_core::Result<axum::Router>`. Resolve exactly one `ControllerRouteDescriptor` by `TypeId`; absent or ambiguous metadata fails setup. Reuse route validation, the typed registrar, and selected Passport preflight when JWT is enabled.

**Implementation by file:**

| File | Required change |
| --- | --- |
| `crates/mads-common/src/http_scope.rs` | Add a constructor for a one-controller scope, including that controller's routes and only guards referenced by them. Leave rooted and complete scopes unchanged. |
| `crates/mads-common/src/route.rs` | Feed the one-controller scope to existing `validate_scoped_descriptors`; retain its `MADS030` conflict and metadata diagnostics. Report absent or ambiguous controller identity before registration. |
| `crates/mads-common/src/router.rs` | Build the selected router with the current `RouterBuildContext`, validated route iterator, and generated registrar. Factor shared registration logic if needed, without altering `build_router`. |
| `crates/mads-common/src/lib.rs` | Export the new function only from document-hidden `__private`. |
| `crates/mads-common/tests/focused_router.rs` (create) | Define two controllers and deliberately invalid unselected metadata; assert behavior through Tower `oneshot`. |

**TDD cycle A — one controller serves**

- [ ] **Step 1: Write failing tests** `builds_only_selected_controller_routes`: a selected GET returns status 200 and its body, while the second controller's path returns 404. Test `rejects_absent_controller_metadata` for a registered provider with no route descriptor.

  ```rust
  assert_eq!(selected.status(), StatusCode::OK);
  assert_eq!(unselected.status(), StatusCode::NOT_FOUND);
  ```
- [ ] **Step 2: Run** `cargo test -p mads-common --features http --test focused_router`; expect compile failure for missing `build_test_router_for`.
- [ ] **Step 3: Implement** descriptor lookup, one-controller scope, and router registration in the mapped files.
- [ ] **Step 4: Run** `cargo test -p mads-common --features http --test focused_router`; expect cycle A tests to pass.

**TDD cycle B — selected validation only**

- [ ] **Step 5: Write failing tests** `ignores_invalid_unselected_controller` (selected route still serves), `rejects_invalid_selected_route` (`MADS030`), and `rejects_ambiguous_selected_controller_metadata` (deterministic setup error). Under `cfg(feature = "jwt")`, add `selected_guard_preflight_ignores_unselected_guard` and assert only the selected guard is required.
- [ ] **Step 6: Run** `cargo test -p mads-common --all-features --test focused_router`; expect the new cases to fail for their asserted reason.
- [ ] **Step 7: Complete** selected validation and guard preflight in `http_scope.rs`, `route.rs`, and `router.rs`; reuse the existing route validator and registrar.
- [ ] **Step 8: Run** `cargo test -p mads-common --features http --test focused_router`, `cargo test -p mads-common --all-features --test focused_router`, and the existing `mads-common` router tests; expect all to pass.
- [ ] **Step 9: Commit** as `feat(testing): build a focused controller router`.

### Task 3: Subject fixture, mock database, and lifecycle runner

**Interfaces:** Produce `TestFixtureBuilder`, `SubjectFixture<T>`, `TestContext`, `TestError`, `TestResult<T>`, and `mads_testing::sea_orm` re-export. `TestFixtureBuilder::mock_database(MockDatabase) -> Self`, `provide<T: Send + Sync + 'static>(T) -> Self`, and `subject<T: Send + Sync + 'static>() -> SubjectFixture<T>` follow the spec. `SubjectFixture<T>::run<F, Fut>(self, F) -> TestResult<()>` takes `F: FnOnce(TestContext) -> Fut`, `Fut: Future<Output = ()>`. `TestContext::resolve<U: Send + Sync + 'static>(&self) -> mads_core::Result<Arc<U>>`. `TestError` distinguishes `Mads`, `MissingMockDatabase`, `UnsupportedMockBackend`, `DuplicateSupply`, `DirectDatabaseSupply`, `Request`, `Serialization`, `Service`, and `ResponseBody`; `TestResult<T> = Result<T, TestError>`.

**Implementation by file:**

| File | Required change |
| --- | --- |
| Root `Cargo.toml` | Add `crates/mads-testing` to members and a workspace `futures-util` entry if absent. Do not change existing package versions. |
| `crates/mads-testing/Cargo.toml` | Fill the existing empty manifest with workspace package metadata and dependencies on `mads-core`, `mads-common` (`http`), SeaORM (`mock` and Tokio runtime), `futures-util`, `serde`, `serde_json`, `tower`, and `tokio`. Keep dependencies free of `mads` and `mads-persistence`; the public facade acceptance tests belong in `mads`. |
| `crates/mads-testing/src/lib.rs` | Document and export the fixture/error types; re-export SeaORM mock types in `sea_orm`; expose Tokio's test attribute and the fixture constructor only under document-hidden macro support. |
| `crates/mads-testing/src/error.rs` | Define `TestError`, `TestResult`, `Display`, and `std::error::Error::source`; keep MADS diagnostics as sources and name setup/request/body error kinds explicitly. |
| `crates/mads-testing/src/fixture.rs` | Store one selected subject and supplied values by concrete type; reject duplicate supplies and raw `DatabaseConnection`; check `MockDatabaseTrait::get_database_backend() == DbBackend::Sqlite`, convert to native connection, set core's required-supply barrier, build/start, expose `TestContext`, catch body unwind, await shutdown, and resume panic. |
| `crates/mads-testing/tests/subject_fixture.rs` | Exercise public fixture behavior through registered service/repository fixtures and hook counters. Use the hidden constructor only until Task 5 provides `#[mads::test]`. |
| `crates/mads-testing/README.md`, `Cargo.lock` | Add the crate's minimal README for its manifest and resolve dependency lock changes; Task 5 fills usage documentation. |

**TDD cycle A — mock and dependency chain**

- [ ] **Step 1: Add manifest, workspace membership, and minimal `src/lib.rs`; write failing tests** `service_uses_mock_database` (queued SeaORM row is returned through service → repository and `resolve::<UserService>()` succeeds), `private_provider_is_selectable`, `missing_mock_never_connects` (`MissingMockDatabase`, connector counter zero), `postgres_mock_is_rejected` (`UnsupportedMockBackend`), `direct_database_supply_is_rejected`, and `duplicate_supply_is_rejected_before_construction` (subject counter zero). Use `mads-core` macros for fixture types and a fresh mock for every test.

  ```rust
  assert!(matches!(error, TestError::MissingMockDatabase));
  assert_eq!(PRODUCTION_CONNECTS.load(Ordering::SeqCst), 0);
  assert_eq!(SUBJECT_CONSTRUCTIONS.load(Ordering::SeqCst), 0);
  ```
- [ ] **Step 2: Run** `cargo test -p mads-testing --test subject_fixture`; expect compile failure for absent fixture exports after the manifest is valid.
- [ ] **Step 3: Implement** `error.rs` and the focused subject and mock path in `fixture.rs`. Defer repeated-supply errors until setup in `run`, as the builder methods return `Self`. Check the database barrier before `MadsBuilder::build`.
- [ ] **Step 4: Run** `cargo test -p mads-testing --test subject_fixture`; expect cycle A tests to pass.

**TDD cycle B — lifecycle and panic cleanup**

- [ ] **Step 5: Write failing tests** `start_and_stop_run_once` (ordered counters), `panic_still_stops` (catch original panic and assert stop counter), `panic_and_stop_failure_preserves_panic` (subprocess checks original panic plus shutdown diagnostic on stderr), `startup_failure_rolls_back` (started hooks stop in reverse order, body counter zero), and `shutdown_failure_returns_error` (MADS cause retained).
- [ ] **Step 6: Run** `cargo test -p mads-testing --test subject_fixture`; expect the new lifecycle cases to fail for their asserted reason.
- [ ] **Step 7: Implement** `run` in `fixture.rs`: build then start, move a cloned `ApplicationContext` into `TestContext`, catch unwinding panics with `FutureExt::catch_unwind`, always await `Mads::shutdown` after the body, report a shutdown error during panic, and resume the original panic. Do not promise cleanup on cancellation or abort.
- [ ] **Step 8: Run** `cargo test -p mads-testing --test subject_fixture`, `cargo check -p mads-testing`, and `RUSTDOCFLAGS="-D warnings" cargo doc -p mads-testing --no-deps`; expect all to pass.
- [ ] **Step 9: Commit** as `feat(testing): add focused subject fixture and lifecycle`.

### Task 4: In-process HTTP client and assertions

**Interfaces:** Consume `mads_common::__private::build_test_router_for<T>()`. Produce `TestFixtureBuilder::controller<T: Send + Sync + 'static>() -> ControllerFixture<T>` and `ControllerFixture<T>::run<F, Fut>(self, F) -> TestResult<()>` with `F: FnOnce(TestClient) -> Fut`, `Fut: Future<Output = ()>`. `TestClient` provides `resolve<U>()`, `request(Method, &str)`, and `get/post/put/patch/delete`. `TestRequest` provides `header(HeaderName, HeaderValue)`, `json<S: Serialize>(&S) -> TestResult<Self>`, and async `send() -> TestResult<TestResponse>`. `TestResponse` owns buffered status, headers, and body; all four `assert_*` methods return `Self` and panic with expected/actual values on mismatch.

**Implementation by file:**

| File | Required change |
| --- | --- |
| `crates/mads-testing/src/fixture.rs` | Add the controller selector and reuse Task 3's setup, start, panic, and shutdown runner. Build the selected router before the body and pass `TestClient` into it. |
| `crates/mads-testing/src/http.rs` (create) | Hold a cloned Axum router and application context; build native requests, encode JSON and headers, dispatch with Tower `ServiceExt::oneshot`, and return a buffered response. |
| `crates/mads-testing/src/response.rs` (create) | Store status, header map, and body bytes once. Implement status, JSON value, UTF-8 text, and header assertions with clear expected/actual panic messages. |
| `crates/mads-testing/src/error.rs`, `src/lib.rs` | Map request creation, JSON encoding, service dispatch, and body read failures to `TestError`; export the HTTP types with public documentation. |
| `crates/mads-testing/tests/http_fixture.rs` (create) | Exercise all public request methods through a selected generated controller, plus assertion failures and teardown. |

**TDD cycle A — in-process dispatch**

- [ ] **Step 1: Write failing tests** `controller_get_uses_selected_chain` (200 and expected body), `unselected_route_is_404`, `post_json_reaches_handler` (decoded payload and status), and `all_methods_dispatch` (PUT, PATCH, DELETE, explicit `request(Method::HEAD, ...)`, and headers reach the expected route behavior).

  ```rust
  app.get("/users/1").send().await.unwrap()
      .assert_status(StatusCode::OK)
      .assert_json(json!({ "id": 1 }));
  app.get("/unselected").send().await.unwrap()
      .assert_status(StatusCode::NOT_FOUND);
  ```
- [ ] **Step 2: Run** `cargo test -p mads-testing --test http_fixture`; expect compile failure for missing `controller`, `TestClient`, or `send`.
- [ ] **Step 3: Implement** the controller run path in `fixture.rs` and request dispatch in `http.rs`. Call Task 2's selected router and buffer the response body once.
- [ ] **Step 4: Run** `cargo test -p mads-testing --test http_fixture`; expect cycle A tests to pass.

**TDD cycle B — assertions and errors**

- [ ] **Step 5: Write failing tests** `assertions_compare_status_json_text_and_header` (all pass on exact values), `mismatch_reports_expected_and_actual` (capture panic text), `malformed_json_reports_body_and_parse_cause`, `request_and_body_errors_are_results`, and `assertion_panic_still_shuts_down` (hook stop counter one). Compare JSON as `serde_json::Value`, including reordered object keys.
- [ ] **Step 6: Run** `cargo test -p mads-testing --test http_fixture`; expect these assertions to fail or be absent.
- [ ] **Step 7: Implement** `response.rs` assertions and remaining `TestError` mapping. Keep every `assert_*` chainable without rereading a body. Ensure failures from `send` return `TestError`, while comparisons panic.
- [ ] **Step 8: Run** `cargo test -p mads-testing --test http_fixture`, `cargo test -p mads-testing`, and `cargo fmt --all --check`; expect all to pass.
- [ ] **Step 9: Commit** as `feat(testing): add in-process HTTP assertions`.

### Task 5: Async test-function macro, Cargo discovery, and documentation

**Interfaces:** Produce `#[mads::test]` from `mads-core-macros`, re-exported through `mads-core` and `mads`. It accepts one zero-argument, nongeneric `async fn` and no attribute arguments. It preserves the function's name, visibility, other attributes, body, and return type; applies `#[cfg(test)]` and Tokio's test attribute via `mads_testing::__private::tokio::test`; and inserts a function-local `fn test_fixture() -> mads_testing::TestFixtureBuilder`. Resolve renamed `mads-testing` dependencies with `proc-macro-crate`. Reject sync functions, arguments, generics, modules, and other non-function items with `syn::Error`.

**Implementation by file:**

| File | Required change |
| --- | --- |
| `crates/mads-core-macros/src/test_fn.rs` (create) | Parse and validate `syn::ItemFn`; emit one Cargo-discoverable Tokio test with a local `test_fixture()` inserted before the original body statements. Keep the fixture name scoped to that function. |
| `crates/mads-core-macros/src/path.rs` | Add `testing_path()` using `proc_macro_crate::crate_name("mads-testing")` and normalize renamed package identifiers; give a compile-time message when the dev dependency is missing. |
| `crates/mads-core-macros/src/lib.rs` | Register and document `#[proc_macro_attribute] pub fn test`; reject attribute arguments. |
| `crates/mads-core/src/lib.rs`, `crates/mads/src/lib.rs` | Re-export the attribute as `mads::test` without introducing a normal `mads` → `mads-testing` dependency. |
| `crates/mads/Cargo.toml` | Add `mads-testing` only as a dev dependency for public macro acceptance tests. |
| `crates/mads-testing/src/lib.rs` | Confirm the document-hidden constructor and Tokio test-attribute re-export match the macro's emitted paths. |
| `crates/mads/tests/test_attribute.rs` (create) | Hold two `#[mads::test] async fn` acceptance tests: one service, one HTTP controller. Include a test that exercises compile-fail fixtures with `trybuild`. |
| `crates/mads/tests/ui-test-attribute/` (create) | Compile-fail fixtures for sync function, parameters, generics, module target, macro arguments, and `test_fixture()` outside an annotated function. Preserve expected `.stderr` files. |
| `crates/mads/tests/consumers/renamed_testing/` (create) | A small Cargo consumer with `mads-testing` renamed in its manifest and its own `[workspace]`; run its annotated async test to prove path resolution. |
| `crates/mads-testing/README.md`, root `README.md` | Show `#[mads::test] async fn`, the explicit SQLite mock, controller assertions, service resolution, and scoped lifecycle cleanup. Remove module-level macro examples. |

**TDD cycle A — a function that Cargo runs**

- [ ] **Step 1: Add `mads-testing` as a `mads` dev dependency and write failing tests**: `#[mads::test] async fn service_fixture_runs()` resolves a selected service, and `#[mads::test] async fn controller_fixture_runs()` sends one request. Add a macro expansion unit test asserting the emitted function contains `#[cfg(test)]`, the Tokio test attribute, and a local `test_fixture()`; assert no module-level helper is emitted.

  ```rust
  #[mads::test]
  async fn service_fixture_runs() {
      test_fixture()
          .mock_database(MockDatabase::new(DbBackend::Sqlite))
          .subject::<UserService>()
          .run(|context| async move {
              assert!(context.resolve::<UserService>().is_ok());
          }).await.unwrap();
  }
  ```
- [ ] **Step 2: Run** `cargo test -p mads --test test_attribute` and `cargo test -p mads-core-macros test_fn`; expect failure because `mads::test` is absent.
- [ ] **Step 3: Implement** `test_fn.rs`, the path helper, and macro re-exports. Use `mads-testing`'s hidden Tokio re-export so the consumer only needs `mads-testing` as a dev dependency for this attribute.
- [ ] **Step 4: Run** `cargo test -p mads --test test_attribute` and `cargo test -p mads --test test_attribute -- --list`; expect both annotated function names in Cargo's listing and both tests to pass.

**TDD cycle B — compile-time boundaries**

- [ ] **Step 5: Write failing compile-fail tests** for sync `fn`, an async function with parameters, a generic async function, a module target, attribute arguments, and a helper call in an unannotated function. Add `consumers/renamed_testing/Cargo.toml` with a renamed `mads-testing` dev dependency and an annotated test. Assert each compile error points at the offending function or item.
- [ ] **Step 6: Run** `cargo test -p mads --test test_attribute`; expect the new `trybuild` cases to fail until diagnostics and path resolution match. Capture `.stderr` only after checking that each diagnostic is intentional.
- [ ] **Step 7: Complete** signature validation and renamed-crate resolution in the macro files. Add the standalone consumer test and documentation; verify the macro preserves an allowed async test return type.
- [ ] **Step 8: Run** `cargo test -p mads --test test_attribute`, `cargo test -p mads-core-macros test_fn`, `cargo test --manifest-path crates/mads/tests/consumers/renamed_testing/Cargo.toml`, and `cargo check -p mads --lib`; expect all to pass. Check the `-- --list` output again after compile-fail fixtures are added.
- [ ] **Step 9: Commit** as `feat(testing): register scoped async tests`.

## Final verification

- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`.
- [ ] Run the repository coverage gate from `.codex/AGENTS.md`: `cargo llvm-cov --workspace --all-features --ignore-filename-regex '(^|/)tests/ui/' --fail-under-lines 85`.
- [ ] Review `git diff` against the spec, including normal rooted/rootless behavior and the untouched pre-existing user directory content; report any environment-only test limitations with the exact failed command.
