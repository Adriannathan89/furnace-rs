# FURNACE examples

These are independent Rust projects. The checkout targets the 2.0.0-beta.2 release. Run each command from its example
directory so `Furnace::burn` loads that project's `furnace.toml` and optional `.env`.
Each project uses local workspace crates, declarative `#[cauldron(...)]`
registration and a marked `root` for compile-time dependency checking.

| Project | What it demonstrates | Port |
| --- | --- | --- |
| [compile-time-di](compile-time-di/) | Declared root checking and a missing-dependency Cargo test | None |
| [missing-dependency](missing-dependency/) | Intentionally fails compilation because a required provider is not registered | None |
| [grpc-transport](grpc-transport/) | All four protobuf RPC shapes, managed clients, Passport/JWT, health/reflection, and optional mixed/TLS mode | 50051 |
| [hello-world](hello-world/) | The smallest FURNACE HTTP application | 3000 |
| [posts-crud](posts-crud/) | PostgreSQL and SeaORM CRUD with a controller calling its repository directly | 3001 |
| [post-crud-with-service](post-crud-with-service/) | The same CRUD routes with an optional `PostService` layer | 3001 |
| [protected-route](protected-route/) | TPRS, validated login, Passport JWT guard, and logger | 3002 |

Start with Hello World, then use the PostgreSQL example when you need a real
database. The protected-route example is self-contained and needs no database.
Each directory contains its own setup steps and `curl` requests.
The two CRUD variants share the same default port and database; run them one at
a time or change their configuration. Start with `posts-crud` and add a service
when business logic needs it.

The [TPRS reference project](https://github.com/Adriannathan89/furnace-rs-rs-example)
defines Trait–Provider–Repository–Service: traits express application contracts,
providers bind them to implementations, repositories handle data access, and
services implement use cases. Controllers are the HTTP boundary. The JWT
example follows that pattern with an in-memory repository. The reference uses
FURNACE 0.8 APIs, so these projects use the current 2.0 crate boundaries.
