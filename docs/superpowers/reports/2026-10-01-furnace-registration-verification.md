# Furnace registration implementation verification

Implemented on `refactor/mads-declaration-module`, starting from `7522026`.
The user approved the spec, implementation plan, and commits for each completed task.

## Delivered API

- `furnace`, `burner`, `storage`, `element`, and `element(lifecycle)` replace old declaration attributes.
- `Furnace::register` records typed providers/controllers, imports, exports, and global status.
- Rooted membership has one explicit owner; direct imports and reachable global exports govern DI access. Rust `pub` and namespaces do not grant access.
- HTTP routes and Passport strategies follow registered controllers and their furnace context.
- Official infrastructure uses explicit furnace registrations; private database helpers stay private.
- `MadsBurnExt` and `Mads::burn` preserve conventional configuration, inspection, lifecycle, binding, and shutdown.
- Scaffold templates, consumers, focused testing fixtures, examples, and migration documentation use the new API.

## Verification

Local Cargo checks use `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` to avoid the disk exhaustion encountered with full debug artifacts. Repository build profiles are unchanged. HTTP/CLI integration tests ran with loopback socket access.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Pass |
| Full locked all-feature workspace suite, stable | 730 passed, 0 failed, 7 ignored |
| Full locked all-feature workspace suite, Rust 1.94 | 730 passed, 0 failed, 7 ignored |
| `cargo test --locked --workspace --all-features --doc` | 32 passed, 0 failed, 3 ignored |
| Core-only facade, JWT-only common, persistence core/PostgreSQL feature checks | Pass |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps` | Pass |
| `cargo package --locked --offline --workspace --no-verify` | All 9 crates packaged |
| Generated scaffold compilation, inspection, HTTP smoke test | Pass |
| Standalone hello-world, posts-crud, protected-route and persistence standard_run compilation | Pass |
| Stable/MSRV macro pass and compile-fail snapshots | Pass without overwrite |

The ordinary package command could not resolve crates.io DNS; the equivalent cached offline packaging succeeded. No package was uploaded. Four live PostgreSQL tests remain ignored because `MADS_TEST_DATABASE_URL` is absent; the existing CI PostgreSQL job runs them with `--ignored`. The other three ignored results are doctest examples.

## Fresh review and fixes

A fresh read-only reviewer examined the whole implementation against the spec and plan. No critical findings were reported. Two important findings were reproduced and fixed in `aea56ab`:

1. An unregistered malformed Passport strategy invalidated unrelated roots. The regression first failed with `MADS130`, then passed after rooted metadata validation was restricted to registered strategies. Registered invalid strategies still fail; unrooted catalog validation remains complete.
2. A guard could consume a privately owned foreign `JwtService` supplied through the builder. The regression first reported a valid graph, then passed with `MADS009` after guard access was checked against explicit furnace ownership. Unowned official defaults remain ambient.

Focused regressions and existing scoped Passport/auto-configuration tests passed 15/15, followed by full stable/MSRV workspace verification.

## Rulings I made

The following preserves every ruling in execution order, including superseded setup decisions. The early concern about unavailable Git writes was resolved by approved sandbox escalation and explicit user authorization; all requested task commits were made.

1. Ruling: Work in the existing refactor/mads-declaration-module branch after sandbox denied worktree creation — worktree skill fallback — no isolated checkout, so preserve unrelated changes.
2. Ruling: Commits may be unavailable because .git is read-only; keep reviewable file changes and ledger rather than request permission for bookkeeping.
3. Ruling: Keep legacy namespace path during internal fixture migration, as plan sequencing allows; remove it from shipped rooted API in Task 8.
4. Ruling: Automated fixture migration preserves prior authored imports and public exports as an initial port, then behavioral assertions are reviewed against explicit ownership. Omitted registrations/unowned provider tests need deliberate semantic updates, not compatibility fallbacks.
5. Ruling: Full debug workspace compilation exhausted disk; removed reproducible target artifacts and use CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 for local checks. Public build configuration unchanged.
6. Ruling: Registered supplied values bypass unused static factories, preserving existing override precedence without changing ownership/export checks. A value override still requires explicit membership. Risk if wrong: duplicate metadata may be hidden when deliberately overridden; intentional compatibility behavior pinned by tests.
7. Ruling: Multiple linked factories for one output remain ambiguous under type-based registration, including custom logger factories; custom logger fixture now supplies its value to an explicitly registered global output. Risk if wrong: factory-selection API may be needed in a future change; current spec explicitly excludes it.
8. Ruling: Runnable examples now reference local workspace crate paths at the existing 0.9.2 version — registry 0.9.1 cannot provide the breaking API — cost if wrong: examples require a checkout until the new API is published.
9. Ruling: Preserve existing fixture test names and semantic wire-role labels — type naming is outside the breaking declaration vocabulary — cost if wrong: some test filenames still say module/service/provider.
10. Ruling: Tests from the plan use existing focused files plus furnace_registration/furnace_scope/furnace_http_scope/furnace_infrastructure/furnace_api instead of creating every proposed filename — preserves focused ownership and verifies behavior through the existing suites — cost if wrong: coverage mapping needs reading test names rather than matching plan filenames.
11. Final: Ruling: Supplied registered values continue to suppress unused constructor ambiguity — preserves existing explicit override precedence and is covered by tests — cost if wrong: an intentional override hides invalid unused factory metadata.
12. Final: Ruling: Preserve route-conflict validation at router finalization while checking controller registration/dependencies before construction — plan explicitly retains existing startup/route-validation sequencing; spec wording concerns the new registration graph — cost if wrong: provider constructors may run before a route-conflict error, as before this change.
13. Final: Ruling: Live PostgreSQL behavior remains for the existing CI service — no local database URL is available — cost if wrong: native connector/lifecycle behavior awaits external verification.
14. Finishing ruling: Keep the user-assigned branch and local per-task commits — user explicitly selected branch-based implementation; no push, PR, or merge requested — cost if wrong: integration remains a separate user decision.

## Deferred minors

- Facade introductory rustdoc still says unrestricted `pub` exports govern DI access (`crates/mads/src/lib.rs:6`). The implementation and migration guide correctly require explicit furnace exports. Deferred as the fresh review's minor documentation finding.

## Integration

The assigned branch and local task commits are preserved. No version bump, push, merge, PR, or publishing is included in this work.
