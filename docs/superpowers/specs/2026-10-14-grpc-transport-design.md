# gRPC server and client transport

Status: approved by the user on 2026-10-14. The user approved the confirmed requirements and concrete API, validation, diagnostic, and compatibility design. The user selected native execution after plan review. Implementation is in progress.

Branch: `feat/grpc-method`.

## Intent

Make gRPC a first-class Furnace transport for serving methods and consuming remote services. Applications define contracts in `.proto` files, use generated Rust types, and integrate services and clients with Furnace dependency injection, scoped cauldrons, lifecycle, configuration, and inspection.

## Confirmed requirements

- Support gRPC servers and injectable outbound clients.
- Use protobuf-first contracts with Tonic and Prost. Rust method attributes must not replace protobuf as the wire-contract authority.
- Support unary, server-streaming, client-streaming, and bidirectional-streaming RPCs.
- Preserve native Tonic cancellation semantics, as clarified during implementation: cancelling a bidirectional call means stopping its request producer and dropping its response. No additional Furnace cancellation handle is introduced. Runtime shutdown still enforces the configured drain deadline.
- Support HTTP and gRPC in one application through separate listeners sharing DI and lifecycle.
- Support plaintext and TLS for servers and clients, including configurable trusted CAs. Framework configuration for mTLS is deferred.
- Let each client select eager or lazy connection. An unavailable eager dependency fails startup; lazy clients attempt connection when used.
- Provide Furnace registration helpers for generated services and named client configuration.
- Use `#[grpc_service(...)]` on a generated Tonic trait implementation to link its server adapter and descriptor set. Register implementations with `grpc_services = [UsersService]` in declarative cauldrons.
- Named clients use distinct typed wrapper outputs, such as `UsersClient`, linked to `[grpc.clients.users]`; wrappers distinguish multiple destinations for the same generated service type.
- Default clients connect lazily. Connection and TLS handshake timeouts default to 10 seconds. No global RPC deadline is imposed; clients and individual calls may configure one. Default encoded and decoded message limits are 4 MiB each and are configurable. Graceful shutdown drains calls for up to 30 seconds, then cancels remaining calls.
- Include Passport/JWT protection in this version.
- Support service-wide protection, explicitly public methods, and method-specific roles and permissions.
- Accept incoming credentials only from `authorization: Bearer <JWT>` metadata. gRPC cookie authentication is outside this version.
- Authenticate streams once at call start. Token expiry during an established stream does not terminate it. Every new call requires fresh validation.
- Support explicit per-call outbound tokens and a configurable token provider. Incoming tokens are forwarded only through an explicit application action.
- Explicit per-call tokens take precedence over token providers. Provider failure or an unusable Bearer token fails locally without contacting the server or falling back to anonymous access.
- TLS is optional through `grpc-tls`. Clients trust bundled WebPKI public roots by default; configured custom CAs are additive unless custom-CA-only trust is explicitly selected. Always verify server hostnames and certificate chains.
- Applications generate Rust code and descriptor sets in their own `build.rs` using `tonic-prost-build`. Furnace documents this workflow and supplies an example; runtime does not invoke `protoc`.
- Repository tests and the gRPC example use a vendored protobuf compiler as a build dependency, as approved during planning. No system compiler installation is required for those builds.
- Include standard gRPC health checks and optional reflection. Reflection is disabled by default.
- Standard health checks are public. Enabled reflection requires a configured Passport guard.
- `Furnace::burn::<App>()` selects HTTP, gRPC, or both from configuration. Mixed serving uses one shared lifecycle coordinator.
- Select transports with `[furnace] transports = ["http", "grpc"]`, `["http"]`, or `["grpc"]`. An absent key preserves HTTP-only startup. Selecting a transport whose Cargo feature is disabled is a configuration error.
- Preserve HTTP `[server]` configuration and add `[grpc.server]` with default host `127.0.0.1` and port `50051`.
- Include CLI service/method and protection-policy inspection. gRPC project generation is deferred.
- Add `furnace grpc`, retain HTTP-only `furnace routes`, and extend `furnace doctor` with gRPC checks. Retain public JSON schema version 2 with a new command-specific data shape.
- Extract reusable Passport verification and policy logic with separate HTTP and gRPC adapters. gRPC protection must work without enabling Furnace's HTTP runtime.

## Existing architecture and constraints

