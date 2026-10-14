# gRPC Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking. Execution method: native, approved by the user. Implementer works in this session; independent whole-branch review follows verification.

**Goal:** Serve and consume all four protobuf-defined gRPC shapes through Furnace DI, configuration-selected runtimes, Passport/JWT, TLS, and offline CLI inspection.

**Architecture:** Keep transport implementation in feature-gated common modules and generic transport membership in core. Share Passport verification and policy logic between HTTP and asynchronous gRPC middleware. One runtime coordinator owns both listeners and the shared lifecycle.

**Tech Stack:** Rust 1.94, edition 2024, Tokio, Tower 0.5, Axum 0.8, Tonic/tonic-prost/tonic-prost-build/tonic-health/tonic-reflection 0.14.6, Prost/prost-types/prost-build 0.14.4, protoc-bin-vendored 3.3.0. Resolve and lock these compatible release lines at implementation time; an MSRV failure must be reported rather than raising Furnace's Rust floor.

**Spec:** [Approved gRPC specification](../specs/2026-10-14-grpc-transport-design.md).

## Global Constraints

- Branch: `feat/grpc-method`; preserve the user's selected branch. Check for concurrent edits before each task.
- Rust 1.94 and edition 2024; workspace version `2.0.0-beta.1`; unsafe code forbidden; public documentation required.
- Core acquires no HTTP, Axum, Tonic, protobuf, JWT, cookie, or Serde dependency.
- Default facade features stay unchanged; `grpc`, `grpc + jwt`, and optional `grpc-tls` compose independently from Furnace `http`.
- Contracts come from `.proto`; applications own build-time generation. Repository tests/example use a vendored compiler only at build time.
- Preserve existing HTTP entry-point/error signatures, Passport strategy signatures, cookies, deadlines, trusted peer addresses, and public CLI schema version 2.
- `furnace.transports` defaults to HTTP when absent; explicit selection is nonempty, unique, and limited to compiled `http`/`grpc` features.
- gRPC bind defaults: `127.0.0.1:50051`; conventional port range 1..=65535; low-level tests may bind port zero.
- Clients default to lazy connection; connection/TLS handshake timeout 10 seconds; no global RPC deadline; configurable 4 MiB encoding and decoding limits.
- Graceful shutdown drains for up to 30 seconds, then cancels remaining calls before stopping shared providers.
- Incoming Bearer metadata only; authenticate once at call admission; retain admitted streaming principal until the stream ends.
- Public health; reflection off by default and guarded when enabled; unrelated services must not leak through descriptors.
- Per-call outbound token overrides provider; malformed credentials/provider failure fail locally; no implicit inbound forwarding.
- TLS verifies hostname and chain; WebPKI public roots plus custom CAs by default; explicit custom-CA-only mode supported.
- Diagnostics: `FURNACE150` registration/descriptors, `FURNACE151` configuration, `FURNACE152` gRPC policy preflight, `FURNACE153` clients/readiness; persistence keeps `FURNACE140`.
- No product code, dependency installation, branch creation, or generated product scaffolding during this planning stage. Newly discovered requirement ambiguity goes to the user.

## Review Focus

1. A reflection request for a file containing both selected and unrelated services must never reveal the unrelated service (Task 7).
2. A rejected streaming call must not consume request messages or invoke a handler, and admitted streams must survive JWT expiry (Task 5).
3. A client/provider failure or duplicate Bearer metadata must perform zero remote calls and disclose no token in its local error (Task 6).
4. A second listener bind failure or drain timeout must release the first listener and stop each provider exactly once (Task 8).
5. A gRPC-only CLI inspection with eager clients and inaccessible TLS files must construct nothing and perform no network/key-file I/O (Task 9).

## File Structure and Dependencies

| Owner | Files / responsibility |
|---|---|
| Core | `src/cauldron.rs`, `src/graph/cauldron.rs`, `src/lib.rs`: neutral transport membership; existing preflight/config/lifecycle stay authoritative |
| Core macros | `src/cauldron/{declaration,contract}.rs`, `src/cauldron.rs`: `grpc_services` provider slots and rooted dependency proofs |
| Common Passport | `src/passport/{context,principal,strategy,error,guard}.rs`, new `policy.rs`, `authentication.rs`, `scope.rs`, `http.rs`: neutral contracts plus HTTP adapters |
| Common gRPC | New `src/grpc/{mod,error,config,descriptor,scope,preflight,service,router,passport,client,tls,health,reflection}.rs`: transport units |
| Common runtime | New `src/runtime/{mod,config,error,coordinator}.rs`: configuration selection and single lifecycle ownership; existing `server.rs` retains HTTP connection delivery |
| Common macros | New `src/grpc_service.rs`, `src/grpc_client.rs`, `src/grpc_reflection.rs`: descriptor/adapter and typed wrapper generation |
| Inspection | Existing `src/inspection.rs`, CLI command/output/inspection modules: transport-neutral private protocol, new command data |
| Fixtures | New `tests/grpc-support` build-only workspace package; `crates/furnace-rs-common/tests/grpc_*.rs`; facade UI/consumer fixtures; CLI fixtures |
| Examples/docs | New independent `example/grpc-transport`; `docs/grpc.md`; existing README, architecture, CLI, feature/crate docs, changelog and CI |

