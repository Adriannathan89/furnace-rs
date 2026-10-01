# Optional controller seals and endpoint skips

User-authorized follow-up to the furnace-rs migration, on `refactor/mads-declaration-module`, based on `6d1b3bb`. The user explicitly superseded the previous required-Sealable and no-endpoint-exceptions contracts, and selected `#[seal(skip)]`.

## Behavior

- A managed controller without `impl Sealable` is public. An explicit empty registration still means public.
- An explicit seal protects controller endpoints except methods marked `#[seal(skip)]`. Skip bypasses authentication and policy checks, including malformed credentials. It does not install an authenticated principal.
- Public and protected methods may share a canonical path. Implicit HEAD follows GET protection. Unsupported methods retain native 405 behavior.
- Only protected selected occurrences create Passport bindings and JWT/strategy requirements. Entirely skipped controllers, including focused fixtures and cfg-disabled protected endpoints, need no JWT output for their endpoints.
- Duplicate, unsupported, non-endpoint, and standalone seal markers produce compile errors. A controller still supports at most one seal.
- CLI `guard_active` reflects each endpoint. The public scaffold omits the formerly required empty `Sealable` implementation. Active documentation and examples are updated.

## Implementation

Generated metadata uses concrete-type autoref dispatch to choose `Sealable` when implemented and public metadata otherwise. There is no blanket trait implementation that conflicts with user implementations. Registration markers no longer require their controller to implement the trait. Existing analysis-local memoization and cauldron ownership remain intact.

Each generated protected method router receives a context-specific Passport `route_layer` before merging. Public methods do not receive it. Existing guard pipelines, typed extractors and native Axum handlers are retained. No dependency, version, protocol/schema or toolchain changes.

## Verification

Observed RED: a controller without Sealable failed to compile; endpoint skip markers were unknown; new unsupported-method regression returned 401 instead of 405. Each is GREEN after implementation/fix.

Targeted runtime coverage includes public defaults without JWT configuration, malformed/denied credential bypass, protected sibling POST, automatic public/protected HEAD, native 405, all-skipped rooted/focused controllers, disabled protected endpoints, cookie extraction, shared policies with distinct cauldron strategies, and private JWT visibility. Static inspection proves per-method `guard_active`.

Four new compile-fail marker cases pass, and the old missing-Sealable negative fixture is now a pass case. Default, renamed, shadowed and cookies-only external consumers compile public controllers without the trait. Final verification uses snapshot overwrite disabled.

Formatter and strict workspace Clippy passed. Core-only, HTTP-only, JWT-only and cookies checks passed; HTTP-only request tests passed. Separate workspace doctests, strict rustdoc, all three standalone examples and all nine package payload checks passed.

Final full stable workspace suite: exit 0, aggregate printed summaries 740 passed / 0 failed / 4 ignored (includes nested test summaries). Final MSRV 1.94 workspace suite: exit 0, aggregate printed summaries 740 passed / 0 failed / 4 ignored. Both full suites ran with snapshot overwrite disabled.

## Review

One fresh read-only reviewer checked optional dispatch, endpoint binding identity, skip isolation, HEAD, cfg selection, feature boundaries and malformed marker handling. Critical: none. Important: MethodRouter::layer wrapped the 405 fallback, and an old macro unit assertion still required a direct Sealable callback. The fallback defect was reproduced by a failing request test, then fixed with route_layer; the obsolete implementation-string assertion was removed while runtime/external-consumer coverage proves optional and explicit dispatch. The stale scaffold expectation was also updated after its observed failure.

Minor findings: none. Historical benchmark concerns are outside this follow-up and remain in the prior migration report. The initial dev-loop failure coincided with macro source edits while its watcher was running; a quiescent targeted rerun passed. Full suites are rerun with Rust source edits paused.

## External coverage

No test database is provisioned. The existing real PostgreSQL cases require `FURNACE_TEST_DATABASE_URL`; their ignored status is retained and CI must provide the external database coverage.

## Handoff

Implementation and verification complete. Keep the assigned branch locally; no push, merge, publish or version bump. Historical benchmark follow-ups remain outside this change.
