# Posts CRUD with a service and PostgreSQL

This independent FURNACE 1.0.1 project connects to PostgreSQL through the opt-in
`furnace-rs-persistence` SeaORM connector. `DatabaseCauldron` supplies the native
`DatabaseConnection` to `PostRepository`; `PostService` coordinates calls to the
repository and `PostController` exposes the HTTP routes. The connector checks
readiness before the HTTP listener binds and closes the connection during
graceful shutdown.

Requires Rust 1.94 or newer, PostgreSQL, and `psql`. From this directory:

```sh
createdb -h 127.0.0.1 -U postgres mads_posts_example
cp .env.example .env
# Edit DATABASE_URL in .env for your local PostgreSQL credentials.
# Run psql with the same URL (the .env file is loaded by FURNACE, not by psql).
psql 'postgres://postgres:postgres@127.0.0.1:5432/mads_posts_example' -f migrations/001_create_posts.sql
cargo run
```

The `furnace` CLI has no database commands. Apply the SQL file with `psql`
before starting this example. `furnace.toml` reads `DATABASE_URL` through dotenv
interpolation; `.env` is ignored by Git.

Try the CRUD routes in a second terminal:

```sh
curl -i -X POST http://127.0.0.1:3001/posts \
  -H 'Content-Type: application/json' \
  -d '{"title":"First post","body":"Hello from FURNACE"}'
curl http://127.0.0.1:3001/posts
curl http://127.0.0.1:3001/posts/1
curl -X PUT http://127.0.0.1:3001/posts/1 \
  -H 'Content-Type: application/json' \
  -d '{"title":"Updated post","body":"New body"}'
curl -i -X DELETE http://127.0.0.1:3001/posts/1
```

The create route returns 201; delete returns 204. A missing post returns 404.
`ValidatedJson<PostInput>` rejects an empty or too-long title before the
controller runs. Database failures become redacted 500 responses, while the
underlying error remains available to the server for diagnosis.

## When to use a service

The request flow is `PostController → PostService → PostRepository → database`.
`PostService` is a thin forwarding layer in this example to demonstrate dependency
wiring. Add business rules or coordinate multiple repositories here when needed.
For straightforward CRUD, start with the [simpler example](../posts-crud/), whose
controller calls its repository directly.

Handlers use `.await?` with the opt-in `sea-orm` feature on `furnace-rs` to turn
`DbErr` into redacted HTTP 500 responses. The service and repository keep their
native `Result<_, DbErr>` signatures.

Both CRUD projects use port 3001 and the same database name by default. Run them
one at a time, or adjust the port in `furnace.toml` and `DATABASE_URL` in `.env`.
