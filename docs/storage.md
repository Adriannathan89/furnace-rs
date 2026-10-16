# Entity-bound storage

The optional `sea-orm-storage` persistence feature binds a managed storage
to one SeaORM entity. It supplies basic CRUD, typed filters, explicit ordering,
page-number pagination, and named native relation loaders.

## Dependencies and registration

~~~toml
furnace-rs = { version = "2.0.0-beta.2" }
furnace-rs-persistence = { version = "2.0.0-beta.2", features = ["sea-orm-storage", "sea-orm-postgres"] }
sea-orm = { version = "2.0", default-features = false, features = ["macros"] }
~~~

A direct persistence dependency is required, including when the storage
attribute is imported through the facade. Cargo-renamed dependencies work.
The storage feature alone selects no driver or runtime. The separate
`sea-orm-postgres` feature supplies PostgreSQL connectivity and the existing
DatabaseCauldron. Import that cauldron in your application root and register
the storage through the normal provider declaration.

The facade's `sea-orm` feature is a separate HTTP conversion for native DbErr;
it does not enable entity-bound storage or convert StorageError automatically.

## Declaration and connection selection

Given a SeaORM entity in module `post`:

~~~rust
use furnace_rs::storage;
use furnace_rs_persistence::sea_orm::DatabaseConnection;

#[storage(entity = post::Entity)]
pub struct PostRepository {
    #[inject]
    database: DatabaseConnection,
}
~~~

The macro selects exactly one injected field whose type path ends in
`DatabaseConnection`, then lets Rust verify the actual native type.
For aliases, renamed imports, or multiple connections, choose explicitly:

~~~rust
type Db = DatabaseConnection;

#[storage(entity = post::Entity, connection = primary)]
pub struct PostRepository {
    #[inject]
    primary: Db,
    #[inject]
    replica: DatabaseConnection,
}
~~~

Selection does not open or clone a connection. Bare `#[storage]` remains the
existing managed-provider contract and requires no persistence feature.
The selected field must be the native DatabaseConnection type or a type alias;
wrappers such as Arc<DatabaseConnection> and transaction handles are rejected.

## CRUD and application models

~~~rust
let found = repository.entity().find_by_id(id).await?;
let inserted = repository.entity().insert(active_model).await?;
let updated = repository.entity().update(changes).await?;
let deleted = repository.entity().delete_by_id(id).await?;
~~~

| Method | Input | Result inside StorageResult |
| --- | --- | --- |
| find_by_id | Native primary-key value, including composite tuples | Option<Entity::Model> |
| insert | Entity::ActiveModel | Entity::Model |
| update | Entity::ActiveModel with its primary key set | Entity::Model |
| delete_by_id | Native primary-key value | bool |

Insert and update preserve native save hooks and Set/Unchanged/NotSet behavior.
The helper adds no existence-check query before update. A missing update target
returns NotFound; an unset primary-key component retains the native database
error. Deletion uses native delete_by_id and does not instantiate an
ActiveModel or run ActiveModel deletion hooks.

Keep input validation, writable-field selection, domain rules, authorization,
and DTO conversions in application methods:

~~~rust
impl PostRepository {
    pub async fn find_post(&self, id: i32) -> StorageResult<Option<Post>> {
        Ok(self.entity().find_by_id(id).await?.map(Into::into))
    }
}
~~~

Here Post and its From<post::Model> conversion belong to the application.

## Filtering, ordering, and pagination

~~~rust
let page = repository
    .entity()
    .query()
    .filter(post::Column::Published.eq(true))
    .order_by_desc(post::Column::CreatedAt)
    .order_by_desc(post::Column::Id)
    .paginate(2, 20)
    .await?;
~~~

Import SeaORM ColumnTrait to build column predicates. Repeated filter calls
combine with AND. Use native Condition for OR groups. Ordering calls append
criteria exactly as written. Columns are typed root-entity columns; external
HTTP sort names must be mapped by application code.

Pagination requires at least one explicit order call. Missing ordering returns
MissingOrder before executing SQL. Include a unique tie breaker, such as all
primary-key columns, when predictable ordering matters. The helper adds no
default order or primary-key criteria. Native database NULL ordering applies.

Both pagination arguments are required u64 values. Page numbers start at 1.
The helper validates positive values and checks offset arithmetic:
`(page_number - 1) * page_size`. Application code chooses size and depth
limits; there is no framework cap, clamping, default size, or TOML policy.