Paths in this table are relative to their owning crates unless fully qualified below. Tasks are sequential: `1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10`. The deliverables share DI, policy, and lifecycle contracts, so this is one coordinated feature plan rather than independent subsystem plans.

## Shared Test Contract

Task 2 creates `tests/grpc-support` (package `furnace-grpc-test-support`, `publish = false`) with `build.rs`, `proto/transport.proto`, and `src/lib.rs`. Test support depends on generated-code libraries only, never Furnace, so all owning crates may use it without a dependency cycle.

Use protobuf package `furnace.test.v1`, messages `Item { uint64 value = 1; }` and `Summary { uint64 count = 1; uint64 total = 2; }`, service `Transport` with `Unary(Item) returns (Item)`, `ServerStream(Item) returns (stream Item)`, `ClientStream(stream Item) returns (Summary)`, and `Bidi(stream Item) returns (stream Item)`. Include unrelated service `Hidden` in the same source file to exercise filtering. Export `pb` and `FILE_DESCRIPTOR_SET: &'static [u8]`.

Test processes use loopback port-zero listeners, explicit shutdown channels, bounded awaits, and task join handles. Generate TLS keys/certificates in temporary directories with a dev-only certificate helper (rcgen 0.14 release line); never commit key material. Tests use fixture-owned JWT signing keys and deterministic claims without logging tokens. Use paused time or an injectable verification clock for expiry assertions; do not add wall-clock sleeps to JWT tests.

---

### Task 1: Extract Passport verification and policy from HTTP

**Files:** Modify root `Cargo.toml`/`Cargo.lock`, `crates/furnace-rs-common/Cargo.toml`, `crates/furnace-rs-common/src/passport/{mod,context,principal,strategy,error,guard}.rs`, `src/http_scope.rs`, `src/http_preflight.rs`, `src/lib.rs`; create `src/passport/{policy,authentication,scope,http}.rs`. Update exports/macros only as needed to preserve existing interfaces.

**Interfaces:** Preserve `PassportStrategy::validate(&self, &PassportContext<'_>, &JwtClaims<Self::Claims>)`, existing principal/token wrappers, and public HTTP guard APIs. Produce neutral `ScopedGuard` carrying static guard, occurrence identity and requester `TypeId`; neutral `GuardOccurrence { provider: TypeId, operation: String }`; `authenticate_bearer(&ApplicationContext, PassportStrategyAdapter, &http::HeaderMap, &http::Method, &http::Uri, Option<SocketAddr>) -> impl Future<Output = PassportResult<ErasedAuthentication>> + Send`; `authorizes(&GuardDescriptor, &ErasedAuthentication) -> bool`. Keep `PassportStrategyCatalog::preflight_scoped` signature over neutral `ScopedGuard`.

- [x] Write focused neutral-context tests: `bearer_context_strips_credentials` asserts no Authorization/Proxy-Authorization/Cookie; `shared_policy_keeps_and_clauses` asserts role-any AND permission-all AND predicates; `http_adapter_retains_typed_extensions` verifies the same principal/token allocations. Existing custom-strategy, cookie, visibility, rejection and Send-future tests remain unchanged.

  Assertion excerpt for the sanitized context test:
  ```rust
  assert!(context.headers().get(http::header::AUTHORIZATION).is_none());
  assert!(context.headers().get(http::header::COOKIE).is_none());
  ```
- [x] Run `cargo test -p furnace-rs-common --all-features passport`; the new neutral entry points initially fail to compile. Record the failure before moving code.
- [x] Add optional workspace `http` 1.x metadata dependency, initially enabled by Furnace `http`, and extract shared units using its metadata/Extensions directly. Keep Axum `FromRequestParts`, response rendering, CookieJar access, HTTP request extraction, and HTTP route occurrence conversion in `passport/http.rs` or feature-gated adapter portions. Adapt existing hidden macro paths without changing application signatures. Preserve credential sanitation and exact Bearer parsing. Task 2 enables the neutral metadata dependency for gRPC too.
- [x] Run `cargo test -p furnace-rs-common --all-features passport`, `cargo test -p furnace-rs --all-features --test passport_ui`, and `cargo test -p furnace-rs --test feature_matrix`. Expected: shared tests pass, existing HTTP UI diagnostics/contracts pass, JWT-only normal dependencies still exclude Axum.
- [x] Stage only Task 1 files and commit `refactor(passport): separate authentication policy from HTTP adapters`.

### Task 2: Add gRPC features, configuration, errors, and generated test contracts

**Files:** Modify root `Cargo.toml`/`Cargo.lock`, `crates/furnace-rs-common/Cargo.toml` and `src/lib.rs`, `crates/furnace-rs/Cargo.toml` and `src/lib.rs`, `crates/furnace-rs-common-macros/Cargo.toml`; create `src/grpc/{mod,config,error}.rs`, `src/runtime/{mod,config}.rs`, `tests/grpc-support/{Cargo.toml,build.rs,proto/transport.proto,src/lib.rs}`; modify `crates/furnace-rs/tests/feature_matrix.rs`.

**Interfaces:** Produce `RuntimeSelection { http: bool, grpc: bool }` with `RuntimeSelection::parse(&Config, compiled_http: bool, compiled_grpc: bool) -> core::Result<Self>`; `GrpcServerConfig::parse(&Config) -> core::Result<Self>`; `GrpcClientConfig::parse(&Config, name: &str) -> core::Result<Self>`; `GrpcClientError`, `GrpcClientResult<T>`, diagnostic constants; fixture package exports described above. Config types own parsed endpoint, source-relative paths, limits, connection mode, and `Duration` values. Parsing does no filesystem/network I/O.

