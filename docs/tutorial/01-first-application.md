# 1. First application

[Learning path](README.md) · Next: [Providers and cauldrons](02-providers-and-cauldrons.md)

The smallest application has three parts: a controller that answers a request,
a cauldron that registers it, and an entry point that starts the server.

## The program

`src/main.rs`:

```rust
use furnace_rs::prelude::*;

#[controller]
struct HelloController;

#[controller(route = "/")]
impl HelloController {
    #[get]
    async fn hello(&self) -> &'static str {
        "Hello, world!"
    }
}

#[cauldron(root, controllers = [HelloController])]
struct AppCauldron;

#[furnace_rs::main]
async fn main() -> Result<(), HttpRuntimeError> {
    Furnace::burn::<AppCauldron>().await
}
```

`furnace.toml`, next to `Cargo.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000
```

## Run it

```sh
cargo run
```

In another terminal:

```sh
curl http://127.0.0.1:3000/
# Hello, world!
```

## What each part does

| Code | Role |
| --- | --- |
| `#[controller]` on the struct | Declares a controller. Its fields, when it has any, are injected dependencies. |
| `#[controller(route = "/")]` on the `impl` | Sets the path prefix for the endpoints inside. |
| `#[get]` | Maps `GET` on the prefix to the method. `#[post]`, `#[put]`, `#[patch]`, and `#[delete]` work the same way, and each accepts a sub-path such as `#[get("/{id}")]`. |
| `#[cauldron(root, controllers = [...])]` | Registers the controller. `root` marks this cauldron as the application entry and checks its dependencies when the crate compiles. |
| `Furnace::burn::<AppCauldron>()` | Loads configuration, builds every registered provider, and serves HTTP until shutdown. |

## Configuration is read from the working directory

`Furnace::burn` reads, in this order, an optional `.env` (used only to fill
`${NAME}` placeholders), an optional `furnace.toml`, and `FURNACE_*`
environment variables, which override the file. Run the program from the
directory that holds `furnace.toml`.

Override a setting for one run without editing the file:

```sh
FURNACE_SERVER__PORT=4000 cargo run
```

To print one line per request while developing, add:

```toml
[furnace]
mode = "debug"
```

## Try it

- Add a second endpoint, `#[get("/health")]`, returning `"ok"`.
- Change the route prefix to `/hello` and request the new path.
- Run with `FURNACE_FURNACE__MODE=debug` and watch a line appear per request.

The same program is available as the runnable
[hello-world example](../../example/hello-world/).

Next: [Providers and cauldrons](02-providers-and-cauldrons.md)
