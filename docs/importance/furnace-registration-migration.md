# Migrating to explicit cauldron registration

This breaking change replaces namespace-owned cauldrons with explicit cauldron membership.
No compatibility aliases are provided for the old attributes or standard startup method.

| Before | After |
| --- | --- |
| `#[module]` | `#[cauldron]` |
| `#[service]` | `#[burner]` |
| `#[repository]` | `#[storage]` |
| `#[provider]` | `#[element]` |
| `#[provider(lifecycle)]` | `#[element(lifecycle)]` |
| `Cauldron` | `Cauldron` |
| `FurnaceRunExt` / `Furnace::run` | `FurnaceBurnExt` / `Furnace::burn` |
| Attribute `imports` / `global` | Chain `.import(...)` / `.global()` |

Keep application type names such as `UserCauldron`, `UserService`, `LoggerCauldron`, and `DatabaseCauldron`.

## Register the complete cauldron

Previously, Rust namespace placement implicitly selected dependencies:

```rust,ignore
#[module(imports = [DatabaseCauldron])]
struct UserCauldron;
#[service]
struct UserService { repository: UserRepository }
#[repository]
struct UserRepository { database: DatabaseConnection }
```

Now record every provider output and controller explicitly:

```rust,ignore
use furnace-rs::prelude::*;
use furnace_rs_persistence::sea_orm::{DatabaseConnection, DatabaseCauldron};

#[burner]
struct UserService { repository: UserRepository }
#[storage]
struct UserRepository { database: DatabaseConnection }

#[cauldron]
struct UserCauldron;
impl Cauldron for UserCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<UserRepository>()
            .provide::<UserService>()
            .controller::<UserController>()
            .import(DatabaseCauldron)
            .export::<UserService>()
    }
}
```

Generic chaining names the dependency type without constructing it. Registration performs no
I/O and must describe a stable topology. Empty furnaces return `CauldronRegistration::new(self)`.

## Register element outputs

Element factories remain ordinary callable functions and support sync, async, fallible,
and lifecycle-resource construction. Register the resulting output, not the factory function:

```rust,ignore
#[element]
fn client(config: Config) -> furnace-rs::core::Result<Client> { Client::from_config(config) }

#[element]
fn account_service(service: AccountServiceImpl) -> std::sync::Arc<dyn AccountService> {
    std::sync::Arc::new(service)
}

// In register:
self.provide::<Client>()
    .provide::<AccountServiceImpl>()
    .provide::<std::sync::Arc<dyn AccountService>>()
```

For `Result<T>` or `Result<LifecycleResource<T>>`, register `T`. Multiple factories with the
same normalized output are ambiguous; selecting factories by function name is not supported.

## Share dependencies deliberately

Rust `pub` makes a name usable by Rust code; `.export::<T>()` makes a registered output
injectable by other furnaces. Imports expose only the directly imported cauldron's exports.
Transitive imports do not expose transitive dependencies, and imported outputs cannot be
re-exported. Controllers contribute routes through `.controller::<T>()`, not exports.

Register a shared output once in its owning cauldron, then import that cauldron wherever needed.
Registering the same output in two reachable furnaces is an error. Independent application roots
may reuse declarations without sharing registration state or provider instances.

A global cauldron exposes only explicit exports and must be reachable from the application's root:

```rust,ignore
#[cauldron]
struct ConfigCauldron;
impl Cauldron for ConfigCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.provide::<AppConfig>().export::<AppConfig>().global()
    }
}
#[cauldron]
struct AppCauldron;
impl Cauldron for AppCauldron {
    fn register(self) -> CauldronRegistration<Self> {
        self.import(ConfigCauldron).import(UserCauldron)
    }
}
```

`LoggerCauldron` globally exports `Logger`. `DatabaseCauldron` globally exports `DatabaseConnection`,
while keeping its factory and connector private. Importing neither leaves them inactive.

## Start the application

```rust,ignore
#[furnace-rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

Configuration precedence, listener defaults, CORS, lifecycle hooks, shutdown, and CLI inspection
remain unchanged. The CLI command is still `furnace-rs run`. Core-only apps use the low-level builder.
Rooted supplied-value overrides must target registered outputs and preserve their owner's
visibility. Focused test fixtures and unrooted complete-catalog builders retain their separate modes.
