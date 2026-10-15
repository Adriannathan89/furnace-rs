# Codebase Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reconcile repository documentation with the 2.0.0-beta.2 implementation and document every production Rust module and function.

**Architecture:** A source-linked implementation guide will explain the cross-crate flows while focused package guides retain usage details. Rustdoc comments will live beside the implementation, grouped by crate ownership; a final audit will keep current release claims, historical records, links, and source references distinct.

**Tech Stack:** Markdown, Rustdoc, Cargo workspace, existing Rust tests and feature matrix.

**Spec:** [Codebase documentation design](../specs/2026-10-15-codebase-documentation-design.md)

## Global Constraints

- Source-level documentation covers production Rust code under workspace crate `src` trees, including private functions, trait methods, implementations, macro expansion helpers, and binary entry points.
- Test-only modules and functions, integration tests, UI fixtures, example application source, build artifacts, and generated Prost/Tonic code are outside this source-comment pass.
- The Markdown audit covers all tracked `.md` files.
- Release notes, changelogs, dated security and benchmark records, and completed design/plan documents are treated as historical snapshots: check their internal claims against their stated revision/date and repair broken references, but do not rewrite an accurate past claim as if it described the current implementation.
- Keep this work documentation-only: no runtime, API, feature, or dependency behavior changes.

## Review Focus

1. **Release framing:** a beta.2 command or feature must not be presented as part of the 1.0.2 released contract. Check the root README, CLI guide, and architecture guide against package versions and release notes.
2. **Feature-gated APIs:** claims about HTTP, JWT/Passport, cookies, gRPC, TLS, persistence, and Tokio must match `Cargo.toml` feature definitions and the existing facade feature-matrix tests.
3. **Mixed transport lifecycle:** HTTP/gRPC startup, readiness, bind failure, admission, drain, cancellation, and provider shutdown claims must match the coordinator, gRPC server, example, and runtime tests.
4. **Static inspection:** claims that inspection skips construction, lifecycle, connections, listener binding, and TLS reads must match the CLI handshake and inspection tests.
5. **Historical evidence links:** benchmark result files and removed-source references must identify the recorded artifact/revision, without changing the meaning of the historical result.

---

### Task 1: Reconcile current and historical Markdown documentation

**Files:**
- Inspect all 58 tracked Markdown files from `git ls-files '*.md'`.
- Modify existing Markdown files identified by the audit; the initial edit set includes `README.md`, `docs/README.md`, `docs/CLI.md`, `docs/ARCHITECTURE.md`, affected crate READMEs, `benchmark/COMPARISON.md`, and `benchmark/REPORT.md`. Create `docs/IMPLEMENTATION.md` only in Task 6.
- Verify claims against owning code and tests in `crates/*/src`, workspace `Cargo.toml` files, and relevant integration/UI tests.

- [x] Check each current claim against the current implementation and record any release-specific scope in its document.
- [x] Add a link to the forthcoming `docs/IMPLEMENTATION.md` from `README.md` and `docs/README.md`; verify the target after Task 6.
- [x] Update `docs/CLI.md` so beta.2 `furnace grpc` inspection is distinguishable from the 1.0.2 CLI release contract.
- [x] Clarify that the non-goals in `docs/ARCHITECTURE.md` describe the 1.0.2 scope, alongside the current beta.2 gRPC/TLS/multi-transport section.
- [x] Correct stale source maps in `crates/furnace-rs-core/README.md`, `crates/furnace-rs-core-macros/README.md`, and `crates/furnace-rs-common/README.md`.
- [x] Document the package purpose, feature/dependency boundaries, source layout, and focused tests in `crates/furnace-rs-grpc/README.md` and `crates/furnace-rs-grpc-macros/README.md`.
- [x] Repair the mismatched raw-result link in `benchmark/COMPARISON.md` and replace the removed local MADS source link in `benchmark/REPORT.md` with a stable reference to its recorded revision.
- [x] Preserve historical claims that are accurate for their stated release/revision; fix broken references and claims that are ambiguous as current behavior.

**Review check:** Confirm the version, feature, transport, and historical-evidence cases in Review Focus against the cited manifests, implementation, tests, and release record.

### Task 2: Document core runtime and core macros

**Files:**
- Modify production Rust files under `crates/furnace-rs-core/src/**/*.rs` and `crates/furnace-rs-core-macros/src/**/*.rs`.
- Exclude test-only files and `#[cfg(test)]` modules/functions.

