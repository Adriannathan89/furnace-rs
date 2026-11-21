# 2. Providers and cauldrons

Previous: [First application](01-first-application.md) · [Learning path](README.md) · Next: [Configuration and validation](03-configuration-and-validation.md)

A controller should translate HTTP and leave the work to something else. This
step moves the work into a service, hides the implementation behind a trait,
and groups both in a feature cauldron.

## The program

Replace `src/main.rs`:

```rust
use std::sync::Arc;

use furnace_rs::prelude::*;

// 1. A contract the rest of the application depends on.
trait Greeter: Send + Sync {
    fn greet(&self, name: &str) -> String;
}

// 2. One implementation, bound to the contract by its `Injector`.
struct EnglishGreeter;

impl Greeter for EnglishGreeter {
    fn greet(&self, name: &str) -> String {
        format!("Hello, {name}!")
    }
}

impl Injector<Arc<dyn Greeter>> for EnglishGreeter {
    type Dependencies = ();

    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<Arc<dyn Greeter>> {
        Ok(Arc::new(Self))
    }
}

// 3. A managed service: `#[burner]` generates its constructor from `#[inject]`.
#[burner]
struct GreetingService {
    #[inject]
    greeter: Arc<dyn Greeter>,
    #[inject]
    logger: Logger,
}

impl GreetingService {
    fn greet(&self, name: &str) -> String {
        self.logger.info("greeting requested");
        self.greeter.greet(name)
    }
}

// 4. A feature cauldron lists its providers and exports what others may use.
#[cauldron(
    providers = [Arc<dyn Greeter> => EnglishGreeter, GreetingService],
    exports = [GreetingService],
)]
struct GreetingCauldron;

#[controller]
struct GreetingController {
    #[inject]
    greetings: GreetingService,
}

#[controller(route = "/greetings")]
impl GreetingController {
    #[get("/{name}")]
    async fn greet(&self, Path(name): Path<String>) -> String {
        self.greetings.greet(&name)
    }
}

// 5. The root imports features; `root` checks the whole graph at compile time.
#[cauldron(
    root,
    imports = [GreetingCauldron, LoggerCauldron],
    controllers = [GreetingController],
)]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

```sh
cargo run
curl http://127.0.0.1:3000/greetings/Ada
# Hello, Ada!
```

## Three ways to write a provider

| You have | Write | Used above for |
| --- | --- | --- |
| A struct whose fields are other providers | `#[burner]` (services) or `#[storage]` (repositories), with `#[inject]` on each field | `GreetingService` |
| A type you construct by hand | `impl Injector for T` with `Dependencies` and an async `inject` | — |
| An implementation of a trait, or a third-party type | `impl Injector<Output> for T`, registered as `Output => T` | `Arc<dyn Greeter> => EnglishGreeter` |

`Dependencies` is a tuple of the providers the constructor needs, and `inject`
receives them in that order. `()` means none.

## Rules for cauldrons

- **Registration is explicit.** A provider exists only if some reachable
  cauldron lists it. Rust modules and `pub` do not affect this.
- **Imports do not leak.** `AppCauldron` can inject `GreetingService` because
  `GreetingCauldron` exports it. `Arc<dyn Greeter>` is not exported, so it stays
  private to the feature.
- **`global` shares exports everywhere.** `LoggerCauldron` is global: importing
  it once at the root makes `Logger` injectable in every cauldron, which is why
  `GreetingCauldron` does not import it.
- **One root.** The cauldron marked `root` is checked as a whole when the crate
  compiles.

## See a missing dependency fail the build

Delete `GreetingService` from the `providers` list and run `cargo check`. The
build stops with an `E0277` error about an unsatisfied `Lookup` bound, pointing
at `AppCauldron`, before anything runs. The
[missing-dependency example](../../example/missing-dependency/) shows this
error on its own, and [compile-time-di](../../example/compile-time-di/) turns it
into a passing test.

## Try it

- Add a `SpanishGreeter` and switch the binding to
  `Arc<dyn Greeter> => SpanishGreeter`. Nothing else changes.
- Remove `GreetingService` from `exports` and read the compiler error.
- Run `furnace doctor` and `furnace routes` to inspect the application and its
  route table (see the [CLI guide](../guides/CLI.md)).

Reference: [Declarative DI](../guides/declarative-di.md) ·
[Architecture](../guides/ARCHITECTURE.md)

Next: [Configuration and validation](03-configuration-and-validation.md)
