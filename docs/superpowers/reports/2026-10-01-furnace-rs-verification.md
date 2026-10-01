# furnace-rs migration verification

Execution date: 2026-10-02. Assigned branch: `refactor/mads-declaration-module`.

Approved design: [spec](../specs/2026-10-01-furnace-rs-cauldron-controller-design.md). Execution: [plan](../plans/2026-10-01-furnace-rs-cauldron-controller.md). Migration base: `aed77c6cd265c199f6ae1a628c3dfe5b7b81f3ef`.

## Delivered behavior

Nine furnace-rs package identities; explicit cauldron registration and ownership; burner/storage/element macros; inherent controller endpoint declarations with native Axum adapters and canonical paths; static controller-wide seals; selected HTTP/security preflight before construction; schema/protocol 2; six-file CLI scaffold; updated active documentation and examples. Package versions remain 0.9.2, edition 2024, MSRV 1.94.

## Local task commits

- `eb47eff feat!: rename the framework to furnace-rs and cauldrons`
- `59ec1c8 feat(http): declare static controller seals and guard policies`
- `8fe995a feat(http): declare endpoints on controller implementations`
- `2b644c3 feat(core): preflight selected endpoints and seals before construction`
- `9110d75 feat(http): protect every controller endpoint with its declared seal`
- `ea1b124 feat!: migrate consumers to cauldrons and sealed direct controllers`
- `d3bbfa8 docs!: document furnace-rs controllers, seals, and migration`
- `e18e7d9 fix: preserve valid route branches and staged benchmark builds`

## Verification

All local Cargo commands used offline mode, workspace commands used the committed lockfile, and incremental/debug output was disabled. HTTP/CLI integration runs used authorized local loopback access.

Initial stable full suite: exit 0, aggregate printed summaries 734 passed, 0 failed, 4 ignored (includes summaries printed by nested test processes). Initial MSRV suite demonstrated seven compiler-diagnostic snapshot mismatches. The rejected programs remained invalid; inspected Rust 1.94 diagnostics require toolchain-specific expectations. Four shared fixtures now follow the existing stable/MSRV directory convention; three stale MSRV expectations were refreshed. Both affected UI suites passed after correction; final full suite results are recorded below.

Feature checks passed for core-only facade, HTTP-only facade, JWT-only common, cookies, persistence without features, and sea-orm-postgres. Normal dependency trees confirm core-only has no common/Axum/JWT/SeaORM dependencies and JWT-only common has no Axum/tower-http/SeaORM dependencies.

Separate locked workspace doctests, strict rustdoc, all three standalone examples, native persistence `standard_run`, nine-package payload checks and workspace crate archive creation passed. No unpublished-dependency registry resolution limitation was encountered. Migration guide's complete example compiled as an external aliased consumer during Task 7.

Final committed stable gate: `cargo test --workspace --all-features --no-fail-fast --offline --locked`, exit 0; aggregate printed summaries 735 passed, 0 failed, 4 ignored (includes nested summaries). Separate workspace doctests: 30 passed, 0 failed, 0 ignored. Formatting and strict full-workspace Clippy passed after the fixes. Final committed MSRV gate: `cargo +1.94.0 test --workspace --all-features --no-fail-fast --offline --locked`, exit 0; aggregate printed summaries 735 passed, 0 failed, 4 ignored. Both gates ran with snapshot overwrite disabled.

The intermediate MSRV rerun overlapped implementation edits and built the previous validator against the new regression; it is not used as final evidence. The final committed-tree MSRV run follows the stable run sequentially.

## External verification

`FURNACE_TEST_DATABASE_URL` was unavailable. Four real PostgreSQL tests were ignored; no database or toolchain was provisioned. Three optional benchmark database/recovery checks were also skipped because their database/application environment was unavailable. The existing CI `postgres` job in `.github/workflows/ci.yml` must run those tests against PostgreSQL 16. Archives were created with `--no-verify`; local source compilation and tests provide compilation evidence, while registry publication remains a separate action.

The database limitation above records the original environment. A later user-authorized [minimal PostgreSQL verification](2026-10-02-postgres-verification.md) passed all four persistence cases on stable/MSRV and all three benchmark database/recovery cases, resolving that local verification gap.

## Fresh whole-branch review

One fresh read-only reviewer, gpt-6-astra/high, was assigned the complete migration range `aed77c6..d3bbfa8`, approved spec/plan, five Review Focus cases, verification logs and rulings. Review resumed on the same agent after a temporary usage-limit error. Critical: none. Important findings: valid divergent capture branches were incorrectly rejected, and benchmark staging retained relative checkout dependencies that break in `/tmp`. Both were independently reproduced by the reviewer, reproduced again by new failing regression tests, and fixed in one pass. The direct controller regression now completes a rooted build and actual requests to both branches; existing duplicate, cross-verb and wildcard rejection tests remain green. The benchmark regression runs actual offline Cargo checks for all three staged examples, including persistence, and verifies original manifests are unchanged. Fixes and MSRV expectations are committed in `e18e7d9`. The benchmark Python suite passed 21 tests with 3 environment-dependent database cases skipped, on Python 3.12.3; its loopback fixtures required authorized socket access.

The suspected guard `cfg_attr` defect did not reproduce: Rust removes disabled declarations before expansion. Reviewer considered all five Review Focus cases. Declined to judge: none. No second reviewer was dispatched; regression tests and final suites verify the fixes.