- [x] Write `grpc_config_defaults` assertions for `127.0.0.1`, 50051, lazy, 10000 ms, 4194304 bytes, no request timeout, and 30000 ms shutdown. Add `transport_selection_rejects_empty_duplicate_unknown_or_uncompiled`, `grpc_config_rejects_zero_overflow_partial_tls_and_custom_ca_only_without_cas`, and `grpc_configuration_paths_follow_source` tests. Feature-tree tests assert default/JWT-only builds exclude Tonic; `grpc + jwt` does not enable Furnace `http`, `axum-extra`, or CORS/cookie dependencies. Tonic's own router may bring Axum internally.

  Pin the parsed server defaults as public config accessors:
  ```rust
  assert_eq!(server.address().to_string(), "127.0.0.1:50051");
  assert_eq!(server.max_decoding_message_bytes(), 4_194_304);
  assert_eq!(server.shutdown_timeout(), std::time::Duration::from_secs(30));
  ```
- [x] Run `cargo test -p furnace-rs-common --no-default-features --features grpc,jwt grpc_config`; initially fails because the feature/config types do not exist. Then implement the feature wiring and config contract.
- [x] Add release-line dependencies from the header; Tonic disables defaults and enables `transport`, `router`, `codegen`; `grpc-tls` implies `grpc` plus Tonic `tls-ring` and `tls-webpki-roots`. JWT backend configuration stays unchanged. Share runtime/inspection serde and Tokio requirements through either transport. Enable shared Passport for `jwt + (http or grpc)` and HTTP adapters only under `http`.
- [x] Generate fixtures using `tonic_prost_build::Config::protoc_executable(protoc_bin_vendored::protoc_bin_path()?)` and `configure().file_descriptor_set_path(...).compile_with_config(...)`. Do not use `std::env::set_var`, which is unsafe under edition 2024. The support crate is dev-only for consumers; its compiler/build libraries must not enter Furnace normal dependencies.
- [x] Run the config tests, `cargo test -p furnace-rs --test feature_matrix`, `cargo +1.94.0 check -p furnace-grpc-test-support`, and `cargo +1.94.0 check -p furnace-rs --no-default-features --features grpc,jwt,grpc-tls,runtime-tokio`. Expected: defaults/negative cases pass; dependency lines resolve on Rust 1.94. Report incompatibility rather than changing the floor or silently selecting an alternative stack.
- [x] Commit `feat(grpc): add opt-in transport configuration and protobuf fixtures`.

### Task 3: Register services in scoped, checked cauldrons

**Files:** Modify `crates/furnace-rs-core/src/{cauldron,lib}.rs`, `src/graph/cauldron.rs`; modify core macro `src/cauldron.rs` and `src/cauldron/{declaration,contract}.rs`; create common `src/grpc/{descriptor,scope,service,preflight}.rs`; create `crates/furnace-rs-common-macros/src/grpc_service.rs`, update macro `src/{lib,path}.rs`; create facade `tests/grpc_ui.rs`, `tests/ui-grpc/{pass,fail}` fixtures and consumer `grpc_renamed/{Cargo.toml,src/main.rs}`.

**Interfaces:** Core hidden trait `TransportProvider: Injector { const TRANSPORT: &'static str; }`; `CauldronRegistration<M>::transport_provider<T: TransportProvider>(self) -> Self`; `CauldronGraph::registered_transport_providers(&self, transport: &str) -> impl Iterator<Item = TypeId>`. Common produces `GrpcService: TransportProvider + Clone` with `fn grpc_descriptor() -> &'static GrpcServiceDescriptor`, `GrpcServiceDescriptor`, `GrpcMethodOverride`, `GrpcApplicationScope::for_cauldron_graph(Option<&CauldronGraph>) -> core::Result<Self>`, `for_application(&Furnace) -> core::Result<Self>`, and `GrpcCauldronExt::grpc_service<T: GrpcService>(self) -> Self`. Descriptor retains service/implementation identity functions, static descriptor bytes, source location and service/method guard declarations. Task 4 extends it with the typed registrar and its argument contract.

- [x] Add compile-pass tests for facade/direct-common/renamed imports, dependency fields, service registration, fluent registration, overrides and root imports. Add compile-fail tests for invalid attribute arguments, generic impls, missing `#[inject]`, undeclared dependencies, inaccessible root outputs, repeated overrides, `public + guard`, and selecting a type lacking service metadata. Root proofs must include each gRPC service dependency tuple. Run the new `grpc_ui` target and confirm failures before expansion changes.
- [x] Add scope tests `only_reachable_registered_services_selected`, `grpc_registered_provider_without_service_metadata_rejected`, `duplicate_wire_service_and_unknown_method_fail_preconstruction`, and `descriptor_conflicts_fail_preconstruction`. Use a constructor sentinel and assert its invocation count remains zero for metadata failures.

  ```rust
  assert_eq!(error.code(), FURNACE150);
  assert_eq!(constructor_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
  ```