The workspace targets Rust 1.94 and edition 2024, forbids unsafe code, and denies missing public documentation. All crates currently use workspace version `2.0.0-beta.1`.

`furnace-rs-core` owns graph analysis, typed construction, configuration, and lifecycle. It has no transport or JWT dependency. `furnace-rs-common` owns HTTP and JWT integrations, and `furnace-rs-common-macros` owns controller and Passport expansion. The public facade composes optional features.

HTTP serving currently owns its own startup and shutdown in `crates/furnace-rs-common/src/server.rs`. Running a second independent lifecycle-owning server would start or stop shared resources incorrectly. Mixed serving therefore needs one coordinator.

Passport is currently gated by `http + jwt`. Its context, principals, strategy extension installation, errors, and guard runtime use Axum types or HTTP scope. The refactor must preserve existing HTTP strategy signatures, typed principal/token APIs, cookie behavior, diagnostics, and error redaction.

Declarative cauldrons currently register providers, controllers, imports, and exports and check rooted DI contracts at compile time. gRPC registration must participate in the same ownership and dependency rules, including unrelated-module exclusion and native-output overrides.

CLI inspection currently uses an HTTP-owned private protocol and public JSON schema version 2. gRPC-only inspection must remain side-effect free and cannot depend on starting a listener or connecting a client.

## Proposed architecture for review

### Integration boundary

Add a feature-gated `grpc` integration under `furnace-rs-common`, re-exported through `furnace-rs`. Keep Tonic, protobuf descriptors, gRPC statuses, and transport-specific configuration outside core.

Keep existing default facade features unchanged. Proposed feature combinations are `grpc` for plaintext serving/consumption, `grpc + jwt` for Passport protection, and `grpc-tls` for optional TLS support. Existing `http + jwt` applications retain their API. Sharing neutral Passport types may require a direct `http` crate dependency, but must not require Furnace's `http` feature or its server runtime.

### Protobuf generation and service registration

Applications own `.proto` files and generate client/server stubs plus a `FileDescriptorSet` in `build.rs` using matching `tonic-prost-build` tooling. Furnace documents this setup and supplies an example. Runtime consumes descriptor bytes for method validation, streaming-shape inspection, and reflection without invoking `protoc`.

`#[grpc_service(...)]` on a generated Tonic trait implementation associates a concrete DI-managed implementation, its generated Tonic server adapter, and its file descriptor set. Add a declarative `grpc_services = [...]` cauldron entry and a fluent counterpart; both register a service as a managed provider with transport metadata. The concrete arguments proposed below are subject to written-spec review.

The generated Tonic trait remains the handler interface: `tonic::Request<T>`, `tonic::Response<T>`, `tonic::Status`, and native streaming types. Furnace resolves each selected service once during bootstrap; request dispatch uses the resulting shared implementation rather than planning the DI graph per request.

Validate service identity, duplicate selected registrations, descriptor membership, and authored method policies before provider construction wherever metadata permits. Reject unknown method names rather than silently ignoring a protection override. Descriptor validation must distinguish protobuf method names from generated Rust method names.

### Named clients

Named client configuration describes an endpoint, connection mode, timeouts, and optional TLS settings. A typed registration helper supplies the generated client as a native DI output. Connections are shared through cloneable Tonic channels; no application-wide mutex serializes all client calls.

Use distinct typed wrapper outputs for clients that share a generated client type but target different endpoints, linked to `[grpc.clients.<name>]` configuration. Furnace DI selects these concrete wrapper types rather than runtime string names. The concrete declaration and accessor proposals below are subject to written-spec review.

Validate endpoint and local configuration before network I/O. Eager connections belong to readiness and must complete before inbound serving starts; lazy configuration creates no remote connection during analysis or inspection. Connection failures retain server-side causes and use redacted framework formatting.

Token providers run for each outgoing call and may supply refreshed tokens. An explicit per-call token takes precedence and prevents invoking the provider for that call. Provider failure or a token that cannot form valid Bearer metadata fails locally, without contacting the server or falling back to anonymous access. Never hold a global lock across network calls or token-provider futures. Forwarding an inbound raw token must be explicit; a verified principal alone is not a forwarding credential.

### Shared runtime and lifecycle

A single coordinator prepares selected HTTP and gRPC transports, starts infrastructure/application lifecycle once, checks eager-client readiness, binds the requested listeners, and serves both. Shutdown stops admitting calls, drains transports, then shuts down shared providers once in reverse lifecycle order.

