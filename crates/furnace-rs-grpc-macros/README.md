# furnace-rs-grpc-macros

This procedural-macro crate implements the registration attributes used by
[`furnace-rs-grpc`](../furnace-rs-grpc/README.md): `#[grpc_service]`,
`#[grpc_client]`, and `#[grpc_reflection]`. Applications normally import them
from the transport crate or the `furnace-rs` facade rather than depending on
this implementation crate directly.

The macros connect application-generated Tonic types to Furnace metadata and
DI registration. They do not compile `.proto` files or generate protobuf
service/client code. The application build owns that step. `proc-macro-crate`
resolves renamed dependencies so generated paths work for standalone and facade
consumers.

## Source layout

- [`src/grpc_service.rs`](src/grpc_service.rs) — service implementation and
  method-policy parsing/expansion.
- [`src/grpc_client.rs`](src/grpc_client.rs) — named generated-client wrapper
  and provider expansion.
- [`src/grpc_reflection.rs`](src/grpc_reflection.rs) — root-scoped reflection
  policy metadata.
- [`src/path.rs`](src/path.rs) — dependency identity and generated path
  resolution.
- [`src/lib.rs`](src/lib.rs) — proc-macro entry points and diagnostics.

The crate has no runtime feature flags. Macro expansion contracts are exercised
through the gRPC and facade consumer/UI tests; run them with:

```sh
cargo test -p furnace-rs-grpc --all-features
cargo test -p furnace-rs --all-features --test grpc_ui
```

See the [gRPC guide](../../docs/grpc.md) for the supported attributes and
application-facing examples.
