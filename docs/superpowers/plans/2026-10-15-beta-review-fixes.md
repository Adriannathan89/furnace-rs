# Beta review fixes

Authority: the user approved all solutions in `docs/issue.md` and requested a defensive F01 before/after comparison under `benchmark/`. Work stays on assigned branch `test/beta-v2-validation-test`.

The user subsequently requested commits per batch. Commit the validated groups separately: gRPC/evidence, core, HTTP/macros, CLI, dependencies/CI, then resolution documentation. Do not push or merge.

## Design and constraints

Use focused regression tests with synthetic data. F01 exercises generated RPCs in process; no external target or offensive traffic is involved. Preserve valid configurations and explicit public routes. Retain evidence of the original behavior and distinguish compilation from test execution. No publishing, pushing, or merging is requested.

## Tasks

1. **gRPC and F01 evidence (F01, F12).** Add conditional-policy regression fixtures and a benchmark runner/report comparing the recorded baseline to current source. Apply method conditions to metadata, handle cfg_attr, scope helper names per expansion. Validate anonymous rejection, authenticated success, explicit public controls, and equal final type names.
2. **Core (F03, F10, F11).** Make interrupted lifecycle cleanup resumable, reject ambiguous TOML segments/collisions, and apply consistent table/scalar/array source precedence. Add cancellation and configuration regression tests; run core tests.
3. **HTTP and macros (F04, F05, F13, F14).** Reject unsupported conditional controller fields clearly; model newtype/transparent validation paths; enforce matching response cookie name policy; retain relevant middleware headers during timeout normalization. Add focused regressions and run affected tests.
4. **CLI (F06–F09).** Retain Cargo invocation scope, provide Cargo-compatible library search paths, watch protobuf/build inputs, and refresh metadata/watch registrations after manifest changes. Add meaningful fixture coverage and run CLI tests.
5. **Dependencies and integration (F02).** Update three committed Rustls locks and audit every committed lock in CI. Run formatting, lint, workspace tests and feature checks appropriate to the changes. Review all diffs and update `docs/issue.md` with resolution and evidence.

## Progress

- Initial review: task scopes have separate source ownership. Shared integration surfaces are tests/build artifacts and final documentation, owned by the coordinator. Core lifecycle changes must preserve runtime startup/cleanup ordering. CLI watcher refresh must retain build cancellation semantics. No scope conflicts found.
- Ruling: use the existing user-assigned branch and workspace; do not create another branch or move the untracked issue report.
- Ruling: source precedence replaces incompatible node shapes and removes obsolete descendants, while compatible tables continue merging by field.
- Task 1: complete. F01 failed on the unchanged macro with anonymous_rejected=0/2 and passed after the policy fix with 2/2; F12 compiled only after helper scoping. The durable comparison recorded 200 rounds against an isolated baseline and patched source, with verified matching fixture/runner/relevant-source fingerprints. Conditional-attribute and namespace fixtures pass; independent review found no defect. Committed b14130f.
- Task 2: complete. Five configuration and three lifecycle regressions failed before patching; all 200 core tests and scoped Clippy pass. Independent HTTP-agent review found no defect. Pending hooks retain documented cancellation/retry obligations.
- Task 3: complete. Macro, real-JSON, cookie, and HTTP/1.1+HTTP/2 timeout regressions validated; full common/common-macros suites, facade compile-fail fixtures, and scoped Clippy pass. Independent core-agent review found no defect.
- Task 4: complete. Invocation scope, Cargo-mediated compiler/sysroot discovery, loader paths, and dependency refresh are implemented. Declared inputs replace blanket file watching, normalized fresh watchers handle external/missing paths and recursion, registration is acknowledged before process launch, and metadata reload is cancellable. All reviewer findings were addressed. Full CLI suite exit0 with 174 reported passes (172 top-level plus 2 nested); final watcher tests 7/7 and final Clippy pass. Committed 286c1c7.
- Task 5: complete. Three Rustls locks updated to 0.23.46; all 11 committed lockfiles pass cached RustSec scan; CI and both release workflows audit all committed locks. Non-CLI workspace tests exit0 with 778 passes and 7 live-PostgreSQL tests ignored. Non-CLI and CLI all-target Clippy pass; formatting/whitespace and 21 Python security tests pass. Independent source reviews approved all fixes. Dependency batch committed 6245c95; core ce2aa46; HTTP 2520bad. Final report records test scope and limitations.
- Ruling: reject conditional controller fields explicitly until their dependency tuple shapes can be generated consistently. This implements the approved fallback and avoids silently compiling inconsistent metadata.
- Ruling: benchmark evidence records failed security assertions on the baseline as the expected pre-fix outcome; missing markers/build failures cannot satisfy the comparison.
