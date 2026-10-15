# Codebase documentation design

**Status:** Approved
**Date:** 2026-10-15
**Branch:** `docs/code-base-documentation`

## Problem

The workspace has user guides and crate READMEs, but they do not yet explain the
implementation as a whole. Some source-layout entries no longer match the tree,
the gRPC crate READMEs are empty, and the current CLI guide introduces a beta.2
command while framing itself as 1.0.2 documentation. The architecture guide also
places a 1.0.2 non-goals list beside beta.2 gRPC support without making the
version boundary prominent. Rustdoc coverage is uneven in private modules and
functions.

## Goals

1. Check every tracked Markdown document and production Rust documentation
   against the current source, feature definitions, tests, and release context.
2. Correct current-documentation claims that conflict with the implementation,
   repair invalid repository links, and make release-version scope explicit.
3. Add a source-linked implementation guide that explains the workspace
   boundaries and the major compile-time and runtime flows.
4. Add module-level Rustdoc to every production Rust source module and useful
   Rustdoc comments to every production function and method, including private
   items.
5. Keep this work documentation-only: no runtime, API, feature, or dependency
   behavior changes.

## Documentation boundary

Source-level documentation covers production Rust code under workspace crate
`src` trees, including private functions, trait methods, implementations, macro
expansion helpers, and binary entry points. Test-only modules and functions,
integration tests, UI fixtures, example application source, build artifacts, and
generated Prost/Tonic code are outside this source-comment pass. Existing tests
remain evidence for documented behavior.

The Markdown audit covers all tracked `.md` files. Current user and contributor
guides are reconciled with this checkout. Release notes, changelogs, dated
security and benchmark records, and completed design/plan documents are treated
as historical snapshots: check their internal claims against their stated
revision/date and repair broken references, but do not rewrite an accurate past
claim as if it described the current implementation.

## Proposed documentation structure

Add `docs/IMPLEMENTATION.md` as the central implementation guide. It will
include source links and explain:

- the workspace crates, dependency direction, feature gates, and facade;
- how procedural macros parse declarations and emit typed constructors,
  descriptors, and inspection metadata;
- how catalogs, selected cauldron scope, provider graph analysis, preflight,
  deterministic construction, and lifecycle startup/shutdown fit together;
- conventional and explicit configuration loading, source attribution, typed
  parsing, and secret handling;
- HTTP route registration, request validation, error boundaries, and Passport
  protection;
- protobuf service/client metadata, gRPC preflight and dispatch, Bearer/JWT
  admission, TLS, reflection/health, static inspection, and shared HTTP/gRPC
  lifecycle coordination;
- persistence connector ownership/readiness and the CLI's build, inspection,
  development-loop, and scaffold flows; and
- which test suites exercise each boundary.

Link this guide from the root README and documentation index. Keep existing
task-focused guides authoritative for user-facing syntax and configuration.
Repair crate README source maps and fill the gRPC package README gaps where they
conflict with or fail to describe the current package boundary.

## Current issues found during read-only exploration

- `docs/CLI.md` starts by describing FURNACE 1.0.2 although the current CLI
  implementation and the guide include beta.2 `furnace grpc` inspection.
- `docs/ARCHITECTURE.md` needs a clear historical 1.0.2 scope around its
  non-goals so they do not read as current claims that contradict beta.2 gRPC
  TLS and multi-transport support.
- `crates/furnace-rs-core-macros/README.md` lists a nonexistent
  `src/provider.rs`; `crates/furnace-rs-core/README.md` lists a nonexistent
  `src/config/`; and `crates/furnace-rs-common/README.md` lists a nonexistent
  `src/extract/` directory where the implementation is `src/extract.rs`.
- The READMEs for `furnace-rs-grpc` and `furnace-rs-grpc-macros` are empty.
- The local Markdown-link scan found a mismatched raw-result filename in
  `benchmark/COMPARISON.md` and a removed MADS source path in
  `benchmark/REPORT.md`; the latter is historical and should link to the
  corresponding recorded source revision or clearly identify it as archived.

These are initial findings, not a complete audit. The implementation pass must
verify every substantive current claim against the owning code and tests.

## Writing principles

- Explain responsibilities, data flow, invariants, side effects, ownership,
  failure behavior, and feature conditions where they matter.
- Avoid comments that merely restate a function or type name.
- For private helpers, describe why the helper exists and the constraints its
  caller relies on; document errors, ordering, cancellation, and security
  boundaries when applicable.
- Link to the owning source module instead of copying implementation detail
  across guides. Keep historical benchmark and security evidence attributed to
  its measured/checked revision.
- Make version-specific statements name the relevant release or feature set.

## Acceptance criteria

- The implementation guide covers every workspace crate and the principal
  compile-time, startup, request, transport, persistence, CLI, and testing flows
  with links to their implementation.
- Every production module has module-level Rustdoc, and every production
  function/method has a meaningful Rustdoc comment; test-only code is excluded.
- Current guides and crate READMEs do not contradict source, feature gates, or
  verified behavior. Historical records remain accurate to their stated time.
- Local Markdown links resolve, source-layout references point to real files,
  and release/version framing is explicit.
- Documentation builds and doctests pass, and the final diff contains no
  behavior or dependency changes.

## Verification

Run formatting and documentation checks, including the workspace's all-feature
Rustdoc build and doctests. Re-run focused tests only where auditing a behavior
claim exposes a discrepancy; documentation-only comment changes do not require
a redundant full behavior-test matrix. Check all local Markdown links and source
path references, then run `git diff --check` and inspect the complete diff.