`paginate(2, 20)` adds OFFSET 20 and LIMIT 20. It does not fetch an extra
record or execute a count. A page contains exactly this metadata:

~~~json
{
  "items": [],
  "page_number": 2,
  "page_size": 20
}
~~~

Page<T> supports Serde serialization when T does. Empty and beyond-end pages
retain the requested metadata. A full page does not say whether another page
exists. Applications needing totals issue their own explicit SeaORM count query.

`query().one().await` returns Option<Model>. It accepts an unordered query;
the chosen matching record is then unspecified. Any authored ordering is
preserved. There is no unbounded collection terminal on this convenience API;
custom repository methods can use native SeaORM for such queries.

Concurrent writes can shift offset pages, even when their ordering is unique.

## Explicit relation loading

Relation-enabled storage requires SeaORM 2's loader-compatible
`#[sea_orm::model]` or `#[sea_orm::compact_model]` entity format.
Basic CRUD and pagination without declared relations also support the older
DeriveEntityModel format.

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

~~~rust
let page = repository.entity().query()
    .with_author()
    .with_comments()
    .order_by_desc(post::Column::Id)
    .paginate(1, 20)
    .await?;
~~~

Only explicitly selected loaders execute. Repeated selection is idempotent.
The native loader chooses join/batch strategies. Pagination bounds the parent
selection before collection loading, and child rows do not consume parent
page slots. Child collections can still be large; their policy belongs to
the application.

Queries on a relation-enabled storage always return native Entity::ModelEx,
including when no relation is selected. Its native wrappers preserve unloaded,
loaded-empty, and loaded relation states. Basic CRUD still returns Entity::Model.
Native model serialization must also be enabled/derived when serializing those
results; the page envelope does not supply serialization for entity types.

When two relationships target the same entity, give the native SeaORM relation
fields distinct relation_enum names, for example:

~~~rust
#[sea_orm(belongs_to, from = "author_id", to = "id", relation_enum = "Author")]
pub author: BelongsTo<super::user::Entity>,
#[sea_orm(belongs_to, from = "editor_id", to = "id", relation_enum = "Editor")]
pub editor: BelongsTo<super::user::Entity>,
~~~

Then select the generated root relation paths in the storage declaration:

~~~rust
relations(
    author = post::Relation::Author,
    editor = post::Relation::Editor,
)
~~~

For an ordinary unambiguous relation use its target Entity selector. Explicit
root variants are for supported native ambiguous/self-relation cases; they
are not interchangeable with target Entity selectors. Nested relation DSLs
and string-based relationship lookup are not generated.

Named loaders are a convenience contract, not an authorization boundary.

## Transactions and custom queries

Applications create and manage transactions through SeaORM. Pass the native
transaction handle to each participating operation explicitly:

~~~rust
use sea_orm::{ActiveModelTrait, TransactionTrait};

let transaction = database.begin().await?;
let saved = active_model.insert(&transaction).await?;
transaction.commit().await?;
~~~

Generated helpers use the injected DatabaseConnection. They do not join a
separately created transaction, even if it originated from that connection.
Use native operations with the transaction handle throughout the transaction.

Custom joins, projections, aggregates, related-column sorting, and specialized
queries remain native SeaORM operations in application repository methods.

## Errors

StorageResult<T> uses a separate StorageError with these kinds:

- InvalidPagination: zero arguments or unrepresentable offset arithmetic.
- MissingOrder: pagination without explicit ordering.
- NotFound: the native update operation reports RecordNotUpdated.
- Database: other native database errors.

Display and Debug redact SQL, parameter values, and native cause text.
Error::source() retains the typed DbErr. HTTP/gRPC response mapping belongs to
the application. Connector lifecycle errors remain PersistenceError/FURNACE140.

## Validation

Focused tests cover generated SQL, native save hooks, composite keys, exact
page limits, relation identity, and independent Cargo feature consumers.
The compatibility fixtures use matching SeaORM/runtime macro pairs for 2.0.3
and 2.0.4. Real PostgreSQL acceptance runs in a unique temporary schema:

~~~sh
cargo test --locked -p furnace-rs-persistence \
  --features sea-orm-storage,sea-orm-postgres \
  --test storage_postgres -- --ignored
~~~

Supply FURNACE_TEST_DATABASE_URL for that command. Mock SQL checks establish
query shape and count; they are not latency or allocation benchmarks.