When one transport cannot bind or unexpectedly fails, stop its sibling and unwind the shared application. Preserve both the original operational error and any shutdown failure. A partial startup must release already-bound listeners.

`Furnace::burn::<App>()` selects transports automatically from `[furnace] transports`: `["http"]`, `["grpc"]`, or `["http", "grpc"]`. An absent key retains HTTP-only behavior. Selecting a transport whose Cargo feature is disabled produces a configuration error. Separate conventional `burn_grpc` and `burn_mixed` entry points are not required. Low-level APIs should accept caller-built Tonic services and shutdown futures.

Configuration uses existing source-loading and `FURNACE_*` environment conventions, preserves HTTP `server.*` keys, and adds `grpc.server.*` for the gRPC listener. The default gRPC host is `127.0.0.1` and port is `50051`. Clients default to lazy connection. Connection and TLS handshake timeouts default to 10 seconds. There is no global RPC deadline; clients and individual calls may set deadlines. Encoded and decoded message limits default to 4 MiB each and are configurable on clients and services. Graceful shutdown drains calls for up to 30 seconds, then cancels remaining calls before shutting down providers.

### Passport and JWT

Separate JWT verification, principal construction, role/permission evaluation, and static strategy metadata from HTTP extraction, HTTP rejection rendering, and controller-scoped preflight. Share existing Passport strategies between HTTP and gRPC without changing their existing validation signatures.

The gRPC adapter supplies a credential-sanitized Passport context for the RPC, installs typed authenticated principal and verified token values in request extensions, and evaluates service/method policy before invoking a handler. Use asynchronous middleware because existing Passport strategies may perform asynchronous validation; a synchronous Tonic interceptor alone is insufficient.

Incoming authorization must reject missing, malformed, duplicate, invalid, expired, and wrong-profile credentials consistently. Proposed mappings are `UNAUTHENTICATED` for credential/identity rejection, `PERMISSION_DENIED` for policy rejection, and redacted `INTERNAL` for operational failures. Public methods bypass their inherited service policy only when explicitly declared.

Apply identical admission protection to all four RPC shapes. An admitted stream retains its principal without periodic JWT revalidation. Application-specific per-message authorization remains application-owned.

gRPC policy metadata must retain strategy/principal type checks, cauldron visibility, and JWT configuration preflight. Cookie-source guards must be rejected for gRPC rather than falling back to Bearer metadata.

### TLS

Use native Tonic TLS with certificate/key loading for servers and server-name verification plus trusted CAs for clients. Resolve file paths according to existing source-aware configuration rules. Reject partial certificate configuration and incompatible endpoint/TLS settings. Certificate validation must not be disabled.

TLS support is optional through `grpc-tls`. Clients trust bundled WebPKI public roots by default. Configured custom CAs add to those roots unless custom-CA-only trust is explicitly selected. Hostname and chain verification remain mandatory. Proposed native escape hatches allow callers to configure Tonic directly. Framework-level mTLS settings and certificate hot reload remain deferred.

### Health and reflection

Standard health exposes serving state for selected services and changes state during startup and shutdown. Reflection consumes the same selected descriptors used for inspection and is enabled only by explicit configuration. Unrelated linked services must not appear in reflected descriptors.

Standard health checks are public. Enabled reflection requires a configured Passport guard; missing or invalid guard configuration fails preflight rather than exposing unguarded reflection. Reflection admission must use the same Passport/JWT validation and normalized status handling as protected application RPCs. Service RPC protection remains enforced regardless of reflection visibility. Reflection discoverability does not authorize invocation.

### CLI inspection

Extend inspection to list protobuf service names, method names, RPC paths, streaming shapes, and effective service/method protection. Report only policy structure and redacted configuration provenance; never serialize tokens, private keys, or configuration values.

Inspection must work for gRPC-only and mixed applications without constructing providers, starting lifecycle hooks, reading secret material, binding sockets, or connecting outbound clients. Continue existing HTTP inspection behavior.

Add `furnace grpc` with the existing package/binary selectors and `--format json` support. Keep `furnace routes` HTTP-only. Extend `furnace doctor` using its existing check-record structure. Retain public JSON schema version 2 and existing command-specific shapes, adding a new `grpc` command data shape. The private application/CLI protocol may advance independently with explicit compatibility checks.

