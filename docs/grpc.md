# Protobuf gRPC transport

FURNACE 2.0.0-beta.2 adds gRPC serving and consumption for all four RPC shapes. The
standalone `furnace-rs-grpc` package provides gRPC by default, with optional `jwt`
and `tls` features. `furnace-rs-grpc-macros` supplies its declaration attributes.
The `furnace-rs` facade exposes `grpc`, `grpc-tls`, and shared `jwt` features.
See the [2.0.0-beta.2 release notes](releases/2.0.0-beta.2.md) for the feature summary.

Applications own their `.proto` files, generated Tonic client/server types, and
encoded file descriptors. Furnace performs no runtime protobuf compilation.
The [independent example](../example/grpc-transport/) uses Tonic 0.14.6, Prost
0.14.4, and a vendored build-time compiler. It also declares the normal `tonic`,
`tonic-prost`, and `prost` dependencies needed by its generated code.

## Managed services

Declare an injector for the implementation, then attach registration to its
concrete generated service trait implementation:

```rust,ignore
#[burner]
struct Transport;

#[grpc_service(
    server = pb::transport_server::TransportServer,
    descriptor = FILE_DESCRIPTOR_SET,
    guard = Reader,
)]
#[tonic::async_trait]
impl pb::transport_server::Transport for Transport {
    #[grpc_method(name = "Unary", public)]
    async fn unary(&self, request: tonic::Request<pb::Item>)
        -> Result<tonic::Response<pb::Item>, tonic::Status>
    {
        Ok(tonic::Response::new(request.into_inner()))
    }
    // Implement the other methods and associated stream types from the proto.
}

#[cauldron(root, grpc_services = [Transport])]
struct App;
```

The registration attribute does not generate protobuf code or a constructor.
Use `#[burner]` or a manual `Injector`. `grpc_method` is a helper consumed inside
`grpc_service`; names refer to authored protobuf method names. `public` overrides
a service guard, while `guard = AnotherGuard` replaces the service policy for
that method. Declare guards with the shared `#[guard]` attribute.

Cauldron ownership, imports, exports, and compile-time DI checks apply to gRPC
implementations. Register ordinary dependencies in `providers`; a constructor
that injects `JwtService` must explicitly declare that provider, as in the example.
Native values supplied with `builder.provide(...)` retain precedence and do not
erase static constructor requirements. Selected metadata and policy bindings
are checked before construction, including full method coverage between the
descriptor and authored generated-trait implementation. Missing operation policy
metadata never dispatches a managed RPC. Each service is resolved once into native routes.

## Runtime selection and manual composition

Import `FurnaceBurnExt` (included in the facade prelude) and call
`Furnace::burn::<App>().await`. Conventional source loading uses optional `.env`
for exact placeholders, optional `furnace.toml`, then `FURNACE_*` environment
overrides. Nested environment keys use double underscores.

```toml
[furnace]
transports = ["grpc"] # Or ["http", "grpc"] in a build enabling both.

[grpc.server]
host = "127.0.0.1"
port = 50051
max_encoding_message_bytes = 4194304
max_decoding_message_bytes = 4194304
tls_handshake_timeout_ms = 10000
shutdown_timeout_ms = 30000
```

Missing transport selection remains HTTP-only when HTTP is compiled. Selection
must be nonempty, unique, and limited to compiled transports. Automatic startup
requires managed operations for each selected listener. HTTP `[server]` settings
and its port-3000 defaults retain their existing meaning.

One coordinator owns both listeners and the application lifecycle. Eager clients
must become ready before binding. All selected listeners bind before admission
and readiness output. A bind failure releases acquired listeners and shuts down
started providers. Shutdown publishes not-serving health, stops admission, drains
active calls, then cancels and joins runtime-owned connection/protocol tasks at
the gRPC deadline before stopping providers. Provider cleanup is still awaited.

`ApplicationRuntimeError` retains original failures through `Error::source()` and
combines operation and shutdown errors. `HttpRuntimeError` remains its HTTP
compatibility alias. Protocol-neutral `TransportBind` and `TransportServe` variants
identify the owning transport. `DrainTimeout` reports forced drain expiry.

For a manually built application, compose raw native routes before starting it:

```rust,ignore
use furnace::prelude::*;

let routes = RuntimeRoutes::new()
    .with_http(build_router(&application)?.merge(native_http))
    .with_grpc(build_grpc_routes(&application)?.add_service(native_grpc));
let bindings = RuntimeBindings::new()
    .with_http(http_address)
    .with_grpc(grpc_config.with_address(grpc_address));
serve_transports(application, routes, bindings, shutdown_future).await?;
```

`RuntimeGrpcExt` supplies `with_grpc` on the shared builders and is exported in the
facade prelude. Omit HTTP composition for gRPC-only builds. Explicit addresses may
use port zero. The helper configures raw HTTP routes once, adds native health and
configured reflection to gRPC, and owns one lifecycle. Existing low-level HTTP
`serve` and `serve_router` remain available. The HTTP connection server retains
its initial/header/body-idle read deadlines and trusted socket peer addresses.

