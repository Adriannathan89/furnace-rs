# Entity-bound storage design

Status: draft for user review. This document authorizes no product implementation.
Date: 2026-10-16.
Workspace observed: branch `develop`, version `2.0.0-beta.2`.

## Intent and confirmed decisions

Reduce repetitive SeaORM repository code through an optional entity-bound
`#[storage]` contract. The macro binds an entity, an injected database
connection, and explicitly declared relation loaders. Applications retain
custom repository methods, DTO conversion, validation, authorization, and
transaction boundaries.

Confirmed requirements:

- Use `entity()` as the generated namespace for CRUD and queries.
- Support typed filtering and ascending/descending ordering.
- Declare entity and relation availability in the macro.
- Pagination is `paginate(page_number, page_size)`, with one-based page numbers.
- Applications choose both pagination arguments. There are no framework
  maximum sizes, maximum offsets, clamping, defaults, or TOML pagination settings.
- `connection = field_name` is optional. Automatic selection recognizes one
  field written as `DatabaseConnection`, including qualified paths; aliases
  and ambiguous declarations use explicit selection.
- Relation-enabled storage uses SeaORM 2's native loader-compatible
  `#[sea_orm::model]` or `#[sea_orm::compact_model]` entity format.
- No generated transaction accessor, transaction creation, commit, rollback,
  or transaction propagation. Applications use native SeaORM for transactions.

Technical details below are proposed for review as part of this draft. They
make the confirmed contract precise rather than representing implemented APIs.

## Existing architecture

The core macro in crates/furnace-rs-core-macros/src/managed.rs currently rejects
all storage arguments and generates an Arc-backed handle, an Injector
implementation, and a static provider descriptor. Every field requires
parameterless `#[inject]`. The managed-provider shape and DI behavior must
remain intact.

The persistence crate has no default database backend. Its existing
`sea-orm-postgres` feature supplies the native connection and lifecycle
integration. The facade's existing `sea-orm` feature converts native DbErr
to redacted HTTP errors; it does not enable a persistence repository helper.

The workspace lockfile currently resolves SeaORM 2.0.3, while the independent
posts-crud example pins 2.0.4. The design uses public APIs present in the
workspace version and must be checked with both resolved versions where
consumers select them. Planning does not change dependency versions or locks.

The existing PostRepository performs application-specific input construction
and Model-to-Post conversion. A generic helper cannot infer these mappings.

## Optional package boundary

Add feature `sea-orm-storage` to furnace-rs-persistence. It enables its optional
SeaORM dependency, SeaORM macros, and an optional workspace Serde dependency
for serializable page envelopes. It does not itself select a SQL driver,
runtime, connection pool, or HTTP integration.

Expose `furnace_rs_persistence::storage` only under that feature. Applications
normally enable both `sea-orm-storage` and `sea-orm-postgres`.

Do not add SeaORM or persistence runtime dependencies to furnace-rs-core or
its macro crate. The macro resolves the consumer's direct
furnace-rs-persistence dependency through proc-macro-crate, including Cargo
dependency renaming. Existing direct-core and facade macro imports both work.
Consumers need a direct persistence dependency; no new facade feature or
facade re-export is introduced by this change.

The persistence crate exports a doc-hidden feature-guard macro in both feature
states. The disabled version emits a focused instruction to enable
`sea-orm-storage`; the enabled version expands the generated storage items.
The core macro must not test a consumer feature with a guessed cfg name.

Plain `#[storage]`, including supported unit structs, remains available
without persistence. The new options apply only to `#[storage]`, never
`#[burner]`. Existing workspace version, edition 2024, Rust 1.94 MSRV,
unsafe-code prohibition, and public-documentation lint remain unchanged.

## Declaration grammar and connection selection

~~~rust
#[storage(
    entity = post::Entity,
    relations(
        author = user::Entity,
        comments = comment::Entity,
    ),
)]
pub struct PostRepository {
    #[inject]
    database: DatabaseConnection,
}
~~~

Supported top-level options are `entity = TypePath`,
`connection = field_ident`, and `relations(alias = SelectorPath, ...)`.
Each option can appear at most once. Trailing commas are accepted.
`entity` is mandatory whenever storage arguments are present. Relations are
optional and require at least one entry when the relations group is supplied.
Unknown options, duplicate options, and duplicate relation aliases fail
compilation at their declaration spans.

