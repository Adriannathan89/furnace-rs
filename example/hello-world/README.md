# Hello World

This is the smallest FURNACE 1.0.2 HTTP application: a controller with inherent endpoints,
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

`furnace.toml` sets the listener address and enables debug request logging in
this checkout. The annotated controller implementation declares its endpoints.
Controllers without `impl Sealable` are public, and `.controller::<HelloController>()`
registers them in the root `Cauldron`. `Furnace::burn` loads local configuration and
starts the server.

Continue with [posts-crud](../posts-crud/) for persistence or
[protected-route](../protected-route/) for authentication and logging.
