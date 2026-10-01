# Hello World

This is the smallest FURNACE 0.9 HTTP application: a route trait, its controller,
an application module, and the conventional `Furnace::burn` entry point.

Requires Rust 1.94 or newer. From this directory:

```sh
cargo run
```

In another terminal:

```sh
curl http://127.0.0.1:3000/
# Hello, world!
```

`furnace.toml` sets the listener address. The `#[routes]` trait declares the HTTP
contract; `#[controller]` implements it; `#[cauldron]` makes the controller
reachable from the application root. `Furnace::burn` loads local configuration and
starts the server.

Continue with [posts-crud](../posts-crud/) for persistence or
[protected-route](../protected-route/) for authentication and logging.
