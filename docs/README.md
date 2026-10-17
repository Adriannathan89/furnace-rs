# FURNACE documentation

The current stable release is **2.0.0**, available on
[crates.io](https://crates.io/crates/furnace-rs/2.0.0). The root
[README](../README.md) covers installation and first use. The guides below
describe the current workspace; release records preserve behavior and
verification for their named versions.

## Guides

| Guide | Purpose |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Crate boundaries, graph selection, construction, and lifecycle |
| [Implementation guide](IMPLEMENTATION.md) | Source-linked walkthrough of compile-time and runtime flows |
| [CLI](CLI.md) | Scaffolding, run/dev, inspection, target selection, and JSON schema 2 |
| [Declarative DI](declarative-di.md) | Compile-time roots, module API, visibility, and migration |
| [gRPC transport](grpc.md) | Protobuf services, named clients, Passport/JWT, shared runtime, TLS, and static inspection |
| [Entity-bound storage](storage.md) | SeaORM CRUD, relation loading, ordering, pagination, and error mapping |
| [Persistence](furnace-rs-persistence.md) | Native SeaORM PostgreSQL setup, configuration, and errors |
| [Migration from MADS](importance/furnace-rs-migration.md) | Removed declarations and their FURNACE replacements |

## Release records

| Release | Notes and verification |
| --- | --- |
| 2.0.0 | [Release notes](releases/2.0.0.md), [verification](releases/2.0.0-verification.md) |
| 2.0.0-beta.3 | [Release notes](releases/2.0.0-beta.3.md), [verification](releases/2.0.0-beta.3-verification.md) |
| 2.0.0-beta.2 | [Release notes](releases/2.0.0-beta.2.md) |
| 2.0.0-beta.1 | [Release preparation](releases/2.0.0-beta.1.md), [verification](releases/2.0.0-beta.1-verification.md) |
| 1.0.2 | [Release record](releases/1.0.2.md) |
| 1.0.1 | [Release record](releases/1.0.1.md) |
| 1.0.0 | [Release record](releases/1.0.0.md) |

## Security and benchmark evidence

| Document | Purpose |
| --- | --- |
| [Security policy](SECURITY.md) | Current released version, vulnerability reporting, and application responsibilities |
| [Security audit](SECURITY_AUDIT.md) | Historical audit findings and their remediation records |
| [1.0.2 implementation scan](security/2026-10-10-implementation-scan.md) | Historical source-review scope, evidence, and limits |
| [Benchmark performance report](../benchmark/PERFORMANCE.md) | Benchmark method and recorded performance comparisons |
| [Benchmark security report](../benchmark/SECURITY.md) | Consolidated security findings, fixes, before/after evidence, and scope limits |
| [Extractor error redaction](security/2026-10-09-extractor-error-redaction.md) | 1.0.2 fix and baseline/patched evidence |

Use the [runnable examples](../example/) for Hello World, PostgreSQL CRUD, and
JWT-protected routes. The [crate guides](../README.md#workspace-crates) cover
individual packages; the [benchmark suite](../benchmark/) retains performance
results and security evidence for their recorded revisions.

[WebSocket transport](socket.md) covers managed gateways, owned sockets, rooms, Redis, and replay.