## Concrete API proposals for written-spec approval

These details turn the agreed registration model into an implementable contract. They are proposed decisions for review, not previously confirmed requirements.

### Managed services and policies

```rust,ignore
#[burner]
pub struct UsersService {
    #[inject]
    users: UserRepository,
}

#[grpc_service(
    server = pb::users_server::UsersServer,
    descriptor = pb::FILE_DESCRIPTOR_SET,
    guard = UserGuard,
)]
#[tonic::async_trait]
impl pb::users_server::Users for UsersService {
    // Unannotated RPCs inherit UserGuard.
    async fn get_user(
        &self,
        request: tonic::Request<pb::GetUserRequest>,
    ) -> Result<tonic::Response<pb::User>, tonic::Status> {
        let _principal = grpc_authenticated::<UserPrincipal, _>(&request)?;
        Err(tonic::Status::unimplemented("example handler"))
    }

    #[grpc_method(name = "Login", public)]
    async fn login(
        &self,
        request: tonic::Request<pb::LoginRequest>,
    ) -> Result<tonic::Response<pb::LoginReply>, tonic::Status> {
        Err(tonic::Status::unimplemented("example handler"))
    }

    #[grpc_method(name = "DeleteUser", guard = AdminGuard)]
    async fn delete_user(
        &self,
        request: tonic::Request<pb::DeleteUserRequest>,
    ) -> Result<tonic::Response<pb::Empty>, tonic::Status> {
        Err(tonic::Status::unimplemented("example handler"))
    }
}

#[cauldron(root, providers = [UserRepository, AccessStrategy],
    grpc_services = [UsersService])]
pub struct App;
```

The example handler bodies return native Tonic statuses only to illustrate the interfaces; business behavior is application-owned. `grpc_service` does not generate protobuf Rust types or managed constructors; `#[burner]` or an explicit `Injector` supplies construction. The macro consumes `grpc_method` helper attributes before passing the trait implementation to `tonic::async_trait`.

Arguments are required `server = RustPath`, required `descriptor = RustPath` naming `&'static [u8]`, and optional `guard = RustPath`. The server path names a generated server constructor without generic arguments. The generated adapter binds it to the implementation type and derives the fully qualified wire service identity from Tonic `NamedService::NAME`.

Each helper override uses the exact protobuf method name, not its snake-case Rust spelling. `public` and `guard` are mutually exclusive. Unannotated methods inherit the service guard; absent a service guard they are public. A method guard replaces the service guard in full; stricter role/permission combinations are authored as a separate existing `#[guard(...)]` type. This avoids ambiguous partial-policy merging. Role/permission `any`/`all` and predicate semantics remain identical to HTTP guards.

Reject duplicate/unknown arguments, malformed or repeated overrides, unknown wire methods, invalid descriptors, duplicate selected wire service names, and cookie-source guards. Initial macro support is non-generic concrete trait implementations; one managed implementation type registers one protobuf service. Applications needing several service traits use distinct managed adapter types over shared injected business services.

Provide a common-layer `GrpcCauldronExt::grpc_service<T: GrpcService>(self) -> Self` for fluent registration. Core retains only generic transport-tagged provider metadata. Declarative expansion treats gRPC implementations as providers in the rooted DI checker while recording their transport membership. No Tonic or protobuf types enter core.

Typed principal accessors are `grpc_authenticated<P, T>(&tonic::Request<T>) -> Result<Authenticated<P>, tonic::Status>` and `grpc_verified_token<C, T>(&tonic::Request<T>) -> Result<VerifiedToken<C>, tonic::Status>`. Missing guard-installed extensions produce a redacted internal status. Existing `Authenticated` and `VerifiedToken` representations remain shared between transports.

### Client declarations and credentials

```rust,ignore
#[grpc_client(name = "users", client = pb::users_client::UsersClient)]
pub struct UsersClient;

#[grpc_client(name = "admin-users", client = pb::users_client::UsersClient,
    token_provider = ServiceTokenProvider)]
pub struct AdminUsersClient;

#[cauldron(providers = [UsersClient, AdminUsersClient, ServiceTokenProvider])]
pub struct RemoteServices;
```

Client declarations are non-generic unit structs. The macro generates a cloneable shared wrapper and `Injector` with `Config` plus the optional token-provider type as its declared dependencies. Names use lowercase ASCII `[a-z][a-z0-9_-]*`. Unknown arguments, invalid names, and non-unit declarations are compilation errors. Two selected wrappers using the same name are rejected as ambiguous named configuration.

