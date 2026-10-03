# Changelog

All notable changes to furnace-rs are documented in this file. Published MADS
release history is retained below; the pending 0.9.2 work and furnace-rs API
migration are consolidated into the first 1.0.0 release.

## [Unreleased]

### Security

- Bound incomplete initial HTTP requests, including idle sockets and partial
  protocol prefaces, and HTTP/1 request headers to ten seconds. Preserve HTTP/2,
  upgrades, long-running handlers, and graceful connection draining.
- Add a ten-second idle deadline while reading request bodies. Progress renews
  the deadline; stalled reads return a safe HTTP 408 response before response
  headers are sent and close the affected HTTP/1 connection. A timeout during
  response streaming terminates that stream. Body-size limits remain enforced.
- Record isolated before/after regression measurements in the
  [HTTP runtime security report](benchmark/HTTP_RUNTIME_SECURITY.md).

## [1.0.0] - 2026-10-03

The workspace is prepared for the first stable furnace-rs release on 2026-10-03. Publication,
the `v1.0.0` tag, and the GitHub release still require the release workflow.

### Breaking changes and migration

- Rename all nine packages from the MADS family to `furnace-rs` and
  `furnace-rs-*`; rename the CLI executable to `furnace` and diagnostics to
  the `FURNACE` family. No aliases preserve the retired declarations.
- Replace module/service/repository attributes with `#[cauldron]`, `#[burner]`,
  and `#[storage]`. Replace provider/element factories with typed `Injector`
  construction and explicit `.provide_with::<Output, Implementer>()` bindings,
  including application-authored shared trait-object outputs.
- Declare members, controllers, imports, exports, and global status in
  `Cauldron::register`. Rust `pub` controls Rust visibility; cauldron exports
  control dependency access, and explicit guards control HTTP authorization.
- Start conventional applications with `Furnace::burn` and `FurnaceBurnExt`.
- Declare endpoints in inherent `#[controller(route = "/users")]`
  implementations; remove `#[routes]` and `controller(routes = ...)`.
  Controllers are public without `impl Sealable`; a static `#[guard]` policy
  attached through `Sealable` protects endpoints, with `#[seal(skip)]` for
  intentionally public exceptions.
- Rename conventional files/environment keys to `furnace.toml` and `FURNACE_*`.
  Finite CLI reports and private inspection use schema/protocol 2. The offline
  generator creates six direct-controller application files.

See [the migration guide](docs/importance/furnace-rs-migration.md) before
upgrading an application from MADS 0.x.

### Added

- `furnace-rs-testing` supplies focused, module-free fixtures for registered
  providers, services, repositories, and controllers; supplied native mocks
  prevent production database construction during test setup.
- `#[furnace::test]` registers a zero-argument async test and creates a local
  `test_fixture()` helper without requiring a direct Tokio dev dependency.
- Scripted SeaORM `MockDatabase` values can supply native `DatabaseConnection`
  dependencies. In-process requests support status, JSON, text, and header
  assertions for only the selected controller's routes and guards.
- Scoped fixtures start lifecycle hooks and await shutdown after completion or
  an unwinding test panic, preserving rollback and the original panic.
- Reproducible HTTP authentication, core/database, and CLI filesystem security
  workloads with failure-sensitive JSON reports and source/binary fingerprints.
- A [security policy](docs/SECURITY.md), [combined security audit](docs/SECURITY_AUDIT.md),
  and [1.0.0 release readiness guide](docs/releases/1.0.0.md).

### Security

- Enforce strict Bearer credential syntax before authentication adapters or
  handlers execute. Internal tab separators, duplicate/combined credentials,
  and invalid token characters reject with the generic 401/Bearer response;
  case-insensitive schemes and multiple ASCII spaces remain accepted. The
  reproduced parser differential required a valid signed JWT, not a forged one.
- Redact retained causes from ordinary core `Error` debug formatting, including
  nested native errors that can contain database credentials. Typed causes
  remain available through explicit `source()` access.