- [x] Add module-level Rustdoc to every production module that lacks it.
- [x] Add meaningful Rustdoc comments to every production function and method, including private graph, configuration, catalog, macro parsing, and expansion helpers.
- [x] Explain each helper's contract and caller-relevant invariants, errors, ordering, and side effects without changing executable code.

**Review check:** Compare the documented graph-selection, construction, configuration, and macro metadata flow with the relevant core tests and compile-fail fixtures.

### Task 3: Document common HTTP, Passport, and macro implementation

**Files:**
- Modify production Rust files under `crates/furnace-rs-common/src/**/*.rs` and `crates/furnace-rs-common-macros/src/**/*.rs`.
- Exclude test-only files and `#[cfg(test)]` modules/functions.

- [x] Add module-level Rustdoc to every production module that lacks it.
- [x] Add meaningful Rustdoc comments to every production function and method, including private routing, extraction, validation, JWT/Passport, runtime-coordination, task, and macro helpers.
- [x] Describe transport/feature boundaries and security-sensitive behavior where callers rely on them; do not change executable code.

**Review check:** Compare the documented route, validation, error, Passport/JWT, and runtime contracts with existing integration, feature-matrix, and UI tests.

### Task 4: Document gRPC and CLI implementation

**Files:**
- Modify production Rust files under `crates/furnace-rs-grpc/src/**/*.rs`, `crates/furnace-rs-grpc-macros/src/**/*.rs`, and `crates/furnace-rs-cli/src/**/*.rs`.
- Exclude test-only files and `#[cfg(test)]` modules/functions.

- [x] Add module-level Rustdoc to every production module that lacks it.
- [x] Add meaningful Rustdoc comments to every production function and method, including private gRPC scope/preflight/client/TLS/server helpers and CLI build/inspection/watch/scaffold/output helpers.
- [x] Describe cancellation, deadline, drain, inspection, and redaction behavior at the functions that enforce those boundaries; do not change executable code.

**Review check:** Compare gRPC/client/runtime claims to gRPC tests, and CLI inspection/dev/scaffold claims to CLI integration and protocol tests.

### Task 5: Document facade, persistence, testing, and extra crates

**Files:**
- Modify production Rust files under `crates/furnace-rs/src/**/*.rs`, `crates/furnace-rs-persistence/src/**/*.rs`, `crates/furnace-rs-testing/src/**/*.rs`, and `crates/furnace-rs-extra/src/**/*.rs`.
- Exclude test-only files and `#[cfg(test)]` modules/functions.

- [x] Add module-level Rustdoc to every production module that lacks it.
- [x] Add meaningful Rustdoc comments to every production function and method, including private connector, fixture, lifecycle, and facade helpers.
- [x] Preserve all signatures, exports, feature gates, and executable code.

**Review check:** Compare database readiness/shutdown, test fixture cleanup, and facade feature/export claims with their existing tests and manifests.

### Task 6: Write the implementation guide

**Files:**
- Create `docs/IMPLEMENTATION.md`.

- [x] Explain the workspace crate map, dependency direction, facade, and feature composition with links to source.
- [x] Explain macro expansion and metadata registration, module scope/provider graph analysis, preflight, construction, lifecycle, and configuration flow.
- [x] Explain HTTP routing/validation/Passport and gRPC service/client registration, authorization, TLS, reflection/health, static inspection, and shared runtime coordination.
- [x] Explain persistence readiness, CLI build/inspection/dev/scaffold flow, and which existing tests cover the subsystem boundaries.
- [x] Keep user-facing syntax/configuration examples in the existing task-specific guides and link to them.

**Review check:** Follow every flow from the guide into its owning source modules and verify that links target the implementations described.

### Task 7: Run documentation checks and inspect the complete diff

**Files:**
- Inspect all changed Rust and Markdown files; do not add generated artifacts.

- [x] Confirm module/function comment coverage by reviewing production source declarations, not only public-item lint output.
- [x] Check all local Markdown links and source-layout references; verify historical links identify the correct artifact or source revision.
- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo test --workspace --all-features --doc`.
- [x] Run `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`.
- [x] Run focused behavior tests only if an audited claim exposed a discrepancy requiring an implementation change; the audit found no documentation discrepancy requiring code changes.
- [x] Run `git diff --check` and confirm the diff changes only Markdown and Rustdoc comments; executable statements, signatures, features, dependencies, and behavior remain unchanged.

**Review check:** All spec acceptance criteria and five Review Focus cases were checked against package manifests, implementation, tests, and recorded revision links. Workspace doctests and warning-denied Rustdoc passed; the source declaration coverage scan found no undocumented production modules or functions.
