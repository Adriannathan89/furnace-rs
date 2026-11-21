# FURNACE documentation

The current stable release is **2.0.0**, available on
[crates.io](https://crates.io/crates/furnace-rs/2.0.0). These pages describe
the current workspace.

## Start here

| If you want to | Read |
| --- | --- |
| Learn the framework step by step | [Learning path](tutorial/README.md) |
| See complete programs | [Runnable examples](../example/README.md) |
| Install and look up a feature | [Root README](../README.md) |
| Move from MADS or furnace-rs 0.x | [Migration guide](guides/furnace-rs-migration.md) |

## Learning path

Five steps, each ending in a program you can run.

| Step | Topic |
| --- | --- |
| [1. First application](tutorial/01-first-application.md) | Controller, cauldron, `Furnace::burn`, configuration file |
| [2. Providers and cauldrons](tutorial/02-providers-and-cauldrons.md) | Services, injection, trait bindings, imports and exports |
| [3. Configuration and validation](tutorial/03-configuration-and-validation.md) | Typed settings, secrets, validated input, error responses |
| [4. Accounts and login](tutorial/04-accounts-and-login.md) | Password hashing, JWT, strategies, guards |
| [5. Testing](tutorial/05-testing.md) | In-process fixtures, supplies, HTTP assertions |

## Examples by goal

| Goal | Example |
| --- | --- |
| Smallest HTTP application | [hello-world](../example/hello-world/) |
| Compile-time dependency checking | [compile-time-di](../example/compile-time-di/), [missing-dependency](../example/missing-dependency/) |
| Registration, password hashing, and login | [user-accounts](../example/user-accounts/) |
| JWT guard with trait-bound services | [protected-route](../example/protected-route/) |
| PostgreSQL CRUD with a repository | [posts-crud](../example/posts-crud/) |
| The same CRUD with a service layer | [post-crud-with-service](../example/post-crud-with-service/) |
| Generated entity CRUD and pagination | [posts-crud-storage](../example/posts-crud-storage/) |
| WebSocket gateways, rooms, and replay | [socket-transport](../example/socket-transport/) |
| gRPC services and clients | [grpc-transport](../example/grpc-transport/) |

## Topic guides

Every topic guide lives in [`guides/`](guides/).

### Application structure

| Guide | Purpose |
| --- | --- |
| [Declarative DI](guides/declarative-di.md) | Compile-time roots, cauldron API, visibility, and migration |
| [Architecture](guides/ARCHITECTURE.md) | Crate boundaries, graph selection, construction, and lifecycle |
| [Implementation guide](guides/IMPLEMENTATION.md) | Source-linked walkthrough of compile-time and runtime flows |
| [CLI](guides/CLI.md) | Scaffolding, run/dev, inspection, target selection, and JSON schema 2 |

### Security and authentication

| Guide | Purpose |
| --- | --- |
| [Password hasher](guides/password-hasher.md) | Argon2 hashing, verification limits, async use, custom backends, and injection |
| [Passport and JWT](../README.md#passport-configuration-and-jwt-profiles) | Key configuration, token profiles, strategies, and guards |
| [Security policy](SECURITY.md) | Vulnerability reporting and application responsibilities |

### Data

| Guide | Purpose |
| --- | --- |
| [Persistence](guides/furnace-rs-persistence.md) | Native SeaORM PostgreSQL setup, configuration, and errors |
| [Entity-bound storage](guides/storage.md) | SeaORM CRUD, relation loading, ordering, pagination, and error mapping |

### Transports

| Guide | Purpose |
| --- | --- |
| [WebSocket transport](guides/socket.md) | Managed gateways, owned sockets, rooms, Redis, and replay |
| [gRPC transport](guides/grpc.md) | Protobuf services, named clients, Passport/JWT, shared runtime, TLS, and static inspection |

### Crate references

The [crate table in the root README](../README.md#workspace-crates) links each
package's own guide, including the
[testing crate](../crates/furnace-rs-testing/README.md).

## Project records

These pages record what was released and verified. They describe their named
versions, not necessarily the current code.

### Releases

| Release | Notes and verification |
| --- | --- |
| 2.0.0 | [Release notes](releases/2.0.0.md), [verification](releases/2.0.0-verification.md) |
| 2.0.0-beta.3 | [Release notes](releases/2.0.0-beta.3.md), [verification](releases/2.0.0-beta.3-verification.md) |
| 2.0.0-beta.2 | [Release notes](releases/2.0.0-beta.2.md) |
| 2.0.0-beta.1 | [Release preparation](releases/2.0.0-beta.1.md), [verification](releases/2.0.0-beta.1-verification.md) |
| 1.0.2 | [Release record](releases/1.0.2.md) |
| 1.0.1 | [Release record](releases/1.0.1.md) |
| 1.0.0 | [Release record](releases/1.0.0.md) |

The [changelog](../CHANGELOG.md) lists every change, including unreleased work.

### Security reviews

| Document | Purpose |
| --- | --- |
| [1.0.0 security audit](SECURITY_AUDIT.md) | Audit findings and their remediation records |
| [1.0.2 implementation scan](security/2026-10-10-implementation-scan.md) | Source-review scope, evidence, and limits |
| [Extractor error redaction](security/2026-10-09-extractor-error-redaction.md) | 1.0.2 fix with baseline and patched evidence |
| [Cookie attribute injection](security/2026-10-10-cookie-attribute-injection.md) | Finding and fix record |
| [HTTP peer address](security/2026-10-10-http-peer-address.md) | Finding and fix record |
| [Native pool capacity](security/2026-10-10-native-pool-capacity.md) | Finding and fix record |
| [Scaffold directory privacy](security/2026-10-10-scaffold-directory-privacy.md) | Finding and fix record |
| [Benchmark security report](../benchmark/SECURITY.md) | Consolidated findings, fixes, and before/after evidence |

### Benchmarks

| Document | Purpose |
| --- | --- |
| [Performance report](../benchmark/PERFORMANCE.md) | Method and recorded performance comparisons |
| [Benchmark suite](../benchmark/README.md) | How to run the load and failure checks |
