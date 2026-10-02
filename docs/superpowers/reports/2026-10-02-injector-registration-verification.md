# Injector registration migration verification

Date: 2026-10-02
Branch: `refactor/furnance-injector`
Status: Implemented and verified; retained on the assigned local branch.

## Delivered behavior

`Injector<T = Self>` replaces `#[element]`. `inject` is an associated asynchronous constructor returning the native output. Dependencies are unit or a typed tuple of one through sixteen cloned `Send + Sync + 'static` inputs. `lifecycle` attaches existing hooks after successful injection. Supplied outputs skip construction and hook attachment.

Cauldrons use `.provide::<Service>()` for self constructors and `.provide_with::<Arc<dyn Trait>, Implementer>()` for selected output constructors. Rooted inspection and construction use the selected descriptor, preserving ownership, imports, exports, cycle checks, diagnostics, and isolation from unrelated registrations. Manual injectors require reachable root registrations; managed macros and deliberately authored native integration metadata retain linked discovery.

Managed burner/storage/controller macros generate injectors and preserve shared handles, self-type normalization, routes, and seals. Logger, explicit JwtService, persistence factories/connectors, and native database connections use Injector construction. Examples, external aliases, compile fixtures, active documentation, and lifecycle tests are migrated. No new repository dependencies, version bump, merge, or push.

## Implementation commits

| Task | Commit | Result |
| --- | --- | --- |
| 1 | cc8fe8e | Typed async construction, tuple resolution, static descriptors |
| 2 | c7eb140 | Authoritative cauldron output bindings and explicit construction |
| 3 | 76d8f63 | Managed service/repository/controller injectors and facade exports |
| 4 | e75882b | Native integrations and lifecycle cleanup |
| 5 | 899d2b9 | Remove element; migrate consumers, examples, fixtures, docs |
| 6 | aa53420, 5475d1b | CLI snapshots, Clippy spacing, and reviewed visibility migration coverage |

Specification and execution plan were approved before implementation. Per-task RED→GREEN evidence and test commands are retained in the archived execution ledger.

## Verification

Both stable Rust 1.96.0 and MSRV Rust 1.94.0 full workspace runs after the review fix exit zero. Each run prints 138 test summaries totaling 754 passing results and four ignored database cases, including nested test runs. New compile fixtures cover three rejected private managed dependencies and three supported public dependency migrations.

All nine planned feature configurations pass. Strict Clippy, strict rustdoc, formatting, and package payload verification for all nine crates pass. Task 5 built the actual three standalone examples (hello-world, posts-crud, protected-route) and verified facade path/alias consumers and scaffold applications.

### Required gates and commands

| Gate | Command | Evidence |
| --- | --- | --- |
| Stable workspace | `cargo test --locked --offline --workspace --all-features` | Exit 0; stable-reviewed.log |
| MSRV workspace | `cargo +1.94.0 test --locked --offline --workspace --all-features` | Exit 0; msrv-reviewed.log |
| Strict Clippy | `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` | Exit 0; clippy-reviewed.log |
| Strict documentation | `RUSTDOCFLAGS='-D warnings' cargo doc --locked --offline --workspace --all-features --no-deps` | Exit 0; docs.log |
| Standalone doctests | `cargo test --locked --offline --workspace --all-features --doc` | Final task completion log; also pass in both full runs |
| Feature configurations | Nine commands in the approved plan | All exit 0; features.log |
| Formatting | `cargo fmt --all -- --check`, `git diff --check` | Exit 0 |
| Package contents | `bash script/verify-package-contents.sh` | Nine packages verified; packages-reviewed.log |
| Live PostgreSQL | Stable and MSRV: `cargo test --locked --offline -p furnace-rs-persistence --features sea-orm-postgres --test postgres -- --ignored --test-threads=1` | Four pass on each toolchain; no ignored cases in dedicated runs |
| Database recovery | `python3 -m unittest discover -s benchmark/tool -p 'test_*.py'` with dedicated DB and posts-crud application environment | 24 pass, no skips |

The database procedure follows the prior PostgreSQL report: owned temporary PostgreSQL 16 cluster, loopback port, separate test and benchmark databases, owned socket directory, and ephemeral credentials. The posts-crud schema was applied to the dedicated benchmark database. All commands exited zero; fast shutdown completed and the owned temporary cluster directory was removed. These are regression checks, not a performance benchmark claim.

Unexpected failures resolved during verification: migrated CLI fixture source lines shifted by 21; strict Clippy rejected stale attribute spacing; Rust 1.94's removed-macro E0433 wording required the existing MSRV snapshot mechanism. No runtime implementation changes were needed in the final verification stage.

## Fresh review and disposition