- [x] Implement neutral membership without replacing existing controller flags; update rooted proofs/slot offsets and fluent lowering for `grpc_services`. Retain manual `Injector`, native overrides, global/direct import rules and legacy cauldrons. Macro derives wire identity from `<GeneratedServer<Implementation> as NamedService>::NAME` and consumes helpers before `tonic::async_trait`.
- [x] Validate descriptors with Prost, build selected method records, reject all unsupported/malformed forms pinned in the spec, and register `preflight_grpc(&PreflightContext<'_>) -> Vec<Diagnostic>` through existing `PreflightDescriptor`. Use analysis memoization to share selected metadata between preflight, JWT defaults, and inspection; do not construct providers.
- [x] Run `cargo test -p furnace-rs-core --all-features`, `cargo test -p furnace-rs-common --all-features grpc::scope`, and `cargo test -p furnace-rs --all-features --test grpc_ui`. Add the renamed consumer to the existing consumer matrix and run that matrix. Expected: root isolation/negative diagnostics pass and HTTP declarative checks remain unchanged; confirm the scope filter actually executes the new tests.
- [x] Commit `feat(grpc): register scoped protobuf services with checked DI`.

### Task 4: Dispatch all four RPC shapes through managed services

**Files:** Create common `src/grpc/router.rs`; extend `src/grpc/{service,descriptor,mod}.rs` and macro `src/grpc_service.rs`; create `crates/furnace-rs-common/tests/grpc_dispatch.rs` and `tests/support/grpc.rs`; extend facade exports.

**Interfaces:** Produce `build_grpc_routes(&Furnace) -> core::Result<tonic::service::Routes>` and hidden `GrpcRegistrationContext<'a> { application: &'a Furnace, config: &'a GrpcServerConfig, scope: &'a GrpcApplicationScope }`. Registrar signature is `fn(&GrpcRegistrationContext<'_>, tonic::service::Routes) -> core::Result<tonic::service::Routes>`. Generated registrar resolves one implementation handle, constructs its server, applies encoding/decoding limits, and installs it. Scoped guard wrapping is supplied by Task 5.

- [x] Add `managed_service_dispatches_all_rpc_shapes`: unary echoes 7; server stream yields [0, 1, 2]; client stream [2, 3, 5] returns count 3/total 10; bidi echoes [8, 13, 21] in order. Assert construction count 1 across concurrent calls. Add `oversized_messages_rejected_in_both_directions`, `unknown_wire_method_is_unimplemented`, and `cancelled_bidi_releases_handler_state`.

  ```rust
  assert_eq!(server_stream_values, [0, 1, 2]);
  assert_eq!((summary.count, summary.total), (3, 10));
  assert_eq!(bidi_values, [8, 13, 21]);
  ```
- [x] Run `cargo test -p furnace-rs-common --all-features --test grpc_dispatch`; initially fails at missing router/registrar behavior.
- [x] Implement typed generated server registration into `tonic::service::Routes`, retaining Tonic message framing, streams, backpressure and native application statuses. The test harness serves caller-built routes with a native Tonic loopback server at this stage; it does not introduce a second Furnace lifecycle owner.
- [x] Run the dispatch target. Expected: all shape/order/concurrency assertions pass, both configured 4194304-byte limits apply, cancellation joins handler tasks, and no graph planning occurs per call.
- [x] Commit `feat(grpc): dispatch unary and streaming methods through managed services`.

### Task 5: Protect gRPC with shared Passport and JWT

**Files:** Create common `src/grpc/passport.rs`; modify `src/grpc/{router,scope,preflight}.rs`, `src/passport/{scope,strategy}.rs`, `src/jwt/auto_configuration.rs`, shared/facade exports; create `tests/grpc_passport.rs`; extend UI and `grpc_jwt_only` consumer fixtures.

**Interfaces:** Consume Task 1 neutral authentication/policy and Task 3 static overrides. Produce `GrpcPassportService<S>` implementing the generated-server Tower service contract, `grpc_authenticated<P: PassportPrincipal, T>(&tonic::Request<T>) -> Result<Authenticated<P>, tonic::Status>`, `grpc_verified_token<C: Send + Sync + 'static, T>(&tonic::Request<T>) -> Result<VerifiedToken<C>, tonic::Status>`, and `passport_status(PassportError) -> tonic::Status`. Wrapped service preserves `NamedService::NAME` and clones safely.

- [x] Parameterize `grpc_guard_admission` over all four shapes and assert: missing/malformed/duplicate/expired/wrong-profile/invalid-signature credentials return `UNAUTHENTICATED`; role/permission/predicate rejection returns `PERMISSION_DENIED`; strategy operational failure returns generic `INTERNAL`. Handler/message-consumption counters remain zero for every rejection. Accessors return the exact principal and verified claims on success.

  Credential-rejection assertions:
  ```rust
  assert_eq!(status.code(), tonic::Code::Unauthenticated);
  assert_eq!(handler_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
  assert_eq!(consumed_messages.load(std::sync::atomic::Ordering::SeqCst), 0);
  ```
