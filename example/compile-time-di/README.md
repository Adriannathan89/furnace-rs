# Compile-time DI example

This standalone beta example uses an existing burner, optional `#[inject]`, a local feature module, and global LoggerCauldron. No HTTP server or database is needed.

```sh
cargo run --manifest-path example/compile-time-di/Cargo.toml
cargo test --manifest-path example/compile-time-di/Cargo.toml
```

The test compiles an intentionally invalid fixture with its logger import removed. A passing test means rustc rejected that application for a missing Logger dependency. The invalid fixtures stay outside ordinary example builds. Stable and Rust 1.94 snapshots are included; both toolchains can run these commands.

The application retains ordinary runtime construction and lifecycle handling. See the [API and migration guide](../../docs/declarative-di.md) for visibility, native bindings, compatibility and the database root variant. DatabaseCauldron additionally needs `furnace-rs-persistence/sea-orm-postgres` and runtime `persistence.seaorm` configuration; it is not required by this example.
