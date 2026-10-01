# furnace-rs-core

`furnace-rs-core` is the framework-neutral semantic core of furnace-rs. It owns the
application model and startup decisions without knowing about Axum, HTTP,
Diesel, PostgreSQL, JWT, cookies, or Serde.

Application authors normally reach this API through the
[`furnace-rs` facade](../furnace-rs/README.md). Use `furnace-rs-core` directly when building a
framework-neutral integration or when a contribution changes graph,
configuration, provider, or lifecycle behavior.

## What this crate owns

- Source-aware scalar and string-array `Config` values, configuration builders,
  TOML support, dotenv interpolation, and configuration diagnostics.
- The `Configuration` trait, `#[derive(Configuration)]` support, and the
  redacted `Secret<T>` wrapper.
- Static provider and module descriptors collected through `inventory`.
- Root-module selection, explicit cauldron ownership, exports, direct imports, and the
  retained `CauldronGraph`.
- Concrete-type provider graph analysis, deterministic construction plans,
  duplicate/ambiguous/missing/cycle diagnostics, and provider state.
- The application-scoped provider registry and construction/application
  contexts.
- Official auto-configuration evaluation and redacted decision reports.
- Lifecycle state, infrastructure hooks, application hooks, rollback, and
  reverse-order shutdown.
- Stable structured FURNACE diagnostics and the low-level `FurnaceBuilder`/`Furnace` API.

The core is intentionally generic. For example, it can decide that an
application requires a type named `Database`, but the PostgreSQL implementation
and the rule that supplies that type belong to `furnace-rs-common`.

## Core execution model

FURNACE combines compile-time metadata with startup-time analysis:

~~~text
core macros
    │
    ▼
static module/provider descriptors
    │
    ▼
inventory catalog + Config
    │
    ▼
root scope and module ownership
    │
    ▼
auto-configuration analysis
    │
    ▼
concrete-type graph validation
    │
    ▼
deterministic construction plan
    │
    ▼
provider registry + lifecycle-managed application
~~~

`FurnaceBuilder::analyze()` is side-effect free. A successful `build()` applies
selected official defaults, constructs providers in plan order, and retains
owned graph and auto-configuration reports for inspection. Provider
constructors receive a typed construction context and resolve concrete Rust
types; requests do not trigger graph planning or general-purpose runtime
reflection.

## Important invariants

- A rooted build analyzes the selected module closure. A builder without
  `root::<M>()` keeps the complete-catalog compatibility path.
- A provider is selected by concrete output type. Exact duplicates, ambiguous
  outputs, missing dependencies, and cycles fail before construction.
- Cauldron imports expose explicit exports from direct imports. Reachable global
  furnaces expose only their exports; Rust namespaces do not grant visibility.
- Auto-configuration is conditional and explainable. It supplies a missing
  infrastructure type only when the linked integration, graph requirement, and
  configuration conditions all match; an explicit application provider wins.
- Analysis and diagnostics retain evidence without serializing secrets or
  configuration values.
- Infrastructure lifecycle hooks are ordered before application hooks and are
  unwound in reverse order when startup or shutdown fails.

## Public building blocks

| API | Purpose |
| --- | --- |
| `Config`, `ConfigBuilder`, `ConfigSource` | Build and inspect source-attributed configuration values. |
| `Configuration` and `Config::parse` | Parse a typed application view from the existing configuration document. |
| `Secret<T>` | Make secret exposure explicit; ordinary formatting is redacted. |
| `Cauldron`, `CauldronRegistration`, `FurnaceBuilder`, `Furnace` | Select, analyze, construct, inspect, start, and shut down an application. |
| `ProviderDescriptor` and `CauldronDescriptor` | Represent compile-time-generated provider/module metadata. |
| `ApplicationGraph`, `ConstructionPlan`, `CauldronGraph` | Inspect selected ownership, dependency edges, and construction order. |
| `AutoConfigurationReport` | Explain active, skipped, overridden, or failed official defaults. |
| `Diagnostic` and `Error` | Carry stable codes, source locations, suggestions, and error causes. |

The core re-exports its procedural macros for ergonomic use. The macro
implementation itself lives in [`furnace-rs-core-macros`](../furnace-rs-core-macros/README.md).

## Features and dependencies

`furnace-rs-core` has no default features. The only feature is:

| Feature | Adds | Why it is optional |
| --- | --- | --- |
| `runtime-tokio` | Tokio runtime support used by the async entry-point helper. | Consumers may provide their own runtime or use a different integration boundary. |

Its direct dependencies are:

- `furnace-rs-core-macros` for generated declarations and entry points.
- `inventory` for link-time/static descriptor collection.
- `dotenvy`, `toml`, and `toml_edit` for the generic configuration pipeline.
- Optional `tokio` for `runtime-tokio`.

It is a dependency of `furnace-rs`, `furnace-rs-common`, and `furnace-rs-extra`. `furnace-rs-cli`
reaches it through the facade and common integration crates.

## Source layout

- `src/builder.rs` — explicit builder, analysis, construction, and retained
  application state.
- `src/graph/` — provider graph, module graph, scopes, diagnostics, and plans.
- `src/auto_configuration/` — generic official-default evaluation and reports.
- `src/config/` and `src/configuration/` — loaded values, typed parsing, and
  secrets.
- `src/descriptor.rs` and `src/catalog.rs` — static metadata and catalog
  normalization.
- `src/context.rs` and `src/registry.rs` — construction/application contexts
  and the concrete-type registry.
- `src/lifecycle.rs` — startup, shutdown, ordering, and rollback.
- `src/diagnostic.rs` — stable framework diagnostics.
- `src/runtime.rs` — the optional Tokio blocking helper.

When changing a macro declaration, update the matching descriptor or model here
as well. When changing a runtime integration, keep the generic policy in this
crate and put integration-specific behavior in `furnace-rs-common`.

## Tests and contributor workflow

Run focused core tests with:

~~~sh
cargo test -p furnace-rs-core --all-features
~~~

Graph, configuration, lifecycle, and diagnostic tests belong beside the
implementation in `crates/furnace-rs-core/src`. Compile-time declaration contracts
are exercised through the trybuild fixtures in
`crates/furnace-rs/tests/ui`. Follow the workspace gates in
[`CONTRIBUTING.md`](../../CONTRIBUTING.md) after focused tests pass.

See the [workspace architecture](../../README.md#workspace-crates) and
[architecture reference](../../docs/ARCHITECTURE.md) for cross-crate rules.