## Named outbound clients

```rust,ignore
#[grpc_client(
    name = "remote",
    client = pb::transport_client::TransportClient,
    token_provider = Tokens,
)]
struct Remote;
```

Register the wrapper and optional provider in `providers`. The injectable wrapper
returns a cheap native generated client via `client()`. Multiple wrappers may
share the same generated generic client type while using distinct named settings.
Cloned channels allow concurrent calls without serializing RPCs through one mutex.

```toml
[grpc.clients.remote]
endpoint = "http://127.0.0.1:50051"
connect = "lazy" # "eager" gates infrastructure readiness.
connect_timeout_ms = 10000
tls_handshake_timeout_ms = 10000
max_encoding_message_bytes = 4194304
max_decoding_message_bytes = 4194304
# Optional; no global request deadline is imposed by default:
# request_timeout_ms = 2000
```

`GrpcTokenProvider::token()` asynchronously returns `Secret<String>` for each call.
A valid explicit `Authorization: Bearer ...` credential suppresses the provider:

```rust,ignore
let mut request = tonic::Request::new(message);
set_grpc_bearer_token(&mut request, &token)?;
request.set_timeout(std::time::Duration::from_secs(2));
let response = remote.client().unary(request).await?;
```

Duplicate or malformed credentials and provider failures fail locally before
remote dispatch. Incoming credentials are never forwarded implicitly. Provider
failure messages are redacted; deliberate debugging can inspect
`GrpcClientError::cause()` without exposing causes through automatic Tonic status
conversion. Eager readiness retains a typed, redacted `GrpcClientError` source
with its original transport or timeout cause available through `cause()`.

Explicit per-call timeout metadata takes precedence over client defaults. The
establishment deadline includes asynchronous credential preparation, connection
readiness, and dispatch to response headers. It does not bound established response
stream lifetime. Native cancellation remains: for bidirectional RPCs, stop the
request producer and drop the response. No Furnace cancellation handle is added.

## Passport and infrastructure

Enable `jwt` for shared Passport admission policies. Only authorization Bearer
metadata is supported by the gRPC integration. Roles, permissions, and typed
predicates run before generated message decoding and handler side effects.
`grpc_authenticated::<Principal, _>(&request)` and
`grpc_verified_token::<Claims, _>(&request)` retrieve the installed typed handles.
Passport context uses the accepted socket peer and sanitized metadata.

Streams authenticate once at admission. A stream can continue after its JWT
expires; subsequent guarded calls validate again. Retain the typed principal
handle in application stream logic when that logic needs the admitted identity.
The managed server also honors incoming `grpc-timeout` through authentication and
handler response establishment; established response bodies retain their lifetime.

Framework statuses use generic `UNAUTHENTICATED`, `PERMISSION_DENIED`, and
`INTERNAL` messages. Application-authored Tonic statuses retain their normal
behavior. Native oversized protobuf messages produce `OUT_OF_RANGE`.

Health Check/Watch is always public. Reflection defaults to disabled. Enable it
explicitly and declare `#[grpc_reflection(guard = Reader)]` on the root cauldron.
Both reflection v1 and v1alpha require that guard. Descriptor replies expose only
selected services plus installed infrastructure, including imported message types;
unrelated service declarations are filtered out. Reflection authorization does
not grant business RPC invocation permission.

## Verified TLS

Enable standalone `tls` or facade `grpc-tls`, then configure a server identity:

```toml
[grpc.server.tls]
certificate = "certs/server.pem"
private_key = "certs/server.key"

[grpc.clients.remote.tls]
custom_ca_files = ["certs/ca.pem"]
custom_ca_only = true
domain_name = "localhost"
```

The remote endpoint must use `https`. Paths resolve relative to their configuration
source, including each configured CA array element. Clients use WebPKI roots by
default. Custom CAs are additive unless `custom_ca_only = true`, which requires at
least one configured CA. HTTPS enables default WebPKI TLS even without a client
TLS table. Hostname and chain verification remain mandatory.
Framework mTLS settings and certificate hot reload are outside this feature.
TLS material is read during runtime preparation, never during static inspection.

## Static inspection

`furnace grpc` (or `furnace grpc --format json`) lists selected service/method names, streaming shapes, and effective
policies through the standard `burn` inspection handshake. `graph` and `doctor`
also operate without constructors, lifecycle hooks, listeners, remote connections,
or TLS-file reads. Public HTTP route reports remain HTTP-only. JSON service
records contain `name`, `implementation`, `location`, and `methods`; methods
contain `name`, `path`, `client_streaming`, `server_streaming`, and nullable
`protection`. Authored policies expose strategy, principal, role/permission clauses,
and predicate identities without configuration values or credentials.

Public JSON schema version remains 2. The private application handshake is version
3 and rejects mismatches explicitly. The CLI accepts the facade and standalone
gRPC package as direct framework dependencies of the matching framework line.