Without explicit connection selection, inspect named injected fields whose
type is a non-generic Rust path ending in `DatabaseConnection`. Exactly one
candidate is selected. Zero or multiple candidates produce a diagnostic
asking for `connection = field_name`.

This is syntactic selection, followed by a compiler check against the actual
native SeaORM DatabaseConnection. A type that merely has the same name cannot
pass the type check. A qualified path and an imported unrenamed
DatabaseConnection work. A renamed import or type alias such as `Db`
requires explicit selection; an explicitly selected alias to the native type
works. Wrappers and DatabaseTransaction fields are not accepted connections.

An explicit field must exist, remain marked `#[inject]`, and have the required
native connection type. References to it borrow the existing managed value;
the helper does not clone the connection, create a new provider, or open a
connection. Entity-bound unit structs are rejected because no connection is
available.

## Generated namespace and basic CRUD

~~~rust
repository.entity().find_by_id(id).await?;
repository.entity().insert(active_model).await?;
repository.entity().update(active_model).await?;
repository.entity().delete_by_id(id).await?;
~~~

The shared helper is `EntityStore<'db, E>`, borrowing DatabaseConnection.
A storage without relations returns this helper directly. A storage with
relations returns a documented, generated storage-specific facade with the
same CRUD signatures and a relation-aware query factory.

| Method | Input | Result inside StorageResult |
| --- | --- | --- |
| find_by_id | Into the entity primary-key ValueType | Option<E::Model> |
| insert | E::ActiveModel | E::Model |
| update | E::ActiveModel | E::Model |
| delete_by_id | Into the entity primary-key ValueType | bool |

Use native ActiveModel insert/update entry points so their hooks retain native
behavior. Explicit `NotSet`, `Set`, and `Unchanged` field semantics come
from SeaORM. An update is not an upsert and adds no preflight SELECT merely
to test existence. A missing update record maps native RecordNotUpdated to
StorageErrorKind::NotFound, retaining the original error as its source.
Other native failures, including an unset update primary key, remain Database
errors. Native hooks can themselves perform application-defined operations.

Deletion uses Entity::delete_by_id and reports whether rows_affected is nonzero.
It does not instantiate a model or run ActiveModel deletion hooks.
No cascade-write behavior or write access to relations is inferred.

All primary-key parts are supported in the entity-declared tuple order.
Entity-bound storage requires a nonempty primary key; enforce it through
compiler trait bounds on PrimaryKey::ValueType/PrimaryKeyArity.

Output remains native SeaORM data. There is no generated DTO mapping, input
deserialization, application validation, or automatic HTTP status mapping.

## Query construction and ordering

~~~rust
repository.entity().query()
    .filter(post::Column::Published.eq(true))
    .order_by_desc(post::Column::CreatedAt)
    .order_by_asc(post::Column::Title)
    .paginate(1, 20)
    .await?;
~~~

`query()` creates an owned builder borrowing the storage's connection.
`filter<C: IntoCondition>(self, condition: C) -> Self` uses native conditions;
repeated filters combine with AND. Applications express OR through
SeaORM Condition. Filters here do not introduce relation joins or alter
root identity.

`order_by_asc(self, column: E::Column) -> Self` and
`order_by_desc(self, column: E::Column) -> Self` take root entity columns.
Calls append criteria in call order; repeated columns are retained with
normal SQL ORDER BY precedence.

Before execution, append every missing primary-key column ascending.
Without authored ordering, this produces primary-key ascending ordering.
A primary key already present retains its authored position and direction.
Track typed root columns and compare their stable IdenStatic identifiers;
do not compare rendered SQL strings or require an extra Column: Eq bound.
The same final ordering applies to ordinary and relation-aware reads.
SQL NULL ordering remains the backend's behavior.

`one().await` returns the first matching root, or None, using that ordering.
The generated collection terminal methods are the two pagination methods
below. Applications can write native SeaORM queries for unbounded reads,
custom joins, aggregates, related-column sorting, or projections.

