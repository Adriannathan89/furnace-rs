# Hello World

This is the smallest FURNACE 2.0.0-beta.3 HTTP application: a controller with inherent endpoints,
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

`furnace.toml` sets the listener address. Request logging stays disabled unless
`furnace.mode = "debug"` is configured. The annotated controller implementation
declares its endpoints.
Controllers without `impl Sealable` are public.
`#[cauldron(root, controllers = [HelloController])]` registers the controller
and checks its constructor dependencies at compile time. `Furnace::burn` loads local configuration and
starts the server.

Continue with [posts-crud](../posts-crud/) for persistence or
[protected-route](../protected-route/) for authentication.
