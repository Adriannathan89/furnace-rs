# furnace-rs-persistence

Native persistence connector integration for furnace-rs. Connector features are
opt-in; the crate has no default database backend.

For PostgreSQL, depend explicitly on:

```toml
furnace-rs-persistence = { version = "0.9.2", features = ["sea-orm-postgres"] }
```

Import `furnace_rs_persistence::sea_orm::DatabaseCauldron` in the application root.
Its global provider exposes SeaORM's native `DatabaseConnection` to selected
services and repositories. Readiness is checked before the listener binds;
the connection closes on graceful shutdown. The connector does not generate
or apply migrations.

For manual construction, `DatabaseFactory::provide(SeaOrmPostgres::new(url))`
returns `PersistenceResult<DatabaseConnection>`: the native connection on
success or a typed `PersistenceError` on failure. `PersistenceError::kind()`
classifies failures while public formatting redacts connection details. FURNACE
does not map persistence failures automatically to HTTP responses. See the
[persistence design](../../docs/furnace-rs-persistence.md) for the connector model.

Configured and native pool durations must fit the platform's monotonic-clock
deadline range. Unrepresentable values return `InvalidConfiguration` before
connecting; typed errors identify the affected key without printing its value.

The connector suppresses tracing while polling SeaORM's connection-establishment
future because its native connection span includes the credential-bearing URL.
This also suppresses tracing from callbacks running inside that operation.
Tracing outside connection establishment and configured statement logging on
the returned database connection keep their existing behavior. Applications
should keep credentials out of their own diagnostic messages and log events.