`Wrapper::client(&self) -> GeneratedClient<GrpcChannel>` returns a cheap clone configured with message limits and middleware. Callers keep native generated RPC methods, including streaming methods. The wrapper does not serialize calls through a shared client mutex.

`GrpcTokenProvider: Clone + Send + Sync + 'static` exposes `fn token(&self) -> impl Future<Output = GrpcClientResult<Secret<String>>> + Send`. The provider is DI-managed and may refresh credentials asynchronously. A configured provider returning an error fails the call; it cannot silently opt into anonymous access.

`set_grpc_bearer_token<T>(&mut tonic::Request<T>, &str) -> GrpcClientResult<()>` validates and sets explicit Bearer metadata. Duplicate authorization values and malformed outbound Bearer metadata fail before the provider or network is invoked. A valid explicit token suppresses provider invocation. Without explicit credentials or a provider, clients send no authorization metadata. Framework errors redact token values.

Native generated clients remain an application-owned escape hatch and can be supplied as ordinary DI outputs without these helpers.

### Reflection guard registration

```rust,ignore
#[grpc_reflection(guard = ReflectionGuard)]
#[cauldron(root, providers = [AccessStrategy], grpc_services = [UsersService])]
pub struct App;
```

The outer attribute registers a typed reflection policy for that root; the inner cauldron attribute retains DI checking. Reflection activation still comes from configuration. Selecting reflection without `jwt`, without this root policy, or with an invalid/invisible strategy fails preflight. At most one reflection policy may be selected for a root. Health `Check` and `Watch` remain public.

Reflection exposes only selected application services plus enabled infrastructure services. Merge matching descriptor files, reject conflicting definitions, retain imported message dependencies, strip unselected service declarations, and strip source-code information. Do not list or return unrelated service declarations simply because they share a descriptor file with a selected service.

### Runtime API compatibility

Move lifecycle coordination into a shared runtime module. Keep `FurnaceBurnExt::burn<M: Cauldron>()` available with either transport feature. Introduce `ApplicationRuntimeError`; retain `HttpRuntimeError` as an alias to the same enum under `http`, preserving existing HTTP variants and source signatures. Add transport-tagged gRPC/bootstrap failures without changing HTTP response errors.

Expose `build_grpc_routes(&Furnace) -> furnace_rs_core::Result<tonic::service::Routes>` for native route composition. Low-level mixed serving accepts prepared HTTP/gRPC routes, explicit addresses/settings, and one shutdown future; it owns the lifecycle once. Exact internal helper signatures belong in the implementation plan.

Selecting a transport with no selected application endpoints is a bootstrap error. Health/reflection alone do not satisfy the application-service requirement. Outbound-client-only applications use the existing low-level core build/start/shutdown APIs and need not select an inbound transport.

### Configuration grammar and validation

```toml
[furnace]
transports = ["http", "grpc"]

[server]
host = "127.0.0.1"
port = 3000

[grpc.server]
host = "127.0.0.1"
port = 50051
max_encoding_message_bytes = 4194304
max_decoding_message_bytes = 4194304
tls_handshake_timeout_ms = 10000
shutdown_timeout_ms = 30000

# Omitting this table selects plaintext.
[grpc.server.tls]
certificate = "certs/server.pem"
private_key = "certs/server.key"

[grpc.reflection]
enabled = false

[grpc.clients.users]
endpoint = "https://users.example.com:50051"
connect = "lazy"
connect_timeout_ms = 10000
tls_handshake_timeout_ms = 10000
max_encoding_message_bytes = 4194304
max_decoding_message_bytes = 4194304
# Optional default deadline for call establishment, not stream lifetime:
# request_timeout_ms = 30000

[grpc.clients.users.tls]
custom_ca_files = ["certs/internal-ca.pem"]
custom_ca_only = false
# Optional certificate/SNI hostname override; verification still applies:
# domain_name = "users.example.com"
```

Endpoint is required for every selected helper client. Client `connect` accepts only `lazy` and `eager`, defaulting to `lazy`. Only `http://` and `https://` endpoints are supported by conventional configuration. HTTPS requires `grpc-tls`; client TLS options on an HTTP endpoint are an error. An omitted client TLS table for HTTPS uses verified WebPKI roots.