## Pagination API, arithmetic, and results

~~~rust
query.paginate(page_number, page_size).await?;
query.paginate_with_total(page_number, page_size).await?;
~~~

Both arguments are required u64 values. There are no hidden policy limits.
Reject zero page_number or page_size as InvalidPagination before any SQL.
Compute offset as `(page_number - 1).checked_mul(page_size)`.
Compute lookahead limit as `page_size.checked_add(1)`.
Arithmetic overflow is InvalidPagination, a representational failure rather
than an application size policy. Do not preallocate a vector using the
requested page size.

Ordinary pagination fetches at most page_size + 1 root records at the calculated
offset, removes the extra root if present, and reports has_next. Relations
are loaded for that bounded root selection, so the lookahead root may also
have its relations loaded. There is no COUNT query.

~~~rust
#[derive(Clone, Debug, serde::Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page_number: u64,
    pub page_size: u64,
    pub has_next: bool,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CountedPage<T> {
    pub items: Vec<T>,
    pub page_number: u64,
    pub page_size: u64,
    pub has_next: bool,
    pub total_items: u64,
    pub total_pages: u64,
}
~~~

Derives impose corresponding bounds only when used. Serializing Page<T>
requires T: Serialize; declaring or using the page otherwise does not.

`paginate_with_total` validates arguments first, then counts matching roots
from the unpaginated filter query and performs the same lookahead read.
Relation loading does not multiply the root count. Compute total_pages by
division plus a nonzero-remainder increment, avoiding ceiling-add overflow.
An empty filtered set has total_pages = 0. A page beyond the end has empty
items and has_next = false while retaining its requested page_number and size.

The count and data are separate statements. There is no snapshot consistency
guarantee, implicit transaction, or promise that offset pages remain unchanged
during concurrent writes. A collection relation can still be large even
when its parent page is small; the application controls its data policy.

## Relation declaration and result types

~~~rust
relations(
    author = user::Entity,
    comments = comment::Entity,
)
~~~

Each alias generates `with_<alias>(self) -> Self` on that storage's query
builder. Repeated selection is idempotent. These methods only set flags;
they do not query the database.

At execution, construct `E::load()` and pass each selected selector to its
native `with(...)`. This uses native join/batch strategies and preserves
SeaORM's relation wrapper semantics, including unloaded versus loaded state.

For relations with multiple paths to the same entity, allow the native
root-relation selector:

~~~rust
relations(
    author = post::Relation::Author,
    editor = post::Relation::Editor,
)
~~~

These paths must be supported by the root entity's generated native loader.
For an ordinary unambiguous relation, use the target entity selector. Explicit
root-relation variants are for the native loader's ambiguous/self-relation
cases; they are not a universal replacement for target entity selectors.
The alias is the Furnace method name; it is not a guessed database relationship.
Entity selector paths use an entity type constructor (e.g. user::Entity);
explicit root relation selectors use a generated variant. No string relation
lookup, automatic recursive traversal, nested-path DSL, or guessed path
selection is added. Compiler checks validate native loader parameter support.
A native selector that type-checks but does not activate a generated load
field must not be advertised as supported; fixtures must establish the
accepted entity and explicit-path cases.

Storage declaring relations always returns E::ModelEx from query terminals,
even when no with_* method is selected. Basic CRUD continues returning
E::Model. Storage without relations uses E::Model for query terminals and
supports existing DeriveEntityModel entities.

Named methods restrict the generated convenience API; they do not implement
authorization or prevent application code from issuing native queries.
No claims about row-level security are inferred from relation metadata.

## Implementation mechanism

Separate shared algorithms from generated named relation methods:

- Shared EntityStore and EntityQuery implement basic CRUD, typed predicates,
  ordering, arithmetic, lookahead envelopes, and total counting.
- The macro emits thin storage-specific facades only when relations exist.
  A relation query holds the shared EntityQuery and one boolean per alias.
- Prepare the root selection with final ordering, offset, and lookahead limit.
  Construct the native loader through E::load(), configure selected relations,
  and transfer the prepared root SelectStatement through public QueryFilter.
  Execute its public all()/one() methods with the borrowed connection.
