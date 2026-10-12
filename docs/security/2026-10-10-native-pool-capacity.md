# Native PostgreSQL zero-capacity pool rejection

Baseline: `bc16a4fc20a7a03ff13cda98de93121fbd2a8541` (1.0.2 source).
Affected implementation: `furnace-rs-persistence/src/sea_orm/connector.rs`.

## Reproduction and prerequisites

Typed configuration already rejected zero pool capacity, but the public native
`SeaOrmPostgres::from_options` and `options_mut` paths did not. With lazy
connection enabled, this input panicked during pool creation without needing a
database:

```rust
let mut options = ConnectOptions::new("postgres://localhost/unused");
options.max_connections(0).connect_lazy(true);
SeaOrmPostgres::from_options(options).connect().await;
```

The real regression executes that public connector in a Tokio task and requires
a typed error instead of a panic. Before the fix it observed
`crossbeam_queue::ArrayQueue` panicking with `capacity must be non-zero`, followed
by `JoinError::Panic`.

```sh
cargo test --locked -p furnace-rs-persistence --features sea-orm-postgres --test connector native_zero_pool_capacity -- --nocapture
```

This is a startup availability bug requiring control of native connection
options. It is not a demonstrated remote request-triggered denial of service.
No very-large-capacity allocation experiment was performed.

## Patch method and compatibility

At the common native connection boundary, reject an explicitly configured zero
maximum before entering SeaORM/SQLx. Return the existing redacted
`InvalidConfiguration` error with operation `configure`. The check runs after
PostgreSQL scheme validation and before pool allocation. Positive and default
capacities retain their existing behavior; a capacity-one lazy connection is the
regression's success control and requires no live database.

Evidence: [baseline](evidence/2026-10-10-pool-red.txt),
[patched](evidence/2026-10-10-pool-green.txt).