`furnace.transports` must be a nonempty array with only `http` and `grpc`, without duplicates. Either order denotes the same transport set. The existing HTTP bind defaults stay unchanged. Conventional gRPC `host` must be a literal IPv4/IPv6 address; port is 1 through 65535. Caller-supplied low-level listeners may use port zero in tests.

Durations are positive millisecond integers fitting checked deadline arithmetic; sizes are positive byte integers supported by Tonic. A zero deadline/size, overflow, partial server TLS configuration, custom-CA-only trust with no configured CAs, or invalid endpoint is a source-attributed configuration error. Files are source-relative. Inspection validates syntax and declared paths without reading certificates/private keys or testing DNS/connectivity.

Client default request timeouts bound call establishment to response headers; they do not promise to bound the lifetime of streaming response bodies. An explicit per-call `tonic::Request::set_timeout` takes precedence over the configured default and is sent as gRPC deadline metadata. Use one deadline mechanism in middleware rather than layering a second conflicting endpoint timeout. Active streams remain subject to client cancellation and the agreed shutdown drain deadline.

### CLI records and errors

The `furnace grpc --format json` envelope retains `schema_version: 2` and `command: "grpc"`. Its data is `{ "services": [...] }`. Each service record contains `name`, `implementation`, `location`, and `methods`. Each method contains `name`, `path`, `client_streaming`, `server_streaming`, and `protection`. Protection is either null for public RPCs or a record of `strategy`, `principal`, `roles`, `permissions`, and `predicates`. Roles/permissions use `{ "mode": "any" | "all", "values": [...] }` when present. Predicates expose authored function identities only. No configuration values or credentials enter the public output.

Sort services by fully qualified protobuf name and methods by protobuf method name. Keep existing command selector, diagnostic, exit-code, and path-normalization rules. `doctor` adds gRPC checks to its existing check array; HTTP route data and other public command shapes remain unchanged. Inspection on a valid root without selected gRPC services returns an empty service array rather than starting a runtime.

Proposed diagnostic allocation is `FURNACE150` for gRPC descriptor/registration errors, `FURNACE151` for gRPC configuration, `FURNACE152` for gRPC protection preflight, and `FURNACE153` for named-client/readiness errors. Existing Passport/JWT diagnostics remain available as causes; persistence retains its existing `FURNACE140`. Framework-produced RPC statuses use fixed generic messages for authentication, authorization, and internal failures. Application-authored `tonic::Status` values retain native Tonic behavior.

## Acceptance criteria

1. An example defined by `.proto` serves and consumes all four RPC shapes through Furnace-managed dependencies.
2. gRPC-only, HTTP-only, and mixed applications compile with their intended feature combinations; the core acquires no transport dependency.
3. Selected services follow cauldron ownership, exports/imports, root isolation, and compile-time DI rules. Duplicate or invalid service/policy metadata fails bootstrap.
4. Shared providers start and stop exactly once in mixed serving. Eager-client failure and either listener's failure unwind all acquired resources.
5. Lazy clients let startup succeed while their remote endpoint is unavailable; eager clients fail readiness under the same condition.
6. TLS succeeds with a trusted certificate and rejects an untrusted CA, wrong hostname, invalid key/certificate pairing, and partial configuration.
7. Protected unary and streaming calls reject bad credentials before handler side effects, enforce roles/permissions, and install the expected typed principal. Explicitly public overrides work.
8. JWT expiry during a started stream does not terminate it; a subsequent call with the expired token is rejected.
9. Outbound per-call tokens and token providers work. No inbound credential is automatically forwarded. Multiple typed clients can coexist without output ambiguity.
10. Health and reflection follow the agreed access policies and selection scope. Reflection is disabled by default.
11. CLI reports service/method/streaming/policy metadata without construction, lifecycle, connections, sockets, or credential disclosure.
12. Existing HTTP Passport/cookie tests and current CLI contracts remain valid under the agreed compatibility strategy.

## Review and implementation gate

The user approved this written specification on 2026-10-14. Any newly discovered unclear requirement must be raised with the user; approval is not permission to introduce unstated behavior.

After approval, write a task-by-task implementation plan with file ownership, public/internal interfaces, tests, feature/MSRV verification, and lifecycle/security regression checks. The user reviews that plan and selects an execution approach before product implementation starts.

## Verification strategy

