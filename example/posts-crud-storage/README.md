# Posts CRUD with entity-bound storage

This standalone FURNACE 2.0.0-beta.3 example enables the optional
`sea-orm-storage` feature on `furnace-rs-persistence`. Its controller calls the
generated `.entity()` API directly for create, read, update, delete, and ordered
page-number pagination. No service or repository methods wrap those operations.
The original [Posts CRUD example](../posts-crud/) remains the manual SeaORM
version, so you can compare both approaches.

Requires Rust 1.94 or newer, PostgreSQL, and `psql`. From this directory,
either create the example database or point `DATABASE_URL` at an existing
database you can use:

```sh
createdb -h 127.0.0.1 -U postgres furnace_posts_storage_example
cp .env.example .env
# Edit DATABASE_URL in .env for your local PostgreSQL credentials.
psql 'postgres://postgres:postgres@127.0.0.1:5432/furnace_posts_storage_example' \
  -f migrations/001_create_posts.sql
cargo run --locked
```

When reusing an existing database, set `DATABASE_URL` to that database and
apply the migration to the same URL. `DATABASE_URL` must name a database that
already exists; having a reachable PostgreSQL server alone is not enough.
The sample URL uses IPv4 loopback (`127.0.0.1`). If your `.env` uses
`localhost` and the PostgreSQL server accepts IPv4 only, use `127.0.0.1` there
as well; `psql` without `-h` tests a Unix socket and does not verify TCP access.

The server uses port 3004. List requests require both pagination values:

```sh
curl 'http://127.0.0.1:3004/posts?page_number=1&page_size=20'
```

`page_number` starts at 1. The example orders by post ID descending and returns
`items`, `page_number`, and `page_size`. The application owns the supplied page
size; invalid zero values receive HTTP 400. Create, update, delete, and lookup
are available at the same `/posts` routes as in the manual example.

Input validation and HTTP error mapping remain application code in the
controller. Native SeaORM models are mapped to the response DTO there. The
entity-bound helper is enabled explicitly in this project's Cargo manifest;
the separate `furnace-rs` `sea-orm-storage` feature converts an unmapped
`StorageError` to a redacted HTTP 500 for `?`. Use `.map_err(...)` on individual
queries when the controller wants a different response, as this example does
for invalid pagination. These features do not select a database driver or
manage transactions.