- Reject unrepresentable typed and native database timeout deadlines before
  SQLx can panic on `Instant` arithmetic; preserve valid and disabled policies.
- Suppress credential-bearing SeaORM connection-establishment tracing with a
  future-scoped dispatcher. Surrounding application tracing is restored;
  native handshake telemetry and inline callback tracing are suppressed.
- Create Unix CLI inspection/control directories with atomic owner-only `0700`
  permissions, preventing other local users from tampering under permissive
  umasks. The check covers actual permissions and owner access in isolated
  child processes; executable replacement races were not reproduced end to end.
- Retain the `time` advisory fix and the explicitly selected AWS-LC JWT backend,
  keeping the RustCrypto RSA implementation out of the normal workspace graph.
  A consumer enabling both crypto backends is covered separately.
- The all-crate audit scanned all nine workspace crates and seven checked-in
  lockfiles, with no known advisory matches at its recorded RustSec snapshot.
  Historical reports keep their original versions, commits, and limitations.
  The prepared 1.0.0 tree also passed a refreshed eight-lockfile advisory scan,
  security smoke contracts, and authenticated live PostgreSQL validation; see
  the [release verification](docs/releases/1.0.0.md#local-verification).

### Compatibility and release tooling

- Align all nine workspace packages, including CLI and testing, at `1.0.0` with
  exact internal pins. Keep Rust edition 2024, MSRV 1.94, current public feature
  boundaries, Axum 0.8, and native SeaORM PostgreSQL integration (minimum 2.0.0).
- Stable and beta publication retain dependency-ordered publishing and now
  require advisory and security-regression verification before publication.
- Update active examples, installation snippets, migration/architecture/CLI
  documentation and version-sensitive generated-project acceptance tests.
- Correct benchmark version reporting to read the actual workspace version
  rather than labeling new local-source runs with a historical release number.

## [0.9.1] - 2026-09-25

### Fixed

- Oversized JSON requests now return `413 Payload Too Large` with
  `Connection: close`, allowing HTTP/1.1 clients to continue safely after the
  server closes the rejected request's connection.
- Passport documentation examples now compile with both the default feature
  set and the `jwt` feature enabled.

## [0.9.0] - 2026-09-24

### Compatibility

- Raise the minimum supported Rust version from 1.85 to 1.94. MADS continues
  to use Rust edition 2024.

### Added

- `mads-persistence` provides an explicitly imported SeaORM PostgreSQL
  `DatabaseModule`, native `DatabaseConnection` injection, readiness and
  graceful close. `DatabaseFactory::provide` returns the native connection or
  a typed, safely formatted `PersistenceError`.

- `mads-common` provide new logger module to imported into main apps with
  default console logger. User can override the logger inner instance using
  provide macros and provide their logger service

### Breaking changes

- Removed the Diesel-backed `mads-common/database` and `mads/database`
  features and their public API. Persistence is available only by depending on
  `mads-persistence` explicitly with `sea-orm-postgres`.
- Removed the `mads db` migration commands and database-specific JSON data
  types. Use SeaORM's native migration tooling instead.
- `mads-cli` now inherits and publishes the 0.9.0 workspace version alongside
  the framework crates. Release scripts no longer accept
  `--keep-cli-version`.

## [0.8.1] - 2026-09-20

### Added

- `#[module(global)]` marks an imported module's public providers as available
  to every module reachable from the application root, without requiring each
  consumer to declare a direct import.

### Compatibility

- `mads-cli` remains at version `0.8.0` and consumes the `0.8.1` framework
  crates through exact internal dependencies.

## [0.8.0] - 2026-09-10

Stable release of the MADS.rs CLI, development loop, and framework
diagnostics.

### Included

- Offline, atomic `mads new <name>` generation of the exact seven-file minimal
  HTTP application. The generated package starts at `0.1.0`, pins the installed
  MADS version exactly, enables only `http` and `runtime-tokio`, and serves
  `Hello World!` from `GET /`.
- `#[derive(serde::Deserialize, Input)]`, the complete built-in validator
  matrix, custom and manual validation, deterministic sourced issues, and
  `ValidatedJson`, `ValidatedQuery`, and `ValidatedPath` handler boundaries.
- Seven named REST errors with a common safe JSON envelope, normalized
  MADS-owned Passport/cookie/validated-extractor rejections, retained Bearer
  challenges, and redacted internal sources.
- Explicit `.into_http()` conversion for MADS database results and native
  Diesel query results when `http + database` is enabled; there is no automatic
  database-to-HTTP conversion.
- `#[derive(Configuration)]`, explicit `Config::parse`, structured aggregated
  failures, startup-provider validation, preserved source loading/precedence,
  and `Secret<T>` with explicit exposure and redacted formatting.
- Focused stable/MSRV compiler diagnostics for constraints owned by MADS
  macros, while unrelated rustc, Cargo, Axum, Diesel, and application failures
  remain native.
- Optional schema version 1 JSON for `new`, `routes`, `graph`, `doctor`, and all
  four finite database commands, including deterministic records, partial
  inspection data, safe diagnostics, and existing 0/1/2 exit classes.
- Cross-platform policy for portable parser, JSON, path, scaffold, and process
  gates, with complete workspace/package/coverage and PostgreSQL gates on
  Linux.

### Compatibility boundaries

- Human output remains the default. `run` and `dev` keep raw Cargo, rustc, and
  application streams and reject `--format`.
- Native Axum `Json`, `Query`, `Path`, routers, middleware, and responses remain
  available without automatic `Input` validation or MADS normalization.
- Typed configuration reads the existing loaded `Config`; it does not add
  sources, alter precedence, or globally discover derived types.
- The generator adds no database, JWT, cookie, migration, Git, dependency
  installation, remote-template, or additional-generator behavior.
- Publication, tags, GitHub releases, and stable version changes are separate
  release operations.

## [0.8.0-beta.1] - 2026-09-09

Complete beta of the MADS.rs validation, configuration, REST delivery, machine
output, and minimal-project workflow. Stable `0.8.0` will promote this same
feature set after fixes and verification; it will not add features.

### Included

- Offline, atomic `mads new <name>` generation of the exact seven-file minimal
  HTTP application. The generated package starts at `0.1.0`, pins the installed
  MADS version exactly, enables only `http` and `runtime-tokio`, and serves
  `Hello World!` from `GET /`.
- `#[derive(serde::Deserialize, Input)]`, the complete built-in validator
  matrix, custom and manual validation, deterministic sourced issues, and
  `ValidatedJson`, `ValidatedQuery`, and `ValidatedPath` handler boundaries.
- Seven named REST errors with a common safe JSON envelope, normalized
  MADS-owned Passport/cookie/validated-extractor rejections, retained Bearer
  challenges, and redacted internal sources.
- Explicit `.into_http()` conversion for MADS database results and native
  Diesel query results when `http + database` is enabled; there is no automatic
  database-to-HTTP conversion.
- `#[derive(Configuration)]`, explicit `Config::parse`, structured aggregated
  failures, startup-provider validation, preserved source loading/precedence,
  and `Secret<T>` with explicit exposure and redacted formatting.
- Focused stable/MSRV compiler diagnostics for constraints owned by MADS
  macros, while unrelated rustc, Cargo, Axum, Diesel, and application failures
  remain native.
- Optional schema version 1 JSON for `new`, `routes`, `graph`, `doctor`, and all
  four finite database commands, including deterministic records, partial
  inspection data, safe diagnostics, and existing 0/1/2 exit classes.
- Cross-platform policy for portable parser, JSON, path, scaffold, and process
  gates, with complete workspace/package/coverage and PostgreSQL gates on
  Linux.

### Compatibility boundaries

- Human output remains the default. `run` and `dev` keep raw Cargo, rustc, and
  application streams and reject `--format`.
- Native Axum `Json`, `Query`, `Path`, routers, middleware, and responses remain
  available without automatic `Input` validation or MADS normalization.
- Typed configuration reads the existing loaded `Config`; it does not add
  sources, alter precedence, or globally discover derived types.
- The generator adds no database, JWT, cookie, migration, Git, dependency
  installation, remote-template, or additional-generator behavior.
- Publication, tags, GitHub releases, and stable version changes are separate
  release operations.

## [0.7.0] - 2026-09-04

Stable release of the MADS.rs CLI, development loop, and framework
diagnostics.

### Included

- Cargo-native `mads run` and `mads dev` with package, binary, and argument forwarding.
- `mads routes`, `mads graph`, and `mads doctor` compiled application inspection.
- `mads db generate` with automatic naming and recursive split-schema loading.
- `mads db migrate`, `mads db rollback`, and `mads db status` database operations.
- Human-readable diagnostics, stable exit classes, redaction, and Linux CI coverage.
- Bounded PostgreSQL schema diff generation with review-required reversible SQL.

### Release boundaries

- Inspection supports the standard `Mads::run::<AppModule>()` entry point only.
- Unsupported schema details such as defaults, indexes, checks, triggers, and complete foreign-key policy require manual SQL review.
- Output is human-readable only; no machine-readable mode is part of v0.7.
- Input validation, expanded HTTP errors, generic typed configuration, and related validation work are deferred to v0.8.

## [0.7.0-beta.1] - 2026-09-01

Complete beta of the MADS.rs CLI, development loop, and framework diagnostics.

### Included

- Cargo-native `mads run` and `mads dev` with package, binary, and argument forwarding.
- `mads routes`, `mads graph`, and `mads doctor` compiled application inspection.
- `mads db generate` with automatic naming and recursive split-schema loading.
- `mads db migrate`, `mads db rollback`, and `mads db status` database operations.
- Human-readable diagnostics, stable exit classes, redaction, and Linux CI coverage.
- Bounded PostgreSQL schema diff generation with review-required reversible SQL.

### Beta limitations

- Inspection supports the standard `Mads::run::<AppModule>()` entry point only.
- Unsupported schema details such as defaults, indexes, checks, triggers, and complete foreign-key policy require manual SQL review.
- Output is human-readable only; no machine-readable mode is part of v0.7.
- Input validation, expanded HTTP errors, generic typed configuration, and related validation work are deferred to v0.8.

## [0.6.0-beta.1] - 2026-08-29

First public beta of the MADS.rs HTTP application foundation.

### Included

- Root-module application scope and managed dependency construction.
- Typed HTTP route contracts and managed controllers on Axum.
- Conventional HTTP startup, configuration, CORS, and graceful shutdown.
- PostgreSQL/Diesel integration with managed lifecycle and migrations.
- JWT, cookie, Passport strategy, principal, and route guard support.
- Native Axum and Diesel escape hatches for application-owned composition.

### Beta limitations

- Declarative validation, OpenAPI generation, and generic trait bindings are not included.
- TLS, HTTP/2 configuration, multiple listeners, and declarative middleware are application-owned.
- Public APIs may change in later `0.6.0-beta.*` releases based on adopter feedback.

[0.9.1]: https://github.com/Adriannathan89/mads/releases/tag/v0.9.1
[0.9.0]: https://github.com/Adriannathan89/mads/releases/tag/v0.9.0
[0.8.0]: https://github.com/Adriannathan89/mads/releases/tag/v0.8.0
[0.6.0-beta.1]: https://github.com/Adriannathan89/mads/releases/tag/v0.6.0-beta.1
[0.7.0]: https://github.com/Adriannathan89/mads/releases/tag/v0.7.0
[0.7.0-beta.1]: https://github.com/Adriannathan89/mads/releases/tag/v0.7.0-beta.1

[1.0.0]: https://github.com/Adriannathan89/furnace-rs/releases/tag/v1.0.0
