# Entity-bound Storage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional entity-bound storage CRUD, typed ordering, explicit relation
loading, and caller-controlled page-number pagination.

**Architecture:** Keep shared SeaORM operations in furnace-rs-persistence behind
sea-orm-storage. Extend the existing storage macro with entity/connection
selection and generate thin named relation query facades. Reuse public native
SeaORM loader APIs and the existing managed-provider DI expansion.

**Tech Stack:** Rust edition 2024, MSRV 1.94, syn/quote/proc-macro-crate, SeaORM 2,
Serde, Tokio, trybuild, PostgreSQL acceptance tests.

**Spec:** [Entity-bound storage design](../specs/2026-10-16-entity-storage-design.md)

**Status:** Draft for review alongside the spec. No implementation or execution
method has been approved. The user explicitly requested both documents before
implementation; review them together before starting Task 1.

## Global Constraints

- Use `entity()` as the generated namespace for CRUD and queries.
- Pagination is `paginate(page_number, page_size)`, with one-based page numbers.
- Pagination requires caller-authored ordering. Do not add an implicit order
  or primary-key tie breaker; applications choose their ordering columns.
- Applications choose both pagination arguments. There are no framework
  maximum sizes, maximum offsets, clamping, defaults, or TOML pagination settings.
- Fetch at most the requested number of parent records. Page results contain
  items, page_number, and page_size; no has_next, lookahead, or total counts.
- Relation-enabled storage uses SeaORM 2's native loader-compatible
  `#[sea_orm::model]` or `#[sea_orm::compact_model]` entity format.
- No generated transaction accessor, transaction creation, commit, rollback,
  or transaction propagation. Applications use native SeaORM for transactions.
- Preserve existing workspace version, edition 2024, Rust 1.94 MSRV,
  unsafe-code prohibition, and public-documentation lint.
- Do not introduce a SeaORM runtime dependency into core or core-macros.
- Resolve renamed persistence dependencies; preserve bare storage and burner.
- Do not update SeaORM versions or unrelated lockfile entries. Task 1 may
  refresh the root lockfile's local package dependency edges for optional Serde.
  Consumer fixture lockfiles are generated only if the existing fixture
  convention requires tracking them.
- All new Rust source/test crates start with crate documentation before
  conditional crate attributes. Generated public items receive documentation.
- Source of truth for implementation is the spec; stop and ask the user if a
  new public-contract ambiguity is found.

## Review Focus

- Page 2 with relations must use OFFSET 20 and LIMIT 20 for size 20, with no
  extra parent or COUNT query.
  Covered by Task 5's relation pagination SQL test and Task 6's live test.
- Two foreign keys to the same entity must load the selected relationship,
  including selection of both. Covered by Task 5 author/editor fixtures.
- A connection alias and an unrelated type named DatabaseConnection must
  not be silently mis-selected. Covered by Task 4 UI fixtures.
- A missing composite-key component or an update with no changed columns
  must follow native fallible behavior without a helper panic or preflight
  query. Covered by Task 2's update tests.
- Huge but representable pagination requests must not acquire policy caps,
  preallocate their requested capacity, or issue SQL after arithmetic failure.
  Missing explicit ordering must also fail before SQL without adding a default.
  Covered by Tasks 1 and 3.

---

## File structure

Create under crates/furnace-rs-persistence/src/storage/:

- mod.rs: public exports and doc-hidden macro support re-exports.
- error.rs: StorageError, StorageErrorKind, StorageResult.
- pagination.rs: Page, validated PageWindow, envelope finishing.
- entity.rs: EntityStore CRUD.
- query.rs: EntityQuery, explicit ordering, prepared page/one support.

Create under crates/furnace-rs-core-macros/src/:

- storage.rs: declaration parser, connection selection, ordinary binding.
- storage/relations.rs: generated relation accessor/query facades.

Modify core-macros/src/managed.rs, lib.rs, and path.rs without changing the
existing handle/Injector/descriptor contract.

Create persistence tests storage_pagination.rs, storage_entity.rs,
storage_query.rs, storage_ui.rs, storage_relations.rs, storage_consumers.rs,
storage_postgres.rs, and support/storage_entities.rs. Add UI fixtures under
tests/ui/storage/{pass,fail}, using the repository's toolchain-specific
snapshot convention when native compiler messages vary. Add independent
consumer manifests under tests/consumers/storage_*.
Each independent consumer has its own [workspace] boundary so its feature
selection is checked independently of the parent workspace.

Create docs/storage.md. Update persistence README, docs/ARCHITECTURE.md,
and CHANGELOG.md only as the feature is implemented.

