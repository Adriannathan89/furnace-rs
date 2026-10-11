# Missing dependency blocked at compile time

This independent FURNACE 1.1.0-beta.1 example intentionally fails compilation.
`OrderService` injects `OrderRepository`, but `AppCauldron` registers only
`OrderService`. Requires Rust 1.94 or newer.

From the repository root:

```sh
cargo check --manifest-path example/missing-dependency/Cargo.toml
```

Expect a nonzero exit and an E0277 diagnostic containing
`Lookup<OrderRepository, _>` at the marked root. `cargo test` and `cargo run`
also fail during compilation. Nothing starts: `main` is empty and the checker
runs without calling `Furnace::burn` or `build`.

Fix the root declaration in `src/main.rs` by registering the missing provider:

```rust
#[cauldron(root, providers = [OrderService, OrderRepository])]
struct AppCauldron;
```

Run the same command again; it now compiles. The repository is a valid injectable
type in both versions; the missing registration causes the error.

This crate has its own workspace, so its intentional failure does not break the
repository's workspace tests. For a valid application with a compile-fail test
that passes when a dependency is rejected, see [compile-time-di](../compile-time-di/).
