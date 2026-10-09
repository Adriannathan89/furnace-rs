# furnace-rs

`furnace-rs` is the stable public facade for furnace-rs applications. It is the crate
most application authors should depend on. The facade composes the
framework-neutral [`furnace-rs-core`](../furnace-rs-core/README.md) with optional
[furnace-rs-common integrations](../furnace-rs-common/README.md) and re-exports the
macros, types, native Axum APIs, and prelude used by application code.

The facade is intentionally thin: it owns public naming, feature composition,
and re-exports, while graph, HTTP, and authentication behavior stays
in the crate that implements it.

## Dependency structure

- `furnace-rs-core` is always enabled.
- `furnace-rs-common` is optional and is selected with default features disabled.
- `furnace-rs-extra` is optional and is selected by the `extra` feature.
- Application crates and `furnace-rs-cli` depend on this facade.

The internal dependency shape is:

~~~text
application
└── furnace-rs
    ├── furnace-rs-core
    │   └── furnace-rs-core-macros
    ├── furnace-rs-common
    │   ├── furnace-rs-core
    │   └── furnace-rs-common-macros
    └── furnace-rs-extra
        └── furnace-rs-core

furnace-rs-cli
├── furnace-rs
└── furnace-rs-common (http-only inspection contract)
~~~

The arrow points from a consumer to its dependency. `furnace-rs-common` is
feature-selected by the facade; it is not enabled by default features inside
its own manifest.

## Feature matrix

| Feature | Expands to | Use |
| --- | --- | --- |
| `default` | `common` + `runtime-tokio` | Conventional HTTP application with the Tokio entry point. |
| `common` | `http` + `logger` | HTTP and logger aggregate; authentication remains opt-in. |
| `http` | `furnace-rs-common/http` | Axum routing/server, validation, CORS, and REST errors. |
| `logger` | `furnace-rs-common/logger` | Tracing-based application logging. |
| `jwt` | `furnace-rs-common/jwt` | JWT service, claims, profiles, algorithms, and key handling without Axum. |
| `cookies` | `http` + `furnace-rs-common/cookies` | Cookie extraction/response support; cookies imply HTTP. |
| `sea-orm` | `http` + `furnace-rs-common/sea-orm` | Propagate SeaORM `DbErr` through `HttpResult` with `?` as redacted 500 errors. |
| `runtime-tokio` | `furnace-rs-core/runtime-tokio` | Tokio support for `#[furnace_rs::main]`. |
| `extra` | `furnace-rs-extra` | Reserved extension boundary. |

Passport guards and strategies require `http + jwt`. Cookie guards add
`cookies`. Database access is a separate opt-in through the
[`furnace-rs-persistence`](../furnace-rs-persistence/README.md) crate.

For an HTTP-only application, use:

~~~toml
[dependencies]
furnace-rs = { version = "1.0.1", default-features = false, features = ["http", "runtime-tokio"] }
~~~

The workspace crates use exact internal version pins. External dependency
versions are maintained in the workspace root `Cargo.toml`.

## Public surface

The facade re-exports:

- Core declarations: `cauldron`, `burner`, `storage`, and typed `Injector` construction,
  `Configuration`, `Secret`, `Cauldron`, `CauldronRegistration`, and the builder/application types under
  `furnace_rs::core`.
- Integration declarations: HTTP verbs, `controller`, `guard`,
  `Input`, Passport derives, and strategy metadata when the required features
  are enabled.
- HTTP types: native Axum extractors/responses, validated extractors,
  `HttpResult`, standard REST errors, router builders, and serving functions.
- Authentication/cookies: `JwtService`, claims/options, Passport types,
  `CookieJar`, and cookie response composition.
- Native escape hatches: `furnace_rs::axum` and Tower-compatible router composition.

Use `furnace_rs::prelude` for the normal application surface. Reach into `furnace_rs::core`
when the application needs framework-neutral configuration, graph, or lifecycle
types.

## Startup contract exposed by the facade

The recommended path is:

~~~text
Furnace::burn::<AppCauldron>()
        │
        ├── loads .env/furnace.toml/FURNACE_* from the current directory
        ├── selects the rooted module scope
        ├── analyzes auto-configuration and the provider graph
        ├── validates selected endpoints, seals, and virtual dependency outputs
        ├── constructs providers and finalizes the Axum router
        ├── starts lifecycle and registered resource readiness
        ├── binds and serves
        └── shuts down in reverse order
~~~

The low-level builder and `serve_router` APIs remain available for explicit
configuration, lifecycle hooks, native router merging, or listener
addresses. A rootless builder retains complete-catalog compatibility. The
inspection commands use a separate private path and stop before construction or
runtime infrastructure.

## Source layout

`crates/furnace-rs/src/lib.rs` is the facade boundary. It contains:

- public re-exports and feature gates;
- the application prelude;
- Axum, logger, JWT, cookie, and Passport export wiring;
- documentation examples that must remain valid for external consumers.

Behavior changes belong in `furnace-rs-core` or `furnace-rs-common`, not in the facade.
When adding an API, update the owning crate first, then expose it here with the
smallest compatible feature gate.

## Tests and contributor workflow

Run facade consumer tests with:

~~~sh
cargo test -p furnace-rs --all-features
~~~

The facade's integration tests verify public feature combinations and external
consumer ergonomics. Macro fixtures live under `crates/furnace-rs/tests/ui`. When
changing a re-export or feature, run at least one no-default-features consumer
and the full workspace feature gate.

See the [root workspace guide](../../README.md#workspace-crates),
[architecture reference](../../docs/ARCHITECTURE.md), and
[contribution guide](../../CONTRIBUTING.md).
