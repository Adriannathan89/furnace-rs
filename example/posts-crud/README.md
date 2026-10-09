# Posts CRUD with PostgreSQL

This independent FURNACE 1.0.2 project connects to PostgreSQL through the opt-in
`furnace-rs-persistence` SeaORM connector. `DatabaseCauldron` supplies the native
`DatabaseConnection` to `PostRepository`; `PostController` calls the repository
directly and exposes the HTTP routes. The connector checks readiness before
the HTTP listener binds and closes the connection during graceful shutdown.

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

## Simple handlers and optional services

The default example needs only a controller, repository, and model. Enable the
`sea-orm` feature on `furnace-rs` so repository errors propagate through
`HttpResult` with `?`:

```rust,ignore
#[get]
async fn list(&self) -> HttpResult<Json<Vec<Post>>> {
    Ok(Json(self.repository.list().await?))
}
```

Every `DbErr` becomes a redacted 500 response. Map an error explicitly when your
application needs a different status, such as 409 for a known domain conflict.

Use a service when there is business logic to coordinate. The runnable
[post-crud-with-service project](../post-crud-with-service/) demonstrates that optional
layer using the same model, repository, database configuration, and CRUD routes:

```sh
# Stop the default server first; both variants use port 3001.
cd ../post-crud-with-service
cargo run
```

`PostService` is deliberately a thin forwarding layer here to illustrate the
wiring; the default application works without it.
