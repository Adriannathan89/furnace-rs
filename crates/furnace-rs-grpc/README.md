# furnace-rs-grpc

Standalone protobuf serving and named generated clients for Furnace, with native
Tonic requests, responses, statuses, and streams. Optional features are `jwt` for
shared Passport admission and `tls` for verified TLS. The facade maps these to
`grpc`, shared `jwt`, and `grpc-tls`.

Applications own their `.proto` code generation and descriptor bytes. Register
concrete trait implementations with `#[grpc_service]` and `grpc_services` in a
cauldron. Register named wrappers with `#[grpc_client]`. Import `FurnaceBurnExt`
for configuration-selected automatic startup, or compose raw native routes with
`RuntimeRoutes`, `RuntimeBindings`, `RuntimeGrpcExt`, and `serve_transports`.

See the [transport guide](../../docs/grpc.md) and
[independent example](../../example/grpc-transport/) for all four RPC shapes,
Passport/JWT, per-call credentials, deadlines, cancellation, health/reflection,
TLS roots, shared lifecycle, and static CLI inspection.
