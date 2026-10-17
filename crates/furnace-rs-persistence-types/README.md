# furnace-rs-persistence-types

Shared storage error contracts for FURNACE's optional SeaORM entity helpers.

This crate defines `StorageError`, `StorageErrorKind`, and `StorageResult`.
Errors retain their native SeaORM cause for explicit inspection while keeping
`Display` and `Debug` output redacted. It registers no database providers and
opens no database connections.

Applications normally use these types through
`furnace_rs_persistence::storage` with the `sea-orm-storage` feature. The HTTP
integration depends on this crate to support `?` in handlers without linking
the PostgreSQL connector's provider registrations.

See the [storage guide](../../docs/storage.md) for usage and error mapping.