- [x] Add `method_policy_replaces_service_policy`, `explicit_public_method_bypasses_service_guard`, `admitted_stream_survives_token_expiry_but_next_call_fails`, `grpc_strategy_context_is_sanitized`, `cookie_guard_rejected_during_preflight`, and `unreachable_strategy_cannot_satisfy_grpc_guard`. Run the target and confirm failures before middleware.
- [x] Implement async admission before invoking the generated server; use request HTTP method POST, canonical RPC URI, sanitized metadata and trusted socket peer for PassportContext. Convert errors with fixed messages `unauthenticated`, `permission denied`, `internal server error`, never HTTP JSON/status codes. Preserve typed extensions through Tonic conversion.
- [x] Extend JWT auto-configuration requirements to the union of selected HTTP/gRPC guards, preserving explicit JWT providers, strategy type/profile checks and cauldron visibility. Task 7 adds selected reflection guards to the same union. Static validation must run before provider construction. Admission validates once per call and does not insert stream-expiry timers.
- [x] Run `cargo test -p furnace-rs-common --all-features --test grpc_passport`, existing Passport tests/UI, and the gRPC/JWT-only consumer on Rust 1.94. Expected: all guards work with Furnace `http` disabled and existing cookie/HTTP behavior remains intact.
- [x] Commit `feat(grpc): enforce Passport JWT policies at RPC admission`.

### Task 6: Build injectable named clients with readiness and outgoing credentials

**Files:** Create common `src/grpc/client.rs`, macro `src/grpc_client.rs`; update common/facade/macro exports, `src/grpc/{descriptor,preflight}.rs`; create common `tests/grpc_clients.rs`; add facade `ui-grpc` client fixtures and `grpc_clients` consumer.

**Interfaces:** Produce `GrpcChannel` implementing `tower::Service<http::Request<tonic::body::Body>, Response = http::Response<tonic::body::Body>, Error = GrpcClientError>` with a boxed Send future; `async fn GrpcChannel::from_config(GrpcClientConfig, Option<ErasedTokenProvider>) -> core::Result<Self>`; `GrpcChannel::ready(&self) -> impl Future<Output = core::Result<()>> + Send`; `GrpcChannel::close(&self)`; `GrpcTokenProvider` and `set_grpc_bearer_token` signatures exactly from the spec. `ErasedTokenProvider` is an internal Arc-held callback returning a boxed Send future with `GrpcClientResult<Secret<String>>`. Generated wrapper exposes `client(&self) -> GeneratedClient<GrpcChannel>`, cloned state, static `GrpcClientDescriptor` with name/output identity/location, and `Injector` dependencies `(Config,)` or `(Config, ProviderType)`.

- [ ] Add `two_typed_clients_target_distinct_servers` and assert independent values from identical protobuf service types. Add `lazy_unavailable_endpoint_does_not_block_startup`, `eager_unavailable_endpoint_rolls_back_startup`, `eager_readiness_precedes_application_hooks`, and `client_clones_allow_concurrent_calls` (barrier proves simultaneous handler entry). Keep eager readiness in infrastructure lifecycle, not in constructors/inspection.
- [ ] Add `per_call_token_suppresses_provider`, `provider_token_refreshed_per_call`, `duplicate_or_malformed_metadata_fails_without_provider_or_network`, `provider_failure_never_calls_remote_handler`, `anonymous_client_has_no_authorization`, and `client_does_not_forward_inbound_credentials`. Assert provider/server counters and redacted error formatting. Cover all four generated RPC methods through wrapper clients.

  In malformed-metadata and explicit-precedence tests respectively:
  ```rust
  assert_eq!(remote_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
  assert!(!error.to_string().contains(token_sentinel));
  assert_eq!(provider_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
  ```
- [ ] Add `per_call_deadline_overrides_client_default` verifying exact `grpc-timeout` metadata, and `response_stream_outlives_establishment_timeout` proving no accidental whole-stream deadline. Run `cargo test -p furnace-rs-common --all-features --test grpc_clients` and `cargo test -p furnace-rs --all-features --test grpc_ui`; confirm expected failures before client generation/middleware.
- [ ] Implement wrapper/macro generation and side-effect-free client descriptor validation, rejecting duplicate selected names. Shared channel state is constructed without connection I/O; lifecycle readiness connects only eager clients, applies 10000 ms connect deadlines and swaps in the live shared channel safely. Stop/drop releases channel-owned resources. Use standard Tonic reconnection; add no transparent retries to application RPCs.
- [ ] Implement async token/deadline middleware before remote dispatch. Honor Tower `poll_ready` on the actual service instance called; no shared lock spans provider futures or network awaits. Explicit Bearer parsing uses the shared strict parser; opaque token syntax is checked locally, JWT cryptography remains server-owned. Use fixed local errors when credentials cannot be prepared.
- [ ] Run both targets plus the named-client consumer on Rust 1.94. Expected: no startup I/O for lazy clients, eager failures roll back, explicit precedence and local failure counters pass, and streams/concurrency preserve native semantics.
- [ ] Commit `feat(grpc): add named DI clients with readiness and Bearer providers`.

### Task 7: Add verified TLS, public health, and guarded scoped reflection

**Files:** Create common `src/grpc/{tls,health,reflection}.rs`, macro `src/grpc_reflection.rs`; modify `src/grpc/{config,descriptor,preflight,router,passport}.rs`, macro/common/facade exports; create `tests/grpc_tls.rs`, `tests/grpc_infrastructure.rs`, `tests/support/tls.rs`; extend UI fixtures.