### Task 1: Optional storage boundary, errors, and page arithmetic

**Files:** persistence Cargo.toml, src/lib.rs, src/storage/{mod,error,pagination}.rs,
Cargo.lock, tests/storage_pagination.rs.

**Interfaces produced:**

- Feature-gated public module storage and optional feature sea-orm-storage.
- StorageErrorKind::{InvalidPagination, MissingOrder, NotFound, Database}.
- StorageError::kind() -> StorageErrorKind and standard Error::source().
- From<sea_orm::DbErr> for StorageError maps to Database and retains its source.
- StorageResult<T> = Result<T, StorageError>.
- Page<T> with exactly items, page_number, and page_size from the spec.
- Doc-hidden PageWindow::new(page_number: u64, page_size: u64)
  -> StorageResult<PageWindow>, plus page_number(), page_size(), offset(),
  and fetch_limit() -> u64 accessors.
- PageWindow::finish<T>(self, items: Vec<T>) -> Page<T>.
- Crate-private StorageError constructors for invalid pagination, missing
  ordering, database sources, and missing-update sources. Formatting always
  redacts source text.

- [ ] **Step 1: Write arithmetic and envelope tests.** Assert window(1,20)
  has offset 0 and fetch_limit 20; window(2,20) has offset 20;
  window(2,1_000_000) is accepted. Zero arguments, multiplication overflow,
  such as window(3,u64::MAX), return InvalidPagination. Windows (1,u64::MAX)
  and (2,u64::MAX) are accepted by the arithmetic layer. Finish 20 returned
  items unchanged; finish empty items into an empty page with the requested
  number/size. Check that serialization has exactly the three specified
  envelope fields and no navigation/count fields.
  Assert Page<NonSerializable> can be constructed and serializable models
  produce the documented JSON field names.
- [ ] **Step 2: Run the failing tests.**
  Run: cargo test --offline -p furnace-rs-persistence --features sea-orm-storage --test storage_pagination
  Expected: feature/types absent.
- [ ] **Step 3: Implement the feature, errors, page math, and result envelopes.**
  Enable optional SeaORM/macros and Serde only in sea-orm-storage, independently
  of the driver feature. Export a doc-hidden SeaORM path for generated code.
  Export a doc-hidden __furnace_storage_enabled! item-forwarding macro in both
  feature states; the disabled implementation emits a focused compile error
  naming sea-orm-storage, while the enabled implementation emits its input.
  Use page_size directly as fetch_limit. Finish by wrapping the returned
  vector unchanged; never allocate according to a requested capacity.
  Add redaction tests retaining a synthetic secret-bearing DbErr through source().
- [ ] **Step 4: Run tests and boundary checks.**
  Run the Task 1 command again, then cargo check --locked --offline
  -p furnace-rs-persistence --no-default-features and the same command with
  --features sea-orm-storage. Expected: success without selecting a driver.
- [ ] **Step 5: Commit.**
  Commit message: feat(persistence): add optional storage pagination contracts

### Task 2: Native entity CRUD

**Files:** src/storage/entity.rs, src/storage/mod.rs,
tests/storage_entity.rs, tests/support/storage_entities.rs.

**Consumes:** StorageResult and error constructors from Task 1.

**Interfaces produced:**

