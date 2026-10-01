# furnace-rs-core-macros

`furnace-rs-core-macros` is the procedural-macro implementation crate for
[furnace-rs-core](../furnace-rs-core/README.md). It translates user declarations into
typed constructors, static metadata, and small runtime adapters.

Application authors should use the re-exports from `furnace-rs-core` or `furnace-rs`. This
crate is an implementation dependency, not a normal application dependency.

## Macros provided

| Macro | Generated contract |
| --- | --- |
| `#[cauldron]` | Declares a unit cauldron and a callback to its authored `Cauldron::register` chain. |
| `#[element]` | Turns a provider function into a typed constructor and provider descriptor. |
| `#[burner]` | Declares an application-scoped managed service and its dependency metadata. |
| `#[storage]` | Declares an application-scoped repository with the same concrete-type wiring model. |
| `#[derive(Configuration)]` | Generates a typed, prefix-aware view over the core `Config` document. |
| `#[furnace_rs::main]` | Converts an async application entry point into a synchronous Tokio-backed entry point when the runtime feature is enabled. |

Provider, service, and repository arguments become concrete dependency edges.
Managed handles are required to be cloneable and shareable so the constructed
application can retain cheap, safe references. The generated code submits
descriptors to the core catalog and keeps source locations available for
diagnostics.

The macros parse declaration shape and generate metadata. They do not own graph
selection, auto-configuration policy, provider construction order, or
lifecycle semantics; those rules belong to `furnace-rs-core`.

## Dependency boundary

This crate is a `proc-macro` crate with no first-party runtime dependency and
no feature-specific API. Its direct dependencies are:

- `proc-macro-crate` for resolving the downstream core/facade crate name.
- `proc-macro2` for token streams.
- `quote` for generated Rust code.
- `syn` for parsing Rust syntax and visiting generated/declaration types.

Only `furnace-rs-core` depends on this crate. The public `furnace-rs` facade reaches the
macros through `furnace-rs-core` re-exports.

## Source layout

- `src/lib.rs` — macro exports and shared expansion helpers.
- `src/cauldron.rs` — cauldron declarations and registration callbacks.
- `src/managed.rs` and `src/provider.rs` — managed type/function expansion and
  dependency extraction.
- `src/configuration/` — `Configuration` derive parsing and validation.
- `src/main.rs` — async entry-point expansion.
- `src/path.rs` — crate-path and generated-path resolution.

Keep syntax diagnostics close to the offending declaration token. Keep semantic
rules that require catalog state, configuration, or graph context in
`furnace-rs-core`.

## Tests

Run the focused macro crate tests with:

~~~sh
cargo test -p furnace-rs-core-macros
~~~

Consumer-facing compile-fail and compile-pass behavior is tested through the
trybuild fixtures under `crates/furnace-rs/tests/ui`, because the generated code must
be checked from the perspective of an external application. Update the
corresponding fixture output when an intentional diagnostic changes.

See the [workspace crate map](../../README.md#workspace-crates) and
[core guide](../furnace-rs-core/README.md) for the runtime side of each macro.