**Interfaces:** TLS helpers: `client_endpoint(&GrpcClientConfig) -> impl Future<Output = core::Result<tonic::transport::Endpoint>> + Send` and `configure_grpc_server(tonic::transport::Server, &GrpcServerConfig) -> impl Future<Output = core::Result<tonic::transport::Server>> + Send`. Runtime preparation invokes these file-reading helpers; pure parsing/inspection does not. Cloneable `GrpcHealth` owns a native `HealthReporter`; `async fn set_serving(&mut self, serving: bool)` updates selected fully qualified services and aggregate status. `GrpcReflectionDescriptor` stores root identity, static typed guard and source location. `reflection_descriptors(&GrpcApplicationScope) -> core::Result<prost_types::FileDescriptorSet>` returns the filtered, merged set. `install_grpc_infrastructure(&Furnace, &GrpcApplicationScope, Routes) -> core::Result<(Routes, GrpcHealth)>` installs health plus optional guarded reflection.

- [ ] Add TLS tests for plaintext success, trusted custom CA, WebPKI/custom additive configuration, custom-only root isolation, bad hostname, untrusted CA, mismatched key, partial TLS configuration and handshake timeout. Generate certificates locally in temporary directories. Assert key/token sentinels never appear in Display/Debug/config provenance. Run `cargo test -p furnace-rs-common --all-features --test grpc_tls` and confirm missing behavior first.
- [ ] Add health/reflection tests: anonymous `Check`/`Watch` succeed; disabled reflection returns `UNIMPLEMENTED`; enabled reflection without root policy/JWT/visible strategy fails preflight; unauthorized reflection returns `UNAUTHENTICATED`; configured policy enforces roles; authenticated reflection lists only selected services plus enabled infrastructure.
- [ ] Add `reflection_file_by_name_and_symbol_exclude_hidden_service`: request the file containing `Transport` and `Hidden`, decode returned descriptors, assert `Hidden` absent, imported message definitions present and source info absent. Conflicting same-name descriptor files fail with `FURNACE150`; identical definitions deduplicate. Run `grpc_infrastructure` and confirm expected failures before filtering/middleware.

  ```rust
  assert!(fds.file.iter().all(|file| file.source_code_info.is_none()));
  assert!(!fds.file.iter().flat_map(|file| &file.service)
      .any(|service| service.name.as_deref() == Some("Hidden")));
  ```
- [ ] Implement feature-gated native TLS setup, 10000 ms handshake deadlines, source-relative certificate loading, mandatory hostname/chain validation and custom-only root policy. Call the async TLS endpoint helper from Task 6's async channel constructor while preserving the rule that constructors do not connect remote endpoints; keep config parsing pure and connection readiness separate.
- [ ] Implement typed root reflection macro and preflight; add selected reflection guards to JWT auto-configuration requirements. Filter service declarations in descriptor files instead of merely hiding names in `list_services`. Install both native v1 and v1alpha reflection protocols under the same policy, so common tooling can query either. Add their descriptors only when enabled. Preserve public health admission independently from service guards.
- [ ] Run both integration targets, gRPC-only/JWT/TLS UI consumers, and `cargo check -p furnace-rs --no-default-features --features grpc,runtime-tokio`. Expected: plaintext works without TLS; enabled TLS honors trust; health/reflection agree with root scope and policy.
- [ ] Commit `feat(grpc): add verified TLS health and protected reflection`.

### Task 8: Select transports in burn and coordinate one lifecycle

**Files:** Create common `src/runtime/{error,coordinator}.rs`; modify `src/runtime/{mod,config}.rs`, `src/server.rs`, `src/server_config.rs`, `src/grpc/{router,health}.rs`, `src/lib.rs`, facade `src/lib.rs`; create `tests/grpc_runtime.rs`; extend automatic-run consumers.

**Interfaces:** Produce `ApplicationRuntimeError` with existing HTTP variants and new `GrpcBind(std::io::Error)`, `GrpcServe(tonic::transport::Error)`, `DrainTimeout`, and retained operation-plus-shutdown causes; `HttpRuntimeError` aliases it under `http`. `FurnaceBurnExt::burn<M: Cauldron>() -> impl Future<Output = Result<(), ApplicationRuntimeError>> + Send` works with either transport feature. `RuntimeRoutes::new()`, `with_http(axum::Router)` (HTTP gate), `with_grpc(tonic::service::Routes)` (gRPC gate); `RuntimeBindings { http: Option<SocketAddr>, grpc: Option<GrpcServerConfig> }`; `serve_transports(Furnace, RuntimeRoutes, RuntimeBindings, impl Future<Output = ()> + Send) -> impl Future<Output = Result<(), ApplicationRuntimeError>> + Send` is the explicit lifecycle-owning escape hatch. A private variant accepts async binders for deterministic bind-failure tests.

- [ ] Add `burn_configuration_selects_http_grpc_or_both`, `missing_transports_retains_http`, `uncompiled_transport_and_empty_application_fail_before_start`, and `mixed_transports_share_provider_identity`. Assert the provider reached by HTTP/gRPC handlers is the same instance, startup count 1, shutdown count 1. Run `cargo test -p furnace-rs-common --all-features --test grpc_runtime`; initially fails before the coordinator.
- [ ] Add `second_listener_bind_failure_releases_first_and_stops_once`, `first_server_failure_stops_sibling`, `eager_client_failure_binds_no_listener`, `partial_startup_retains_operational_and_shutdown_errors`, `shutdown_drains_then_cancels_after_30000ms`, and `health_changes_to_not_serving_before_drain`. Use injected binders, explicit shutdown, paused-time deadline checks, and rebind assertions; no timing guesses.

  ```rust
  assert_eq!(starts.load(std::sync::atomic::Ordering::SeqCst), 1);
  assert_eq!(stops.load(std::sync::atomic::Ordering::SeqCst), 1);
  assert!(tokio::net::TcpListener::bind(first_address).await.is_ok());
  ```