~~~rust
pub struct EntityStore<'db, E: EntityTrait> { /* borrowed connection */ }
impl<'db, E: EntityTrait> EntityStore<'db, E> {
    pub fn new(database: &'db DatabaseConnection) -> Self;
    pub async fn find_by_id<K>(&self, key: K) -> StorageResult<Option<E::Model>>
    where K: Into<<E::PrimaryKey as PrimaryKeyTrait>::ValueType>;
    pub async fn insert(&self, model: E::ActiveModel) -> StorageResult<E::Model>;
    pub async fn update(&self, model: E::ActiveModel) -> StorageResult<E::Model>;
    pub async fn delete_by_id<K>(&self, key: K) -> StorageResult<bool>
    where K: Into<<E::PrimaryKey as PrimaryKeyTrait>::ValueType>;
}
~~~

Apply the native IntoActiveModel bounds required by insert/update; do not
erase types or use dynamic dispatch.

- [ ] **Step 1: Write MockDatabase CRUD tests.** Cover found/missing lookup,
  generated insert ID, Set versus Unchanged/NotSet on update, missing-update
  mapping to NotFound, delete with zero/one affected rows, composite-key SQL
  predicates, before/after-save hook counters, and redacted database failure.
  Include an unset composite-key component and a model with no changed
  fields; assert native errors/results and logged query counts rather than
  inventing a helper SELECT.
- [ ] **Step 2: Run cargo test --locked --offline -p furnace-rs-persistence
  --features sea-orm-storage --test storage_entity.** Expected: missing CRUD.
- [ ] **Step 3: Implement CRUD through native ActiveModel operations and
  Entity find/delete operations.** Map only RecordNotUpdated to NotFound,
  retaining its source. Preserve other DbErr variants as Database.
- [ ] **Step 4: Repeat Task 2 tests.** Expected: all assertions pass.
- [ ] **Step 5: Commit.**
  Commit message: feat(persistence): add typed entity storage operations

### Task 3: Filtering, ordering, and root pagination

**Files:** src/storage/query.rs, src/storage/entity.rs, src/storage/mod.rs,
tests/storage_query.rs, tests/storage_pagination.rs.

**Consumes:** EntityStore and PageWindow.

**Interfaces produced:**

- EntityStore::query(&self) -> EntityQuery<'db, E>; the result borrows the
  database lifetime, not the temporary helper.
- EntityQuery::filter<C: IntoCondition>(self, condition: C) -> Self.
- EntityQuery::order_by_asc(self, column: E::Column) -> Self and order_by_desc.
- EntityQuery::one(self) -> async StorageResult<Option<E::Model>>.
- EntityQuery::paginate(self, page_number: u64, page_size: u64)
  -> async StorageResult<Page<E::Model>>.
- Doc-hidden PreparedPage<'db, E> returned by EntityQuery::prepare_page(self,
  page_number, page_size) -> StorageResult<PreparedPage<'db, E>>.
- PreparedPage::database(&self) -> &'db DatabaseConnection;
  statement(&self) -> SelectStatement; finish<T>(self, Vec<T>) -> Page<T>.
- Doc-hidden EntityQuery::prepare_one(self) -> (&'db DatabaseConnection,
  SelectStatement ), preserving authored ordering and applying LIMIT 1.

The prepared page contains the bounded root selection with authored ordering,
calculated offset, and LIMIT page_size. No count query is retained. Generated
code uses the doc-hidden interfaces without reaching into private fields.

- [ ] **Step 1: Write query SQL/result tests.** Assert AND/OR filters survive
  pagination; pagination without authored ordering returns MissingOrder with
  an empty query log. Explicit ID DESC is retained; title DESC stays the sole
  criterion unless the caller adds a tie breaker. Composite-key criteria
  retain exactly the authored sequence and direction. Assert paginate(2,20)
  emits OFFSET 20 LIMIT 20, loads no lookahead root, logs no COUNT, and
  beyond-end pages remain empty. Invalid arithmetic leaves the query log
  empty and takes precedence over MissingOrder when both apply.
  A large valid size appears unchanged as LIMIT in SQL; page_size=u64::MAX
  is accepted by preparation when offset arithmetic succeeds. Unordered
  one() emits LIMIT 1 without adding ORDER BY or returning MissingOrder.
- [ ] **Step 2: Run cargo test --locked --offline -p furnace-rs-persistence
  --features sea-orm-storage --test storage_query.** Expected: missing builder.
- [ ] **Step 3: Implement the builder and prepared interfaces.** Forward typed
  filters and ordering to native Select, tracking only whether an order was
  supplied. Validate the window first and require ordering before pagination
  executes. Apply the calculated offset and LIMIT page_size without query
  rewriting, implicit primary-key criteria, count queries, or result trimming.
  Make one() preserve optional authored ordering and apply LIMIT 1.
- [ ] **Step 4: Repeat Tasks 1–3 tests.** Expected: pass, including a chained
  EntityStore::new(&database).query() lifetime fixture that stores the returned page
  query in a local variable before awaiting it.
- [ ] **Step 5: Commit.**
  Commit message: feat(persistence): add ordered storage queries and pagination

### Task 4: Storage macro binding and optional connections

