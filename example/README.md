# FURNACE 0.9 examples

These are three independent Rust projects. Run each command from its example
directory so `Furnace::burn` loads that project's `furnace.toml` and optional `.env`.
Each project uses local workspace crates to demonstrate the new explicit cauldron
registration API.

| Project | What it demonstrates | Port |
| --- | --- | --- |
| [hello-world](hello-world/) | The smallest FURNACE HTTP application | 3000 |
| [posts-crud](posts-crud/) | PostgreSQL, `furnace-rs-persistence`, SeaORM, and post CRUD | 3001 |
| [protected-route](protected-route/) | TPRS, validated login, Passport JWT guard, and logger | 3002 |

Start with Hello World, then use the PostgreSQL example when you need a real
database. The protected-route example is self-contained and needs no database.
Each directory contains its own setup steps and `curl` requests.

The [TPRS reference project](https://github.com/Adriannathan89/furnace-rs-rs-example)
defines Trait–Provider–Repository–Service: traits express application contracts,
providers bind them to implementations, repositories handle data access, and
services implement use cases. Controllers are the HTTP boundary. The JWT
example follows that pattern with an in-memory repository. The reference uses
FURNACE 0.8 APIs, so these projects use the current 0.9 crate boundaries.