- [ ] Implement a common conventional source-loading/inspection path and selection preflight before `build`. Configure only selected listeners/infrastructure; preserve validation of registered managed-service metadata. Build routes, start shared infrastructure/application hooks exactly once, wait for eager readiness, bind both addresses, then admit requests. Print readiness only after successful binds; startup output identifies both protocols and their methods.
- [ ] On signal/error, mark health not-serving, stop admission, send graceful shutdown to both servers, wait for gRPC's configured drain deadline, cancel outstanding calls/tasks when it expires, then stop providers in reverse order once. Supervise active connection/call tasks and await their termination; dropping only the top-level serve future is insufficient evidence of cancellation. A bind failure releases any acquired listener and unwinds started providers. Preserve the original operation and any shutdown error. Do not introduce a timeout that prevents provider shutdown from being awaited.
- [ ] Preserve existing HTTP low-level `serve`/`serve_router`, trusted peer extensions, HTTP header/body read deadlines and router/CORS composition. Reuse the custom HTTP connection server rather than replacing it with a default Axum server. On gRPC connections, ensure native trusted peer address reaches the shared Passport context.
- [ ] Run `grpc_runtime`, existing server unit/security tests, and automatic HTTP/gRPC/mixed consumers on Rust 1.94. Expected: exact-once counters, listener release, deadline ordering, prior HTTP safeguards and source-compatible `Result<(), HttpRuntimeError>` examples pass.
- [ ] Commit `feat(runtime): select HTTP and gRPC with one lifecycle coordinator`.

### Task 9: Add offline gRPC CLI inspection without changing public JSON contracts

**Files:** Modify common `src/inspection.rs`, `src/runtime/{mod,coordinator}.rs`, `src/lib.rs`; modify CLI `Cargo.toml`, `src/{command,inspection,render,main}.rs`, `src/output/{model,human,json}.rs`; create CLI `tests/grpc_inspection.rs`, fixtures `tests/fixtures/inspection/grpc/{Cargo.toml,build.rs,src/main.rs,proto/service.proto,furnace.toml}`; extend existing protocol/JSON tests.

**Interfaces:** Existing hidden `InspectionKind` adds `Grpc`; private `INSPECTION_PROTOCOL_VERSION` advances from 2 to 3 (public JSON stays 2). `InspectionReport` adds `grpc_services: Vec<GrpcServiceReport>` with fields matching the spec. Produce public CLI data `GrpcData { services: Vec<GrpcServiceData> }`, nested `GrpcMethodData`/`GrpcProtectionData`, and `CommandData::Grpc`. Common inspection is compiled with either transport; HTTP report construction is gated separately. CLI may enable common `grpc` privately to deserialize reports; it need not enable TLS/JWT to display static policy records.

- [ ] Add `grpc_command_human_and_json_report_all_shapes_and_policies`: assert public envelope version 2, command `grpc`, exact field names, protobuf paths, streaming flags, sorted order, null public protection and guarded role/permission records. Add target-selector/unknown-argument/nonzero-diagnostic tests matching existing CLI rules.

  ```rust
  assert_eq!(json["schema_version"], 2);
  assert_eq!(json["command"], "grpc");
  assert!(json["data"]["services"].is_array());
  ```
- [ ] Add `grpc_inspection_never_constructs_or_connects_or_reads_tls_material`: fixture has a panicking constructor, eager unreachable clients, nonexistent TLS file paths, and public inspection metadata. `grpc`, `graph`, and `doctor` must return static results without executing the constructor or network/certificate loading. Invalid static configuration remains a diagnostic. Add `grpc_without_registered_services_reports_empty_array`.
- [ ] Run `cargo test -p furnace-rs-cli --test grpc_inspection`; confirm unsupported-command/missing-record failures before parser/output changes.
- [ ] Refactor inspection to use analysis, memoized selected metadata, neutral Passport preflight and pure config parsing. Extend protocol negotiation, retaining private tokens, owner-only temp directories, safe-file writes and explicit version-mismatch diagnostics. Older linked applications stay detectable: report a clear unsupported private protocol rather than executing them as a server.
- [ ] Add the new command and schema records; keep HTTP routes shape byte-for-byte compatible. `doctor` uses existing check records for gRPC selection/config/registration/policy findings; no new top-level JSON fields. Human output shows service, method, streaming shape and effective protection.
- [ ] Run new target, `inspection_protocol`, `inspection_cli`, and `json_cli`. Expected: new static reports pass, current HTTP JSON/selector behavior passes, and the mismatched private protocol is rejected clearly. Run platform-safe unit/parser tests on CI's existing Linux/macOS/Windows matrix.
- [ ] Commit `feat(cli): inspect gRPC methods and protection without startup I/O`.

### Task 10: Ship the example, docs, feature matrix, and complete verification

**Files:** Create independent `example/grpc-transport/{Cargo.toml,build.rs,proto/transport.proto,furnace.toml,README.md,src/main.rs,src/app.rs,src/service.rs,src/client.rs,src/auth.rs}` and its lockfile. Update `README.md`, `docs/{README,ARCHITECTURE,CLI}.md`, new `docs/grpc.md`, crate READMEs, `CHANGELOG.md`, facade `tests/feature_matrix.rs`, consumer manifests/tests, `.github/workflows/ci.yml` and existing release-workflow feature gates when required.

