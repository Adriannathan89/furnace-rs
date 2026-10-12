# Declarative dependency injection

The `2.0.0-beta.1` API registers modules and checks a selected root graph at compile time. The executable [example](../example/compile-time-di/) includes a missing-dependency test run by `cargo test`.

```rust
use furnace_rs::{burner, cauldron, Logger, LoggerCauldron};
#[burner]
struct Service { #[inject] logger: Logger }
#[cauldron(providers = [Service], exports = [Service])]
struct Feature;
#[cauldron(root, imports = [Feature, LoggerCauldron])]
struct App;
```

`root` checks every registered constructor's `Injector::Dependencies`, including constructors in imported modules. Checking happens even if the program never calls `build` or `burn`. Every named dependency field on a burner, storage or managed controller must have `#[inject]`. The marker takes no arguments and is consumed by the managed macro; an unmarked field is a compilation error. Unit structs need no marker. Manual `Injector` implementations declare dependencies through `Injector::Dependencies` and do not use field markers.

Options are `providers = [Service, Output => NativeInjector]`, `controllers = [Controller]`, `imports = [Module]`, `exports = [Output]`, `global`, and `root`. Providers without an explicit injector use `Injector<Self>`. Native bindings use `Injector<Output>`. Controllers require typed `Injector` metadata. Exports must be local providers; controllers and re-exporting an imported output do not qualify. Duplicate options and authored list entries are rejected.

Local providers/controllers, exported outputs of direct imports, exported outputs of reachable globals, and intrinsic `Config` form each module's scope. An indirect non-global export is invisible. Linked globals and another application's imports cannot satisfy the selected root. Diamonds and direct/global overlap share one owner. Private source types and constructors can stay private; opaque generated markers carry metadata across crates. Rust types, rather than type-name strings, decide dependency membership.

Bare `#[cauldron]` and `#[cauldron()]` keep manual `Cauldron::register` behavior. The fluent API and builder stay available. Empty declarative modules use `#[cauldron(providers = [])]`. Declarative imports require declarative module metadata, support qualified paths, re-exports and `use Module as Alias`, and exclude module `type` aliases in 2.0.0-beta.1. Dependency/output type aliases continue working. A marked root cannot be imported by another marked root. Declarative import cycles may fail during metadata expansion; legacy runtime cycle diagnostics stay available. Compiler expansion limits still apply to large graphs.

Native values supplied with `builder.provide(value)` retain runtime precedence and lifecycle behavior, but cannot erase static requirements: all declared constructor dependencies must still be present. Arbitrary runtime auto-configuration outputs are not implicit static capabilities. Declare an injector/output binding when a checked constructor needs one.

Compile-time checking covers declared availability and scope. Runtime preflight still validates ownership, duplicate outputs, cycles, HTTP metadata and configuration. Constructors, resource readiness, database credentials, network binding and lifecycle can still fail at runtime. Keep handling their `Result`s.

## Database and logger

With `furnace-rs-persistence/sea-orm-postgres` and the facade's `logger` feature:

```rust
use furnace_rs::{burner, cauldron, Logger, LoggerCauldron};
use furnace_rs_persistence::sea_orm::{DatabaseCauldron, DatabaseConnection};
#[burner]
struct Service {
    #[inject] database: DatabaseConnection,
    #[inject] logger: Logger,
}
#[cauldron(root, providers = [Service], imports = [DatabaseCauldron, LoggerCauldron])]
struct App;
```

DatabaseCauldron exports the native SeaORM connection and keeps the factory/connector private to its module. Its connector requires intrinsic Config and valid `persistence.seaorm` settings at runtime. LoggerCauldron exports the default Logger. Both remain global, work with legacy roots, and accept the existing native overrides. The ordinary compile tests use SeaORM mocks and do not need PostgreSQL.