**Files:** core-macros/src/{lib,managed,path,storage}.rs,
core-macros/tests/support/{managed,path}.rs, persistence/tests/storage_ui.rs,
tests/ui/storage/pass/*.rs, tests/ui/storage/fail/*.rs and expected snapshots.

**Consumes:** EntityStore from Task 2 and EntityQuery from Task 3.

**Interfaces produced:** `#[storage(entity = E, connection = field)]`,
optional automatic field selection, and documented `entity()` access.
The parser also defines relation alias/selector AST entries for Task 5.

- [ ] **Step 1: Write parser and UI tests.** Passing cases include one injected
  unqualified connection, one qualified connection, explicit alias selection,
  a core import, and a facade import. Failing cases include missing entity,
  unknown/duplicate arguments, zero/multiple candidates, missing selected
  field, selected wrong type, unrelated type named DatabaseConnection,
  alias without explicit selection, missing inject, entity-bound unit struct,
  keyless entity, and storage arguments on burner. Preserve bare storage tests.
- [ ] **Step 2: Run cargo test --locked --offline -p furnace-rs-core-macros
  and cargo test --locked --offline -p furnace-rs-persistence
  --features sea-orm-storage --test storage_ui.**
  Expected: new option acceptance cases fail before implementation.
- [ ] **Step 3: Implement storage-only parsing and generation.** Keep common
  managed expansion responsible for the Arc handle, Injector and descriptor.
  Resolve the direct persistence dependency with proc-macro-crate; select
  qualified/unqualified native connection names syntactically, then type-check
  the selected borrowed field. Aliases work through explicit selection.
  Scope generated support names and document entity(). Missing direct
  persistence dependency or disabled sea-orm-storage must identify the
  dependency/feature the caller needs, not emit a proc-macro panic.
  Wrap feature-dependent output with __furnace_storage_enabled!, resolved
  through the persistence dependency; do not inspect consuming-crate features.
  Include a repository.entity().query() fixture stored before its final await.
- [ ] **Step 4: Repeat tests and existing core macro consumer tests.**
  Expected: supported shapes and existing snapshots remain compatible;
  diagnostics point to the new argument/field. Store native trait diagnostic
  variants under the established MSRV/stable fixture layout where necessary.
- [ ] **Step 5: Commit.**
  Commit message: feat(macros): bind storage to optional entity helpers

### Task 5: Declared native relation loaders

**Files:** core-macros/src/storage/relations.rs, core-macros/src/storage.rs,
persistence/tests/storage_relations.rs, tests/support/storage_entities.rs,
tests/ui/storage/{pass,fail} relation fixtures.

**Consumes:** Parsed aliases/selectors, EntityStore CRUD, EntityQuery prepared
interfaces, Page, StorageResult.

**Interfaces produced:** relation-enabled storage facades with matching basic
CRUD signatures and a generated query wrapper containing:
filter, order_by_asc, order_by_desc, with_<alias>, one,
paginate(page_number, page_size).
Query terminals return E::ModelEx, including when no relation is selected.

- [ ] **Step 1: Write loader fixtures and tests.** Define dense-format entities
  for posts, author/editor users, comments, tags and a post-tag junction.
  Assert author and editor selectors distinguish two FKs to users, both
  selectors can be selected together, repeated with_author() is idempotent,
  undeclared with_* methods fail compilation, invalid selectors fail their
  native loader bounds, and a legacy entity used with relations fails.
  Exercise one-to-many and many-to-many batch results, loaded/unloaded
  native wrappers, a query with no selected relations, duplicate aliases,
  two storage declarations for the same entity, and qualified equal-final-name
  storages in different modules.
- [ ] **Step 2: Run cargo test --locked --offline -p furnace-rs-persistence
  --features sea-orm-storage --test storage_relations and storage_ui.**
  Expected: named loader methods absent.
- [ ] **Step 3: Generate the relation facade and query wrapper.** Keep one
  boolean per alias and delegate root filters/order to EntityQuery. At each
  terminal, construct E::load() with inferred native loader type, apply
  selected native with(selector) calls, replace its public QueryFilter
  statement with the prepared root statement, and call public all()/one().
  Use a generated named query wrapper so selected aliases need no reflection
  or guessed EntityLoader sibling type. Keep helper types scoped, documented,
  and visible sufficiently for the storage's public return types.
- [ ] **Step 4: Add and run page-boundary regressions.** For size 20/page 2,
  assert root OFFSET 20 LIMIT 20 before native relation loading. For size 2,
  a parent with three children must still occupy one root slot. Verify
  exactly the selected parents' children are loaded, with no extra parent,
  result trimming, or COUNT. Missing explicit ordering must return MissingOrder
  before any parent or relation query. A native selector that silently
  fails to activate a loader field is a failing test, not accepted support.
- [ ] **Step 5: Commit.**
  Commit message: feat(storage): generate explicit native relation loaders

### Task 6: Consumer compatibility and real database acceptance

**Files:** persistence/tests/storage_consumers.rs,
tests/consumers/storage_{core,facade,renamed,disabled}/Cargo.toml and src/main.rs,
tests/storage_postgres.rs, .github/workflows/ci.yml.

**Consumes:** Completed storage public APIs.

**Interfaces produced:** independent Cargo consumer coverage, documented
PostgreSQL acceptance command, and CI feature-boundary checks.

- [ ] **Step 1: Add consumer fixtures.** Check direct core plus persistence,
  facade plus persistence, renamed core/facade/persistence packages, missing
  persistence dependency, and disabled storage feature independently of the
  main workspace's all-features unification. Use missing-docs deny in positive
  generated-code consumers; retain Rust 1.94-compatible fixture syntax.
- [ ] **Step 2: Run the consumer harness.** Each positive cargo check must
  succeed; each negative check must fail with its targeted message. Do not
  accept unrelated dependency or compilation failure as a negative pass.
- [ ] **Step 3: Add PostgreSQL tests gated like existing persistence tests.**
  Use FURNACE_TEST_DATABASE_URL and isolated test-owned tables/schema with
  cleanup. Verify tied sort values/composite PK ordering, pages 1 and 2,
  collection fan-out, author/editor distinction, many-to-many loading,
  empty/beyond-end pages, exact root limits, absence of implicit ordering,
  update/delete missing records, and hook behavior.
  Application-owned transaction examples use the transaction handle directly;
  generated CRUD does not implicitly join it.
- [ ] **Step 4: Run targeted acceptance and feature checks.**
  Run: cargo test --locked --offline -p furnace-rs-persistence
  --features sea-orm-storage,sea-orm-postgres --test storage_postgres -- --ignored
  when a test database is available. If unavailable, report it explicitly and
  do not label real database acceptance as passed.
  Check no-default-features, storage-only, postgres-only, and both features.
  Add those compile checks to CI. Check Rust 1.94 with an installed MSRV
  toolchain, or report its unavailability without silently claiming coverage.
  Check native loader consumers against workspace SeaORM 2.0.3 and an isolated
  SeaORM 2.0.4 consumer without changing the root lockfile version.
- [ ] **Step 5: Commit.**
  Commit message: test(storage): cover consumers features and native database behavior

### Task 7: Documentation and integrated verification

**Files:** docs/storage.md, crates/furnace-rs-persistence/README.md,
docs/ARCHITECTURE.md, CHANGELOG.md, relevant crate API documentation.

**Consumes:** Verified signatures and behavior from Tasks 1–6.

- [ ] **Step 1: Document direct dependencies/features and a complete example.**
  Include inferred/explicit alias connections, every CRUD result shape,
  caller-authored ascending/descending criteria and unique tie breakers,
  page-number arguments, three-field JSON envelopes, MissingOrder failures,
  relation ModelEx requirements, and absence of application caps. State that
  pagination adds no lookahead read or count and generates no count method.
  Show domain/DTO mapping as application code. Explain that generated helpers
  using DatabaseConnection do not enlist in separately created transactions.
  Explain native delete-by-id versus ActiveModel hooks, child collection size,
  concurrency limits of offset pages, and error redaction.
- [ ] **Step 2: Add compiled documentation examples.** Reuse test entity
  fixtures for runtime behavior, and make public code examples compile with
  feature requirements stated. Add the implemented feature to Unreleased.
- [ ] **Step 3: Run affected suites and required checks.**
  Run cargo fmt --all --check; cargo test --locked --offline
  -p furnace-rs-core-macros; cargo test --locked --offline
  -p furnace-rs-persistence --all-features; and cargo clippy --locked --offline
  -p furnace-rs-core-macros -p furnace-rs-persistence --all-targets --all-features
  -- -D warnings. Run cargo test --locked --offline -p furnace-rs-persistence
  --all-features --doc and build its docs with RUSTDOCFLAGS="-D warnings".
  Repeat independent consumer feature checks after final edits.
- [ ] **Step 4: Run integration checks after scoped checks pass.**
  Run cargo test --locked --offline --workspace --all-features and workspace
  all-target/all-feature Clippy. Run only relevant previously passing checks
  again when later changes invalidate them. Record commands, versions, skipped
  database/MSRV checks, and failures without overstating results. Test outcomes
  and generated SQL counts must support any performance claim; none is promised
  by this design.
- [ ] **Step 5: Review the final diff against every spec acceptance criterion.**
  Confirm only planned features and lockfile edges changed, no transactions or
  pagination caps were introduced, and generated public APIs satisfy docs lint.
- [ ] **Step 6: Commit.**
  Commit message: docs(storage): document entity repository contracts

## Review and execution handoff

Review this plan together with the draft spec before product implementation.
Suggested execution is native (the implementing agent works through tasks)
because parser generation, shared prepared-query interfaces, and native loader
integration depend closely on each other. No subagent is spawned by this plan.

Once the user approves the documents and chooses an execution method, use
superpowers:executing-plans for native execution or
superpowers:subagent-driven-development for delegated execution. Establish the
approved implementation branch/worktree at that time; this documentation task
does not switch branches or execute the plan.
