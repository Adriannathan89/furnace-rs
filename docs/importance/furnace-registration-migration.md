# Migrating to explicit furnace registration

This breaking change replaces namespace-owned modules with explicit furnace membership.
No compatibility aliases are provided for the old attributes or standard startup method.

| Before | After |
| --- | --- |
| `#[module]` | `#[furnace]` |
| `#[service]` | `#[burner]` |
| `#[repository]` | `#[storage]` |
| `#[provider]` | `#[element]` |
| `#[provider(lifecycle)]` | `#[element(lifecycle)]` |
| `Module` | `Furnace` |
| `MadsRunExt` / `Mads::run` | `MadsBurnExt` / `Mads::burn` |
| Attribute `imports` / `global` | Chain `.import(...)` / `.global()` |

Keep application type names such as `UserModule`, `UserService`, `LoggerModule`, and `DatabaseModule`.

## Register the complete furnace

Previously, Rust namespace placement implicitly selected dependencies:

```rust,ignore
#[module(imports = [DatabaseModule])]
struct UserModule;
#[service]
struct UserService { repository: UserRepository }
#[repository]
struct UserRepository { database: DatabaseConnection }
```

Now record every provider output and controller explicitly:

```rust,ignore
use mads::prelude::*;
use mads_persistence::sea_orm::{DatabaseConnection, DatabaseModule};

#[burner]
struct UserService { repository: UserRepository }
#[storage]
struct UserRepository { database: DatabaseConnection }

#[furnace]
struct UserModule;
impl Furnace for UserModule {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<UserRepository>()
            .provide::<UserService>()
            .controller::<UserController>()
            .import(DatabaseModule)
            .export::<UserService>()
    }
}
```

Generic chaining names the dependency type without constructing it. Registration performs no
I/O and must describe a stable topology. Empty furnaces return `FurnaceRegistration::new(self)`.

## Register element outputs

Element factories remain ordinary callable functions and support sync, async, fallible,
and lifecycle-resource construction. Register the resulting output, not the factory function:

```rust,ignore
#[element]
fn client(config: Config) -> mads::core::Result<Client> { Client::from_config(config) }

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
injectable by other furnaces. Imports expose only the directly imported furnace's exports.
Transitive imports do not expose transitive dependencies, and imported outputs cannot be
re-exported. Controllers contribute routes through `.controller::<T>()`, not exports.

Register a shared output once in its owning furnace, then import that furnace wherever needed.
Registering the same output in two reachable furnaces is an error. Independent application roots
may reuse declarations without sharing registration state or provider instances.

A global furnace exposes only explicit exports and must be reachable from the application's root:

```rust,ignore
#[furnace]
struct ConfigModule;
impl Furnace for ConfigModule {
    fn register(self) -> FurnaceRegistration<Self> {
        self.provide::<AppConfig>().export::<AppConfig>().global()
    }
}
#[furnace]
struct AppModule;
impl Furnace for AppModule {
    fn register(self) -> FurnaceRegistration<Self> {
        self.import(ConfigModule).import(UserModule)
    }
}
```

`LoggerModule` globally exports `Logger`. `DatabaseModule` globally exports `DatabaseConnection`,
while keeping its factory and connector private. Importing neither leaves them inactive.

## Start the application

```rust,ignore
#[mads::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Mads::burn::<AppModule>().await
}
```

Configuration precedence, listener defaults, CORS, lifecycle hooks, shutdown, and CLI inspection
remain unchanged. The CLI command is still `mads run`. Core-only apps use the low-level builder.
Rooted supplied-value overrides must target registered outputs and preserve their owner's
visibility. Focused test fixtures and unrooted complete-catalog builders retain their separate modes.
