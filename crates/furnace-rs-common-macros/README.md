# furnace-rs-common-macros

`furnace-rs-common-macros` contains the procedural macros for the integration layer
implemented by [furnace-rs-common](../furnace-rs-common/README.md). It generates route
metadata, typed Axum registration adapters, validation traversal, and
Passport metadata.

Application authors should import these macros through `furnace_rs::prelude` or
`furnace-rs-common` re-exports. This crate is not intended to be a direct
application dependency.

## Macros provided

| Macro | Generated contract |
| --- | --- |
| `#[get]`, `#[post]`, `#[put]`, `#[patch]`, `#[delete]` | Adds a typed route method declaration and path metadata. |
| `#[controller]` | Declares managed controller metadata and typed inherent endpoint adapters. |
| `#[guard]` | Declares a complete unit-struct policy activated by a selected controller seal. |
| `#[derive(Input)]` | Generates deterministic, transport-independent validation traversal. |
| `#[derive(PassportPrincipal)]` | Generates role/permission accessors for a typed principal. |
| `#[passport_strategy]` | Registers a managed Passport strategy adapter and static strategy metadata. |

`#[controller]` struct and inherent implementation declarations keep handler dispatch typed. Handler names in
metadata are used for diagnostics and inspection, not string-based runtime
dispatch. The generated route adapter resolves a controller once from the core
application context.

`#[derive(Input)]` runs after Serde representation conversion at the validated
extractor boundary. It generates ordered issues for supported fields, nested
values, collections, and maps; it does not perform I/O or async validation.

Guard and Passport macros emit static policy information and typed adapters.
Runtime verification, strategy validation, static controller seals, and safe HTTP
failure mapping belong to `furnace-rs-common`.

## Features and dependencies

This is a `proc-macro` crate with no first-party runtime dependency.

| Feature | Meaning |
| --- | --- |
| `passport` | Enables the Passport strategy macro implementation. |
| `cookies` | Enables cookie token sources; Passport remains separately gated. |

Direct dependencies:

- `proc-macro-crate` for resolving the downstream integration crate.
- `proc-macro2` for token streams.
- `quote` for generated code.
- `syn` with visitor support for syntax and type traversal.

Only `furnace-rs-common` depends on this crate. Its macros are re-exported through
the facade when the relevant public features are enabled.

## Source layout

- `src/lib.rs` — public macro exports and shared expansion helpers.
- `src/endpoint.rs` and `src/verb.rs` — endpoint parsing, paths, and typed adapter helpers.
- `src/controller.rs` — managed struct and inherent endpoint expansion.
- `src/input/` — validation attributes, Serde paths, and traversal checks.
- `src/guard.rs`, `src/passport_principal.rs`, and
  `src/passport_strategy.rs` — guard and authentication declarations.
- `src/path.rs` — path normalization and generated-path handling.

The macro crate owns declaration syntax and generated code. Keep runtime
selection, graph validation, and error policy in `furnace-rs-common`.

## Tests

Run focused tests with:

~~~sh
cargo test -p furnace-rs-common-macros
~~~

Consumer-facing compile-fail and compile-pass fixtures live under
`crates/furnace-rs/tests/ui`. When a macro expansion changes, test both the syntax
diagnostic and the generated consumer behavior. Follow the feature matrix in
the [common guide](../furnace-rs-common/README.md) before changing feature gates.
