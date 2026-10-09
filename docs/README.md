# FURNACE documentation

The current released version is **1.0.2**, available on
[crates.io](https://crates.io/crates/furnace-rs/1.0.2). The root
[README](../README.md) describes installation and usage; current source and tests
define behavior.

| Guide | Purpose |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Crate boundaries, graph selection, construction, and lifecycle |
| [CLI](CLI.md) | Scaffolding, run/dev, inspection, target selection, and JSON schema 2 |
| [Persistence](furnace-rs-persistence.md) | Native SeaORM PostgreSQL setup, configuration, and errors |
| [Migration from MADS](importance/furnace-rs-migration.md) | Removed declarations and their FURNACE replacements |
| [Security policy](SECURITY.md) | Current release, reporting, scope, and application responsibilities |
| [Security audit](SECURITY_AUDIT.md) | Historical 1.0.0 evidence and links to subsequent fixes |
| [1.0.2 release](releases/1.0.2.md) | Released changes, version alignment, and verification |
| [1.0.1 release](releases/1.0.1.md) | Released fixes and recorded preparation checks |
| [1.0.0 release record](releases/1.0.0.md) | Original stable-release contract and verification evidence |
| [Extractor error redaction](security/2026-10-09-extractor-error-redaction.md) | Fix released in 1.0.2 and baseline/patched evidence |

Use the [runnable examples](../example/) for Hello World, PostgreSQL CRUD, and
JWT-protected routes. The [crate guides](../README.md#workspace-crates) document
individual packages; the [benchmark suite](../benchmark/) retains measured
results and security regressions for their recorded source revisions.
