# furnace-rs-grpc

`furnace-rs-grpc` is the standalone protobuf gRPC serving and client crate for
Furnace. It integrates application-owned Prost/Tonic code with Furnace provider
registration, selected-cauldron validation, startup, and shutdown. The crate
re-exports Tonic and its registration attributes so applications can use their
generated service and client types directly.

## Features and dependency boundary

The crate has no default features. Its base dependencies include Tonic's
transport/router/codegen support and the shared Furnace core and transport
runtime contracts. Optional features are:

| Feature | Effect |
| --- | --- |
| `jwt` | Enables shared Passport admission for guarded RPCs. |
| `tls` | Enables verified client and server TLS configuration. |

The `furnace-rs` facade exposes this functionality through `grpc`, `jwt`, and
`grpc-tls`. The standalone crate does not enable Furnace HTTP routing. For
combined HTTP and gRPC applications, select both facade transport features so
the shared runtime owns one application lifecycle.

Applications own `.proto` files, code generation, and descriptor-set bytes.
`#[grpc_service]` registers a concrete implementation of an application-
generated server; `#[grpc_client]` provides a named DI wrapper around a
generated client. The crate adds registration and transport behavior without
replacing Tonic's request, response, status, or streaming APIs.

## Runtime and security

`FurnaceBurnExt` selects configured transports automatically. For native route
composition, use `RuntimeRoutes`, `RuntimeBindings`, `RuntimeGrpcExt`, and
`serve_transports`. Both paths coordinate readiness, listener binding,
admission, bounded drain, and provider shutdown.

The `jwt` feature applies scoped Bearer authorization at RPC admission. The
`tls` feature validates server identities and client trust/hostnames. Reflection
is opt-in and uses a root-scoped guard; public health is installed with the
managed gRPC infrastructure. Client calls support per-call credentials,
deadlines, and native Tonic cancellation behavior.

## Source layout

- [`src/grpc`](src/grpc/) — service/client descriptors, scope validation,
  preflight, routing, Passport, TLS, health, and reflection.
- [`src/runtime`](src/runtime/) — transport selection, inspection, runtime
  adaptation, and the supervised gRPC server.
- [`src/lib.rs`](src/lib.rs) — crate re-exports and feature-gated Passport API.

## Tests

Run the gRPC integration suite with:

```sh
cargo test -p furnace-rs-grpc --all-features
```

It covers service registration and dispatch, all RPC shapes, client deadlines
and credentials, Passport admission, TLS verification, health/reflection, and
shared runtime behavior. Facade feature combinations and macro compile-fail
contracts are exercised by `cargo test -p furnace-rs --all-features`.

See the [transport guide](../../docs/grpc.md) for configuration and user-facing
examples, and the [independent example](../../example/grpc-transport/) for
server, client, mixed HTTP/gRPC, and TLS runs.
