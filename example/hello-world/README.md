# Hello World

This is the smallest MADS 0.9 HTTP application: a route trait, its controller,
an application module, and the conventional `Mads::burn` entry point.

Requires Rust 1.94 or newer. From this directory:

```sh
cargo run
```

In another terminal:

```sh
curl http://127.0.0.1:3000/
# Hello, world!
```

`mads.toml` sets the listener address. The `#[routes]` trait declares the HTTP
contract; `#[controller]` implements it; `#[furnace]` makes the controller
reachable from the application root. `Mads::burn` loads local configuration and
starts the server.

Continue with [posts-crud](../posts-crud/) for persistence or
[protected-route](../protected-route/) for authentication and logging.