Use focused unit tests for configuration, scope, descriptor/policy validation, redaction, and token selection. Add macro compile-pass/fail tests and renamed-dependency consumer fixtures for registration contracts. Use loopback Tonic integration tests for real protobuf framing, all streaming shapes, TLS, authentication, readiness, cancellation, and mixed lifecycle failures. Use CLI fixtures that fail if inspection constructs a provider or performs I/O.

Run workspace formatting, Clippy, tests, doctests, rustdoc warnings, and Rust 1.94 compatibility gates after focused checks pass. Test HTTP-only, gRPC-only, gRPC/JWT, TLS, mixed, and no-default-feature consumer combinations. Documentation-only drafting does not constitute implementation verification.

## Technical references

- Tonic server: https://docs.rs/tonic/latest/tonic/transport/struct.Server.html
- Tonic routes: https://docs.rs/tonic/latest/tonic/service/struct.Routes.html
- Tonic client endpoints: https://docs.rs/tonic/latest/tonic/transport/struct.Endpoint.html
- Client TLS: https://docs.rs/tonic/latest/tonic/transport/channel/struct.ClientTlsConfig.html
- Server TLS: https://docs.rs/tonic/latest/tonic/transport/server/struct.ServerTlsConfig.html

Exact dependency versions and MSRV compatibility will be verified when finalizing the plan; latest online documentation alone does not establish workspace compatibility.

Confirmed during implementation: per-call and configured RPC establishment deadlines include asynchronous token-provider wait, connection readiness, and dispatch. Established response streams retain native lifetime behavior.


## Approved crate migration amendment

The user requested migration before continuing the remaining implementation and confirmed this layout:

- `furnace-rs-grpc` is a standalone transport package. gRPC is its base functionality; its optional features are `jwt` and `tls`.
- `furnace-rs-grpc-macros` owns `grpc_service`, `grpc_client`, and `grpc_reflection` expansion and resolves standalone, renamed, and facade dependencies.
- The `furnace-rs` facade retains `grpc` and `grpc-tls`, mapping them to the new package. Facade `jwt` weakly enables the new package's `jwt` feature without enabling gRPC in JWT-only applications.
- Remove gRPC APIs, features, implementation, macros, and Tonic/Prost dependencies from common. Shared Passport/JWT stays in common. Transport-neutral Bearer metadata parsing and hidden scoped Passport integration interfaces support the new crate without enabling Furnace HTTP.
- Common JWT auto-configuration receives selected protection requirements through a neutral inventory callback. It never depends on the gRPC package or its protobuf types.
- Direct standalone gRPC consumers can use the re-exported core macros and optional Passport macros without declaring extra package aliases. Core and Passport macro path resolution recognizes this consumer path without adding transport dependencies to core.
- Existing gRPC behavior, security policies, deadlines, native cancellation, version floor, and proto ownership remain unchanged. Move the already implemented tests and in-progress TLS/health/reflection work with their implementation.
- Shared lifecycle/startup and private inspection coordination remain transport-neutral in common. Remaining Tasks 8 and 9 use integration registrations from the gRPC crate rather than importing Tonic or Prost into common.

Migration is explicitly authorized by the user's instruction to start migrating now, followed by confirmation of standalone/facade separation and dedicated macro/features layout. Native execution and the final independent review remain selected.

### Approved runtime API adjustment after crate separation

The user approved protocol-neutral `ApplicationRuntimeError::TransportBind { transport, source }` and `TransportServe { transport, source }`, retaining the original errors through `std::error::Error::source()`. Existing HTTP variants and `HttpRuntimeError` remain compatible. Shared `RuntimeRoutes` and `RuntimeBindings` expose `new()` and HTTP `with_http(...)`; the standalone crate supplies `RuntimeGrpcExt::with_grpc(...)` for native Tonic routes and `GrpcServerConfig`. The facade prelude exports this extension. Explicit configuration may override its address with `GrpcServerConfig::with_address(...)`, including port zero.

Common owns transport selection, conventional source loading, coordination, and task supervision without importing Tonic or Prost. Linked adapters validate selected managed operations before provider construction and prepare native routes before startup. The gRPC adapter serves native Tonic routes through supervised Hyper HTTP/2 connection and stream tasks, using verified Rustls for TLS, because Tonic's convenience server does not expose task handles for forced cancellation. HTTP retains the existing guarded connection implementation.