## Rulings I made

- Ruling: Use the existing user-assigned refactor/mads-declaration-module branch — explicit user authorization supersedes a new worktree — cost if wrong: isolation from unrelated changes must be monitored.
- Task 1: Ruling: Preserve the established double-underscore nested environment mapping (FURNACE_SERVER__PORT) — spec changes the prefix and preserves configuration behavior; single underscore in the task example is a typo — cost if wrong: users expecting FURNACE_SERVER_PORT need the documented nested spelling.
- Task 3: Ruling: Validate canonical conflicts before Axum registration in Task 3; pin preconstruction builder rejection in Task 4 after its core callback boundary exists — required sequencing of the approved interfaces — cost if wrong: Task 4 must complete before shipping to prevent duplicate routes from causing constructor side effects.
- Task 4: Ruling: Memoize static seal metadata within each analysis and share it between JWT requirement evaluation and final preflight — JWT condition selection needs seal requirements before virtual defaults can be selected, so the callback may run while gathering conditions; validation still completes after virtual selection and before constructors — cost if wrong: callback timing differs from a literal after-conditions reading, while exactly-once evaluation and constructor safety remain enforced.
- Task 4: Ruling: Focused analysis evaluates official defaults against only the selected providers, excluding fixture-required supplies — dynamic controller seals introduce JWT requirements without a static DI edge, so filtering evaluators solely by declared dependencies misses them — cost if wrong: focused fixtures may report extra skipped defaults.
- Task 4: Ruling (supersedes the broad focused-default evaluation ruling): Preserve focused isolation with an explicit integration descriptor flag for metadata-based requirements; only JWT opts in — existing core tests prove evaluating unrelated descriptors is forbidden — cost if wrong: official integrations with dynamic requirements must opt in explicitly. Existing focused isolation tests RED→GREEN gate this correction.
- Task 5: Ruling: Attach the existing Passport pipeline once to each validated controller router before merging it — one static controller policy protects every endpoint uniformly, and generated typed extraction remains unchanged — cost if wrong: future per-endpoint exceptions would require changing the layer boundary; those exceptions are explicitly excluded by the approved design. Bindings retain each controller/method/canonical-path/handler/cauldron occurrence.
- Task 6: Ruling: Split the protected-route example into a public login controller and a sealed profile controller — endpoint exceptions are forbidden by the approved controller-wide seal contract — cost if wrong: applications need separate controllers for different protection policies.
- Task 6: Ruling: Carry the six-template archive assertion into consumer migration — removing routes.rs necessarily changes the package payload in this task — cost if wrong: archive tooling changes land one task earlier than the plan lists.
- Task 6: Ruling: Retire trait-only UI constraints that are valid inherent Rust methods, migrate path/receiver/extractor/policy diagnostics, and use explicit policy descriptors in metadata tests — the retired API no longer creates linked guard inventory — cost if wrong: historical trait coverage is replaced rather than preserved verbatim.
- Task 6: Ruling: Gate consumer migration with workspace lib/integration tests and defer stale rustdoc examples to Task 7 — the plan places active documentation after API removal, and common runtime tests already pass while the old Passport doctest fails — cost if wrong: doctests remain red between the two task commits.
- Task 6: Ruling: Report removal is verified by generated-consumer absence assertions and complete version-2 JSON snapshots, without claiming an observed original report-field RED — the initial acceptance run stopped at intermediate compilation errors — cost if wrong: report-field coverage lacks the planned original implementation RED evidence.
- Task 7: Ruling: Rename the active benchmark build helper to build_furnace.py and its existing test — the user requested the complete project identity change; historical benchmark results stay untouched — cost if wrong: callers of the old helper filename must follow the updated benchmark guide.
- Task 8: Ruling: Review the entire migration from aed77c6 rather than only the final verification task base d3bbfa8 — the executing-plans whole-branch review requirement and all five Review Focus items require the complete implementation — cost if wrong: the review examines a larger diff than Task 8 wording suggests.
- Task 8: Ruling: Preserve the assigned branch locally at handoff without presenting merge/push options again — the approved plan explicitly excludes merge, push and publish — cost if wrong: integration remains a separate user action.
- Task 8: Ruling: Keep compiler-specific diagnostics in stable/MSRV fixture directories using the existing UI convention — identical rejected programs produce different compiler-owned E0433 wording, spans and follow-on errors — cost if wrong: duplicated fixture sources require synchronized edits.

## Deferred minors

- Historical benchmark locks remain under `benchmark/targets/mads/locks` at 0.9.1, while the helper reads `benchmark/targets/furnace-rs/locks`. Cost: initial builds resolve fresh dependencies rather than reusing those historical pins; subsequent successful builds save new locks. Reconciling tracked benchmark inputs is deferred.
- `benchmark/COMPARISON.md` links to a renamed historical JSON filename that does not exist. Cost: that evidence link remains broken; the original `mads` JSON artifact remains preserved.

## Handoff

All eight tasks completed. Task 8 completion gate records `d3bbfa8..e18e7d9` with the full stable suite; the final sequential MSRV run confirms the same committed implementation. This report and plan status land in a separate local documentation commit.

Local commits only; preserve the assigned branch and workspace. No merge, push, publish or remote repository rename. Disposable execution artifacts are removed only after their rulings and results are preserved here.
