# furnace-bench

Unpublished Rust CLI for FURNACE's HTTP performance benchmarks. Uses Tokio and
reqwest with HTTP/1.1, persistent connections, response validation, and explicit
failure accounting. It is a standalone Cargo workspace; framework crates do not
depend on this tool.

```sh
cargo build --release --locked --manifest-path benchmark/runner/Cargo.toml
benchmark/runner/target/release/furnace-bench build
benchmark/runner/target/release/furnace-bench compare --profile extended --runs 5 \
  --seed 20261016 --output /tmp/furnace-comparison.json
cargo test --locked --manifest-path benchmark/runner/Cargo.toml
```

Run from the repository root. The default comparison needs disposable
PostgreSQL databases; select `--scenario hello --scenario auth` to omit CRUD.
Use `--help` for options. Loopback sockets and subprocesses must be permitted
for integration tests and benchmarks.

See the [benchmark guide](../README.md) for the documentation index and the
[performance report](../PERFORMANCE.md) for results, historical comparisons,
and measurement limits.