- Do not name a generated EntityLoader sibling by modifying a Rust path.
  Do not rely on SeaORM's doc-hidden fetch(db, page, size) entry point.
  Calling the native paginator with page_size + 1 would change later page
  offsets; the prepared statement must carry the separately calculated offset.
- The unpaginated count query retains the same root filters and excludes
  relation fan-out and lookahead bounds.
- All execution uses static dispatch. No dynamic entity registry, schema
  introspection, startup SQL, synchronization lock, or connection clone is
  required by the helper.

Generated helper names and implementations must remain scoped to their
storage declaration. Public generated accessors, query facades, and named
loader methods have documentation and satisfy deny(missing_docs).
Malformed declarations return diagnostics rather than proc-macro panics.

## Errors and application integration

Expose a separate StorageError, StorageErrorKind, and StorageResult<T>.
Kinds are InvalidPagination, NotFound, and Database.
StorageError::kind() exposes the category. Standard Error::source() retains
the native database cause where present. Display and Debug omit SQL,
parameters, credentials, and retained cause text.
Implement From<sea_orm::DbErr> for StorageError as Database; the update helper
performs the explicit NotFound mapping described above.

The helper does not map errors to HTTP or gRPC. Applications decide whether
an absent lookup or missing update becomes a 404, another response, or a
domain error. Existing PersistenceError and FURNACE140 connector behavior
remain separate. No new framework diagnostic code is required for this API.

Transactions are written using native SeaORM by the application. Generated
helpers borrow the injected DatabaseConnection; calling them while an
application transaction exists does not enlist them in that transaction.
Documentation must show application transaction code using its transaction
handle directly, avoiding an example that accidentally mixes handles.

## File responsibilities

- crates/furnace-rs-persistence/src/storage/mod.rs: public feature-gated exports.
- storage/error.rs: typed redacted operation errors.
- storage/pagination.rs: validated page windows and serializable results.
- storage/entity.rs: EntityStore basic operations.
- storage/query.rs: root EntityQuery and prepared reads used by generated code.
- crates/furnace-rs-core-macros/src/storage.rs: storage options and validation.
- core-macros/src/storage/relations.rs: named relation facade/query generation.
- core-macros/src/path.rs: renamed persistence dependency resolution.
- core-macros/src/managed.rs: preserve common managed expansion and attach
  storage-only output without changing burner behavior.
- persistence integration tests and UI fixtures: behavior and compiler contracts.
- docs/storage.md and persistence README: usage, defaults, ownership, and
  transaction boundaries. CHANGELOG.md records the feature only when implemented.

## Acceptance criteria

1. Existing bare storage/burner macro consumers remain compatible without
   enabling a database feature.
2. Direct-core, facade, and renamed dependency consumers compile.
3. Connection inference and explicit aliases follow the declaration rules;
   invalid cases produce focused diagnostics.
4. Basic operations preserve native input/output and hook behavior. Composite
   keys, missing rows, unset update keys, and database errors are tested.
5. ASC/DESC, filters, composite-key tie breakers, and one-based page offsets
   produce the expected SQL and root records.
6. Zero values and arithmetic overflow cause no database query; large valid
   requests are not capped. No COUNT occurs in ordinary pagination.
7. Counted pages apply the same root filters; zero totals and pages beyond
   the end retain the documented envelope behavior.
8. Native relation results distinguish author/editor correctly, collection
   loading does not duplicate or truncate parents, and page 2 uses the correct
   offset despite the lookahead record.
9. Result envelopes serialize when models do; errors redact retained causes.
10. Feature-boundary, missing-docs, Rust 1.94, and public documentation checks
    cover generated consumers. PostgreSQL-backed checks verify real relation
    and ordering behavior; mock-only checks are reported separately.

## Sources consulted

- Existing macro, persistence connector, and posts-crud source in this repository.
- Installed SeaORM 2.0.3 EntityTrait, EntityLoaderTrait, QueryFilter,
  ActiveModelTrait, and generated Entity Loader source.
- [SeaORM Entity Loader](https://www.sea-ql.org/SeaORM/docs/relation/entity-loader/).
- [SeaORM Model Loader](https://www.sea-ql.org/SeaORM/docs/relation/model-loader/).
