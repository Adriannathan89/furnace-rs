# Learning path

Five short steps take you from an empty project to a tested application with
registration, password hashing, and JWT-protected routes. Each step adds one
idea and ends with a program you can run. Read them in order the first time;
afterwards, use the [topic guides](../README.md#topic-guides) as reference.

| Step | You build | Ideas introduced |
| --- | --- | --- |
| [1. First application](01-first-application.md) | An HTTP server answering `GET /` | Controller, cauldron, `Furnace::burn`, `furnace.toml` |
| [2. Providers and cauldrons](02-providers-and-cauldrons.md) | A greeting feature split into its own cauldron | `#[burner]`, `#[inject]`, `Injector`, trait bindings, imports and exports |
| [3. Configuration and validation](03-configuration-and-validation.md) | A configurable endpoint that rejects bad input | `#[derive(Configuration)]`, `Secret`, `#[derive(Input)]`, `ValidatedJson`, error envelope |
| [4. Accounts and login](04-accounts-and-login.md) | Registration, login, and a protected profile route | `PasswordHasher`, `JwtService`, Passport strategy, `#[guard]`, `Sealable` |
| [5. Testing](05-testing.md) | In-process tests for a service and a controller | `#[furnace_rs::test]`, fixtures, supplies, response assertions |

## Before you start

- Rust 1.94 or newer.
- A new binary crate (`cargo new my-app`), or the starter from the
  [CLI](../guides/CLI.md) (`furnace new my-app`).
- This dependency block; later steps say when to add a feature:

  ```toml
  [dependencies]
  furnace-rs = "=2.0.0"
  serde = { version = "1", features = ["derive"] }
  ```

Every listing in steps 1 to 3 is a complete `src/main.rs`. Steps 4 and 5 walk
through the runnable [user-accounts example](../../example/user-accounts/),
which is split into modules the way a real feature would be.

## The vocabulary in one minute

| Term | Meaning |
| --- | --- |
| **Provider** | A value the framework constructs and hands to whatever asks for it: a service, a repository, a configuration struct. |
| **`Injector`** | The trait that says how a provider is built and what it depends on. `#[burner]` and `#[storage]` generate it. |
| **Controller** | A provider whose annotated methods are HTTP endpoints. |
| **Cauldron** | A named list of providers and controllers, plus the cauldrons it imports. Similar to a module in other frameworks. |
| **Root** | The cauldron passed to `Furnace::burn`. Marking it `root` checks the whole dependency graph at compile time. |
| **Guard** | A static authentication and authorization policy attached to a controller. |