**Interfaces:** Document exactly the approved attributes, wrapper `client()`, Bearer helper/provider, configuration, runtime error alias, native routes escape hatch, health/reflection policies and stream/deadline semantics. Example defaults to gRPC-only with all four shapes; document switching to mixed mode and TLS using locally generated certificates. Include a consumer path that uses the generated client wrappers against the serving example. JWT demo credentials are runtime-provided or clearly scoped synthetic test keys; no private keys or tokens are committed.

- [ ] Add `grpc_feature_and_consumer_matrix` cases: no-default/core-only; unchanged default/HTTP-only; `grpc`; `grpc,jwt`; `grpc,grpc-tls`; mixed `http,grpc,jwt,grpc-tls`; direct/renamed common/facade; generic client type shared across distinct wrappers; explicit native provider overrides. Assert core dependency tree excludes transport/JWT/Serde and vendored compiler/build tools stay out of Furnace normal dependencies. Include compile-fail policy/descriptor grammar acceptance targets in CI.

  ```rust
  assert!(!core_normal_tree.contains("tonic v"));
  assert!(!core_normal_tree.contains("serde v"));
  assert!(!facade_normal_tree.contains("protoc-bin-vendored v"));
  ```
- [ ] Run the matrix target and record failing missing scenarios before finishing exports/docs/consumer wiring. Implement the example using application-owned `tonic-prost-build` plus vendored compiler configured safely through `Config::protoc_executable`. Exercise all shapes, protected calls, explicit client tokens, token provider, health and guarded reflection in a deterministic example integration test.
- [ ] Update architecture/feature docs and Unreleased changelog; document that client deadlines bound establishment, authenticated streams retain principals, reflection does not authorize invocation, HTTP-only defaults are preserved, and custom roots can be isolated. Update conventional configuration/startup and CLI instructions without adding gRPC scaffolding or changing released-version guidance.
- [ ] Run focused example/matrix checks: `cargo test -p furnace-rs --all-features --test feature_matrix`; `cargo test --manifest-path example/grpc-transport/Cargo.toml`; and `cargo +1.94.0 check --locked --manifest-path example/grpc-transport/Cargo.toml`. Expected: all pass with no system protoc installation.
- [ ] Run full workspace gates once focused tasks pass: `cargo fmt --all --check`; `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`; `cargo test --locked --workspace --all-features`; `cargo test --locked --workspace --all-features --doc`; `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps`; `cargo +1.94.0 test --locked --workspace --all-features`; `cargo package --locked --workspace --no-verify`; `cargo audit --no-yanked --file Cargo.lock` where installed. Execute existing coverage/security gates from CI for changed integration surfaces; report prerequisites and evidence rather than claiming an unavailable gate passed.
- [ ] Review the entire branch against the 12 acceptance criteria and five Review Focus cases. Check docs/config/macros for matching names and status-code behavior. Resolve actual failures; raise any newly discovered requirement ambiguity with the user. Do not merge, publish or deploy as part of this plan.
- [ ] Commit `docs(grpc): document transport APIs and verify feature compatibility` with only the remaining example/docs/matrix/workflow changes.

## Acceptance Coverage and Handoff

| Spec acceptance criterion | Task / proving target |
|---|---|
| 1: Four RPC shapes serving and consuming | 4, 6, 10: `grpc_dispatch`, `grpc_clients`, example tests |
| 2: Independent features and neutral core | 1, 2, 10: `feature_matrix`, MSRV consumers |
| 3: Ownership/DI/metadata validation | 3, 5: `grpc_ui`, scope tests, preconstruction sentinels |
| 4: Exactly-once lifecycle and rollback | 6, 8: readiness and `grpc_runtime` |
| 5: Eager versus lazy availability | 6: client readiness tests |
| 6: Verified TLS and invalid configuration | 2, 7: config and `grpc_tls` |
| 7: All-shape policies and typed principals | 5: `grpc_passport` |
| 8: Stream survival and subsequent expiry rejection | 5: deterministic expiry regression |
| 9: Token precedence and distinct clients | 6: token counters and typed client tests |
| 10: Health/reflection access and filtering | 7, 8: infrastructure and drain tests |
| 11: Side-effect-free CLI | 9: panicking constructors, eager clients and unreadable-file fixture |
| 12: Existing HTTP/CLI behavior | 1, 8, 9, 10: existing tests and workspace gates |

Published release/API checks support the chosen library lines: [Tonic 0.14.6 and its declared Rust floor](https://docs.rs/crate/tonic/0.14.6), [Prost 0.14.4](https://docs.rs/crate/prost/0.14.4), [Tonic Prost build APIs](https://docs.rs/tonic-prost-build/latest/tonic_prost_build/struct.Builder.html), [safe protoc executable configuration](https://docs.rs/prost-build/0.14.3/prost_build/struct.Config.html#method.protoc_executable), and [vendored compiler 3.3.0](https://docs.rs/crate/protoc-bin-vendored/3.3.0). These checks are not a substitute for locked dependency builds on Rust 1.94.

**Execution approved:** The user selected native execution after receiving the plan. Implement the tasks in this session, obtain a final independent review, and preserve `feat/grpc-method`. Raise newly discovered requirement ambiguity before choosing behavior.