One read-only `gpt-6-astra` review of `54d21f5..aa53420` checked the approved spec, plan, ledger rulings, and all five Review Focus items. No Critical or Minor findings; one Important finding: public managed injectors expose field types through the associated dependency tuple, so private dependency types can fail E0446. Declined to judge: empty.

Accepted fix: the migration guide explains sufficiently visible dependency types, narrower service visibility as an alternative, and that Rust visibility grants no cauldron export. The existing migration documentation gate failed on missing E0446 guidance before the edit, then passed. UI fixtures exercise E0446 and successful migration for burner, storage, and controller. The approved tuple contract is retained. No second review was dispatched.

## Compatibility limits

- `#[element]` and its lifecycle form are removed with no alias.
- Public Injector implementations must expose sufficiently visible dependency types; private fields can remain private. Rust `pub` still does not grant DI visibility.
- Generic type names are canonical Rust names; descriptor type-name getters are no longer const because Rust 1.94 cannot evaluate generic `type_name` in constants.
- Tuple inputs require Clone, Send, Sync, and static lifetimes; constructor futures must be Send. Maximum tuple/managed dependency arity is sixteen.
- Manual injectors need rooted registration. The hidden metadata bridge exists for intentionally discoverable native integrations, not automatic discovery of arbitrary implementations.

## Rulings I made

- Task 1: Ruling: generic output/dependency names use runtime callbacks; type_name getters become non-const while existing authored macro names remain unchanged — Rust 1.94 rejects const type_name, verified locally — cost if wrong: downstream const getter callers must migrate in this breaking release.
- Task 2: Ruling: resolve manual descriptors directly from selected members during scope analysis rather than merging them into the global linked catalog — keeps unrelated roots isolated and authoritative descriptors intact — cost if wrong: integrations that bypass the selected graph may need to consume rooted metadata.
- Task 2: Ruling: rooted explicit construct<T> also selects the member injector before falling back to the catalog — regression reproduced wrong competing value 99 instead of 1, fixed to 1 — cost if wrong: rooted explicit construction now follows the authored binding instead of the linked factory.
- Task 4: Ruling: expose doc-hidden InjectorMetadata::DESCRIPTOR plus authored-name/location/visibility modifiers for deliberate integration inventory submissions — preserves native catalog/focused discovery and original inspection metadata without element — cost if wrong: hidden integration bridge must be kept compatible across the migration.
- Task 5: Ruling: preserve generic manual injector provenance by carrying registration locations into rooted graph analysis; authored managed/integration descriptor locations stay intact — RED diagnostics pointed into core/injector.rs instead of the binding — cost if wrong: diagnostic source locations would mislead manual-injector users.
- Task 5: Ruling: core macro self-resolution uses its absolute extern self alias in both the library and doctest crates — new Injector doctest reproduced crate::__private missing; the library already defines extern crate self as furnace_rs_core — cost if wrong: macros authored inside core could fail to resolve their runtime types.
- Task 5: Ruling: native JwtService supports explicit Injector<JwtService> construction from Config without a new catalog submission — typed registration requires the owned type contract, while unregistered official defaults stay conditional — cost if wrong: explicit JWT registrations could accidentally change default activation or redaction; regression suites cover these boundaries.
- Task 5: Ruling: migrate factory contract tests to local constructor structs and deliberately authored low-level inventory metadata, preserving their unrooted catalog coverage; product applications use cauldron registrations — cost if wrong: tests could exercise metadata paths different from plain rooted user services; separate plain Injector root tests cover that path.
- Task 5: Ruling: generic dependency names are canonical Rust names and UI stderr is refreshed for typed tuple diagnostics — the same input order and invalid-bound behaviors remain tested — cost if wrong: users relying on abbreviated inspection text will see expanded names.
- Task 5: Ruling: the plan's basic example refers to the existing hello-world example — verify the actual three standalone projects — cost if wrong: no fourth example is introduced or silently omitted.
- Task 6: Ruling: retain version-specific removed-element UI snapshots in existing stable/MSRV directories — Rust 1.94 and 1.96 differ only in E0433 wording — cost if wrong: future compiler wording changes require snapshot updates, not relaxed compile-fail checks.
- Final: Ruling: retain the approved tuple-associated Injector contract and document its Rust visibility consequence for public managed providers — public associated types cannot expose private dependency types; Rust pub still grants no DI export — cost if wrong: users must make dependency types public or narrow service/controller visibility during migration.

## Deferred minors

None from this review.

## Evidence archive

Execution logs and the complete ledger are preserved under `/tmp/furnace-injector-verification/`. This path is local verification evidence, not a tracked deliverable. This plan's ignored execution scratch directory is removed only after all required work is complete; sibling plan directories are preserved.
