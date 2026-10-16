# WebSocket Server Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved server-only WebSocket transport, including managed gateways, owned sockets, optional distributed Redis rooms, replay, and Furnace lifecycle/CLI integration.

**Architecture:** Axum performs upgrades on a separate supervised listener. Static descriptors select DI-managed handlers by cauldron; managed connection dispatch uses a shared sending/room service. Memory implements live local rooms, Redis Pub/Sub implements distributed live rooms, and independent Redis Stream readers implement replayable memberships.

**Tech Stack:** Rust 2024, Rust 1.94 minimum, existing Axum 0.8.9, Tokio, Hyper, Serde, inventory, proc-macro-crate, rustls/tokio-rustls, redis-rs, Redis Pub/Sub and Streams, tokio-tungstenite for protocol tests.

**Spec:** [Approved design](../specs/2026-10-17-websocket-transport-design.md).

## Global Constraints

- Work on the assigned `feat/websocket-transport` branch; do not create another branch/worktree.
- Use `furnace-rs-socket` and `furnace-rs-socket-macros`, workspace version `2.0.0-beta.3`, existing workspace lints, and no unsafe code.
- Server endpoints only; separate explicit listener address; standard WebSocket; Axum internally.
- Standalone optional `jwt`, `tls`, `redis` features; no default features. Facade `socket`, `socket-tls`, `socket-redis`; optional JWT forwarding.
- No implicit HTTP application routes, gRPC, database, or Redis dependency for a basic socket application.
- Passport only at handshake; Bearer/cookie/query sources; reject ambiguous credentials without fallback.
- Supplied origins require an explicit allowlist match; absent Origin is allowed.
- Live/replayable mode is fixed per dynamic room; replayable rooms require both explicit age and count retention limits.
- Replay only named JSON room events; no client acknowledgement tracking; no exactly-once guarantee.
- Room joins require an application approval hook, including before replay. Authorized joins may read any retained history.
- Redis is one endpoint with authentication and verified TLS; no Sentinel or cluster-aware routing.
- Reject distributed room/broadcast sends during Redis outages; preserve local connections; local direct sends continue.
- Per-connection gateway dispatch is sequential by default; concurrent mode defaults to 16 active handlers.
- Incoming limit 1 MiB; outgoing limit 256 messages AND 4 MiB; disconnect on exhaustion.
- Ping interval 30s, pong timeout 10s, shutdown timeout 10s; lease refresh 5s, expiry 15s; configurable.
- CLI inspection must not instantiate providers, bind listeners, read TLS material, or contact Redis.
- No deployment, publishing, or remote changes are part of execution.

## Review Focus

1. A fragmented WebSocket message must hit the aggregate 1 MiB limit; control frames must remain responsive while application handlers are busy (Tasks 3 and 5).
2. Duplicate/encoded credential fields and Origin variants must not bypass admission or leak tokens (Task 6).
3. Simultaneous join/delete/recreate across processes and expired leases must not admit members to a deleted room or reuse stale cursors (Tasks 8 and 9).
4. Redis disconnects between append, reply, subscribe, and lease renewal must produce explicit uncertainty and preserve recovery boundaries (Tasks 8 and 9).
5. Conditional and renamed macro consumers, plus old CLI reports, must preserve correct scope and compatibility without activating transports (Tasks 4, 10, and 11).

## File and interface map

Create `crates/furnace-rs-socket/` with `src/lib.rs`, `config.rs`, `error.rs`,
`protocol.rs`, `descriptor.rs`, `scope.rs`, `context.rs`, `dispatch.rs`,
`admission.rs`, `rooms/{mod,memory,redis,replay}.rs`,
`rooms/scripts/{create,join,leave,delete,publish,lease,trim}.lua`, and
`runtime/{mod,adapter,server,tls,inspection}.rs`. Keep protocol parsing, admission,
transport serving, and storage behavior in their respective modules.

Create `crates/furnace-rs-socket-macros/` with `src/{lib,path,gateway,endpoint}.rs`.
Create focused socket integration tests and `tests/support/{mod,server,redis,tls}.rs`.
Keep consumer and compile-fail fixtures under the existing facade test structure.

Modify existing workspace/facade manifests and exports, core cauldron macros,
shared runtime and inspection modules, CLI command/output modules, and workflows.
Add `docs/socket.md` and independent `example/socket-transport/`.

Public foundation types used below:

- `SocketResult<T> = Result<T, SocketError>`; `SocketError::code() -> &'static str`. Re-export Axum's `Message` and `WebSocket` for the signatures below.
- `EndpointMode::{Raw, Events}`; `ProcessingMode::{Sequential, Concurrent { max_concurrency: usize }}`.
- `SocketServerConfig::parse(&Config) -> furnace_rs_core::Result<Self>`.
- `SocketEnvelope { event: String, id: Option<String>, data: serde_json::Value, room: Option<String>, cursor: Option<String> }`.
- `RoomMode::{Live, Replayable { max_age: Duration, max_messages: usize }}`.
- `SocketHub`: cloneable DI-managed interface; application code scopes it with `gateway(path: &str) -> SocketGatewayHub`.
- `SocketGatewayHub::{create_room(&str, RoomMode), delete_room(&str), send(&ConnectionId, Message), send_room(&str, Message), emit_room(&str, &str, Value), broadcast(Message)}` are async and return `SocketResult<()>`, except `emit_room` returns `SocketResult<Option<String>>` (cursor for replayable rooms).
- `SocketContext::{id(), handshake(), hub()}` expose current connection identity, handshake metadata/authentication, and its gateway-scoped hub. Async `send(Message)`, `emit(&str, Value)`, `join(&str, Option<&str>)`, and `leave(&str)` return `SocketResult<()>`; joining always passes the authorization hook.
- JSON event handlers receive `(SocketContext, serde_json::Value)`; raw handlers receive `(SocketContext, String)` or `(SocketContext, axum::body::Bytes)`; connect/disconnect receive `SocketContext`; join hook receives `(SocketContext, String)` and returns `SocketResult<()>` (success grants access).
- All handler methods are async `&self` methods on an `Injector`; macros generate typed adapters returning `BoxFuture`. Owned endpoint: async `&self` method receiving `(SocketHandshake, axum::extract::ws::WebSocket)` and returning `SocketResult<()>`.

These exact signatures make implementation tasks interoperable. The design review
approved the API structure; plan review includes these concrete signatures.

## Task 1: Configuration, wire protocol, and crate boundaries

**Files:** Create transport/macro manifests and minimal `src/lib.rs`; transport `config.rs`, `error.rs`, `protocol.rs`; `tests/config_protocol.rs`. Modify workspace `Cargo.toml`, `Cargo.lock`, facade `Cargo.toml` and `src/lib.rs`.

**Interfaces:** Produce foundation config/error/envelope/mode types from the map. `SocketEnvelope::parse(&str) -> SocketResult<Self>` and `to_text(&self) -> SocketResult<String>`. Config retains private secret-bearing Redis/TLS settings and exposes redacted `Debug`.

- [ ] Write failing tests `defaults_match_approved_limits`, `requires_explicit_address`, `invalid_limits_rejected`, `protocol_roundtrip`, `reserved_events_rejected_for_application_registration`, and `configuration_debug_redacts_secrets`. Assert the exact global defaults and optional envelope fields; distinguish client fields from server-supplied room/cursor metadata.
- [ ] Run `cargo test -p furnace-rs-socket --test config_protocol`; expect missing crate/types initially, then assertion failures before implementation.
- [ ] Add both workspace crates, implement parsing and documented types, and establish optional dependencies and facade feature/module wiring needed by later macro consumers. Enable Axum `ws` only for socket consumers. Use existing dependency versions for common libraries; verify the redis-rs version/features and Rust floor before locking it. Standalone `jwt` uses neutral Passport support rather than HTTP cookie routing. Declare test-only tokio-tungstenite compatible with Axum's underlying protocol version. Gate integration tests on required features so no-default-feature test runs do not compile unavailable JWT/TLS/Redis APIs. Require an explicit namespace when Redis is configured to prevent accidental application overlap.
- [ ] Run the test and `cargo check -p furnace-rs-socket --no-default-features`; require success without Redis/JWT/TLS activation. Record dependency version and MSRV evidence.
- [ ] Commit the tested foundation as `feat(socket): add configuration and event protocol`.

## Task 2: Transport selection and selected registration scope

**Files:** Modify common `src/runtime/{config,burn}.rs`, `src/inspection.rs`; gRPC `src/runtime/inspection.rs`, `src/grpc/config_tests.rs`; create socket `src/{descriptor,scope}.rs`, `tests/registration.rs`; extend common runtime selection tests.

**Interfaces:** `RuntimeSelection::parse_registered(&Config, &[&str]) -> core::Result<Self>` adds a public `socket: bool` selection field while retaining the existing `parse(config, compiled_http, compiled_grpc)` entry point as an HTTP/gRPC compatibility wrapper. Update existing struct literals for the new false field. `SocketApplicationScope::for_cauldron_graph(Option<&CauldronGraph>) -> core::Result<Self>` and `for_application(&Furnace) -> core::Result<Self>` expose validated `SocketEndpointDescriptor`s selected through `TransportProvider::TRANSPORT = "socket"`.

- [ ] Write failing tests for socket-only and mixed selection, uncompiled/duplicate selections, preserved HTTP defaults, reachable versus unrelated registrations, duplicate paths/events, reserved prefixes, and no-runnable-endpoint diagnostics. Use test-only explicit descriptors before macros exist.
- [ ] Run `cargo test -p furnace-rs-socket --test registration` and targeted common runtime tests; verify failures identify missing selection/scope behavior.
- [ ] Implement registered selection and migrate internal callers, including gRPC inspection, away from assumptions that only HTTP/gRPC exist. Descriptor fields hold path, mode, processing, source, provider identity, handshake guard, method names, and typed provider-resolution adapters. Scope validation never constructs providers.
- [ ] Run registration/common tests and existing `cargo test -p furnace-rs-grpc --test grpc_runtime`; verify old parsing behavior remains supported.
- [ ] Commit as `feat(socket): register selected socket endpoints`.

## Task 3: Supervised listener and owned sockets

**Files:** Create socket `src/runtime/{mod,adapter,server}.rs`, `src/context.rs`, `tests/owned_endpoint.rs`, `tests/runtime.rs`, `tests/support/{mod,server}.rs`.

**Interfaces:** `build_socket_routes(&Furnace) -> core::Result<axum::Router>`; `RuntimeSocketExt::with_socket` implemented for `RuntimeRoutes` (Axum router) and `RuntimeBindings` (`SocketServerConfig`). `SocketHandshake` exposes request metadata, peer address, application context, and extension access. Runtime adapter registers under `socket` and uses `PreparedTransport`/`RuntimeTasks`.

- [ ] Write protocol tests for real owned text/binary echo, failed upgrade, fragmented aggregate-size rejection, socket-only startup, separate HTTP/socket ports, bind rollback, and an owned handler that never returns but is cancelled/joined on shutdown.
- [ ] Run `cargo test -p furnace-rs-socket --test owned_endpoint --test runtime`; require expected failures before adding serving behavior.
- [ ] Serve Axum upgrades over Hyper with upgrades enabled; retain connection and upgraded handler tasks in the shared runtime. Supervise the actual owned future even if Axum internally spawns its upgrade callback. Apply message/frame limits before transfer. Gate new upgrades on readiness/shutdown and carry actual peer metadata. Require selected registrations before startup.
- [ ] Run both suites, ensuring no listener or upgraded task survives shutdown. This task's fixture descriptors invoke owned handlers without macro dependency.
- [ ] Commit as `feat(socket): serve supervised owned websocket endpoints`.

## Task 4: Gateway/endpoint macros and cauldron declarations

**Files:** Create macro `src/{path,gateway,endpoint}.rs`; modify core macros `src/path.rs`, `src/cauldron.rs`, `src/cauldron/{declaration,contract}.rs`; create socket `tests/macros.rs`; facade `tests/socket_ui.rs`, `tests/ui-socket/`, and consumer fixtures for standalone/renamed socket dependencies.

**Interfaces:** `#[socket_gateway]` decorates inherent impls using map signatures; `#[socket_endpoint]` decorates an impl containing one owned handler. Add explicit `socket_gateways = [...]` and `socket_endpoints = [...]` cauldron lists; both register transport providers and participate in existing DI contract/exports. Attribute parameters select path/mode/processing and optional `guard = Type`; protected registration requires `jwt`. Helper handler attributes are consumed by the enclosing macro.

- [ ] Write compile-pass cases for standalone/facade/renamed dependencies and conditional attributes; compile-fail cases for missing Inject, incorrect argument/return types, duplicate handlers, incompatible modes, invalid concurrency, reserved events, and unsupported options. Assert excluded conditional registrations do not leave phantom descriptors.
- [ ] Run macro unit tests and `cargo test -p furnace-rs --test socket_ui --all-features`; establish expected compilation failures before expansion implementation.
- [ ] Implement hygienic crate lookup following gRPC patterns; generate static descriptors, typed DI adapters, and `TransportProvider` impls. Extend every cauldron provider count, leaf, output, import/export, and duplicate calculation for both lists. Do not bypass existing compile-time dependency contracts.
- [ ] Run macro/UI tests and existing core/DI UI tests; review compiler stderr snapshots rather than accepting them blindly.
- [ ] Commit as `feat(socket): add gateway and endpoint registration macros`.

## Task 5: Managed dispatch, queues, and heartbeats

**Files:** Create socket `src/dispatch.rs`; extend `src/context.rs`, `runtime/adapter.rs`; create `tests/gateway.rs`, `tests/backpressure.rs`.

**Interfaces:** Produce `SocketHub`, `SocketGatewayHub`, `ConnectionId`, and `SocketContext` from the map, initially with direct sends and backend delegation points. Private `run_gateway` owns the socket, receives control frames independently of handler work, and supervises handler futures.

- [ ] Write tests for raw text/binary dispatch, JSON handler replies/request IDs, recoverable errors, generic internal errors, connect/disconnect hooks, sequential ordering, max-16 concurrent work, ping/pong under blocked handlers, and both byte/count queue limits. Assert peer disconnect/shutdown cancels handlers and calls disconnect once.
- [ ] Run `cargo test -p furnace-rs-socket --test gateway --test backpressure`; require behavior failures before implementation.
- [ ] Implement one bounded outbound queue with byte accounting released after write/cancellation. Preserve control-frame handling while handlers run; bound handler admission without an auxiliary unbounded pending list. Close a slow client on queue exhaustion; promptly cancel work on shutdown/heartbeat failure. Protect the writer from hanging beyond drain deadlines. Keep lifecycle hooks separate from concurrent message execution.
- [ ] Run the suites with deterministic Tokio time where possible and real sockets for protocol behavior. Confirm JSON errors never appear on raw endpoints.
- [ ] Commit as `feat(socket): dispatch managed gateway connections`.

## Task 6: Passport handshake and Origin admission

**Files:** Create socket `src/admission.rs`, `tests/admission.rs`; extend descriptor/macro/context modules; reuse common neutral Passport internals without enabling its HTTP feature.

**Interfaces:** Private async `admit(application, descriptor, request_metadata) -> SocketResult<SocketHandshake>` installs typed authentication extensions before upgrade. Typed `SocketHandshake::authenticated<P: PassportPrincipal>() -> SocketResult<Authenticated<P>>` is available with `jwt`. Query/cookie names default to `access_token`, configurable on protected registrations.

- [ ] Write handshake-status tests for each credential source, missing/expired/invalid credentials, all multi-source pairs, identical tokens, repeated headers/cookies/query values, percent encoding, malformed cookie syntax, unrelated cookies, and public versus guarded routes. Cover exact origins, schemes/ports, malformed/multiple Origin, empty allowlist, and absent Origin. Capture logs/diagnostics and assert token values are absent.
- [ ] Run `cargo test -p furnace-rs-socket --features jwt --test admission` and verify admission failures before implementation.
- [ ] Implement single-source extraction and shared Passport strategy resolution within the endpoint's cauldron. Do not forward token-bearing request URLs into diagnostics. Validate Origin before upgrade independently of Passport. Provide typed principal access throughout the connection without per-event reauthentication.
- [ ] Run tests with `jwt`, without `jwt`, and dependency-tree checks proving cookie admission does not activate the Furnace HTTP cookie adapter.
- [ ] Commit as `feat(socket): authenticate websocket handshakes`.

## Task 7: In-memory rooms and authorized built-in commands

**Files:** Create socket `src/rooms/{mod,memory}.rs`, `tests/rooms.rs`; extend hub/context/dispatch.

**Interfaces:** Implement the map's hub operations. A private backend interface supplies room create/delete/join/leave/publish behavior without becoming a public pluggable-backend API. Dispatch `furnace.join`/`furnace.leave`; `on_join` approval is required for both client and server-context joins.

- [ ] Write tests for idempotent create, conflicting settings, nonexistent rooms, missing/denying/allowing join hooks, leave/disconnect cleanup, per-gateway/namespace isolation, service-originated sends, empty-only deletion, and broadcast inclusion exactly once. Assert replayable room creation fails explicitly without Redis and raw live room sends preserve frame type.
- [ ] Run `cargo test -p furnace-rs-socket --test rooms`; establish failing behavioral assertions.
- [ ] Implement synchronized local room definitions/memberships and delegate hub sending through bounded connection queues. JSON join acknowledgement occurs after approved membership; private message errors preserve request IDs. Room names do not create rooms automatically. Replay metadata is server-generated and cannot be forged by a client envelope.
- [ ] Run room tests and gateway suites; verify rejected joins never receive room data.
- [ ] Commit as `feat(socket): add authorized local rooms and broadcasting`.

## Task 8: Redis live delivery, leases, and room lifecycle

**Files:** Create socket `src/rooms/redis.rs`, scripts `create.lua`, `join.lua`, `leave.lua`, `delete.lua`, `publish.lua`, `lease.lua`; create `tests/redis_live.rs`, `tests/support/redis.rs`; extend runtime preparation.

**Interfaces:** Private async `RedisBackend::prepare(config, tasks) -> SocketResult<Self>` establishes command and Pub/Sub channels before readiness. Both backends implement Task 7's operations. Redis room metadata includes immutable mode/settings and a new generation per recreation; membership references expiring instance leases and current generation.

- [ ] Write real Redis tests for two-instance fan-out, raw text/binary frames, no duplicate local sends, concurrent conflicting create, join/delete races, active-member deletion rejection, crash/expired lease cleanup, stale-instance fencing, gateway/namespace isolation, startup failure, outage/direct-send behavior, and reconnect/resubscribe readiness. Inject a publish reply loss and assert no application-level retry.
- [ ] Run `cargo test -p furnace-rs-socket --features redis --test redis_live`; the fixture spawns an isolated Redis process on a temporary port/directory. Missing Redis is an explicit infrastructure failure for this suite, not a passing skip.
- [ ] Implement atomic Lua operations for definition, generation, membership, empty deletion, and publish validation. Use a namespace-scoped live Pub/Sub channel with explicit gateway/room/generation targets; route only to approved local members. Publish once through Redis, including sender-instance delivery via its subscription. Use a `MultiplexedConnection` for publish/append commands with reconnect managed outside individual operations, avoiding a manager that automatically retries an uncertain write. Do not separately fan out locally. Never accept stale leases as authorization to operate. Validate lease timing against Redis server time; restore membership under a renewed lease before reopening operations after reconnect.
- [ ] Supervise command/subscription health and lease renewal. Backend readiness requires all three; outage returns `backend_unavailable` while preserving connections. Use bounded reconnect backoff; cap attempts by shutdown cancellation rather than leaking tasks.
- [ ] Run Redis and local room suites, checking each child Redis process is terminated and temporary state removed. Commit as `feat(socket): distribute live rooms through redis`.

## Task 9: Replayable rooms, retention, and gap-free resume

**Files:** Create socket `src/rooms/replay.rs`, `rooms/scripts/trim.lua`, `tests/redis_replay.rs`; extend Redis scripts/publisher and event dispatch.

**Interfaces:** `emit_room` atomically appends replayable events and returns their stream cursor. Private paged history reads expose room generation, highest published ID, and highest trimmed ID. Each replayable membership has an independent stream read cursor; Redis consumer groups are not used for fan-out.

- [ ] Write tests for age AND count trimming, valid resume, expired/malformed/future cursors, room recreation, replay after a crash, independently reconnecting clients, hook-before-history, newer messages published during replay, simultaneous publishers, retention overtaking a slow replay, and Redis outages at append/read/acknowledgement boundaries. Assert per-room order, no replay/live gap, bounded pages, and no retained direct/global/raw messages.
- [ ] Run `cargo test -p furnace-rs-socket --features redis --test redis_replay`; verify assertions fail before implementing history behavior.
- [ ] Append and trim atomically using Redis server time; maintain the highest removed ID, room creation boundary, and current generation outside the trimmed stream. Reject a requested cursor beyond the captured high-water ID as `invalid_cursor`. Enforce exact count/age visibility even when cleanup runs on reads or periodic maintenance. An expired history result must remain distinguishable when all entries have been trimmed.
- [ ] On approved join, capture generation and high-water ID. For an explicit cursor, page entries strictly after it through that boundary while validating the retention watermark; without a cursor, begin at the boundary without historical replay. Enqueue join acknowledgement after catch-up. Continue independent bounded `XREAD` from that boundary for live stream entries; this avoids a Pub/Sub handoff gap. If trimming overtakes the reader, report history expiration rather than skipping. Verify room generation throughout. Terminate readers on leave, disconnect, deletion, and shutdown.
- [ ] Read stream entries in Redis order for both normal delivery and recovery. After backend reconnection, resume from the last enqueued cursor, subject to retention; client reconnection still uses its last processed cursor. Pub/Sub is only the live-room transport, not the source of replayable delivery order. Do not promise client acknowledgement or retry uncertain appends.
- [ ] Run replay/live/local room suites. Commit as `feat(socket): replay retained room events with resume cursors`.

## Task 10: Native TLS and shared lifecycle completion

**Files:** Create socket `src/runtime/tls.rs`, `tests/tls.rs`, `tests/support/tls.rs`; extend runtime/config and `tests/runtime.rs`; modify facade manifest/exports and `tests/socket_runtime.rs`.

**Interfaces:** TLS configuration uses certificate/private-key PEM paths and a bounded handshake timeout following existing gRPC conventions. `prepare_tls(&SocketServerConfig)` loads and validates identity before any listener is ready. Public facade re-exports socket APIs under `socket`; `runtime` re-exports shared types without introducing ambiguous extension traits.

- [ ] Write tests for valid WSS, wrong-host/untrusted certificate rejection by clients, malformed/missing/mismatched server identity, bounded/cancelled incomplete TLS handshake, ordinary proxy-backend WS, three-transport configuration, failed-prepare rollback, and final provider shutdown after socket/Redis tasks finish.
- [ ] Run `cargo test -p furnace-rs-socket --all-features --test tls --test runtime` and facade socket runtime tests; establish failing integration paths.
- [ ] Implement rustls identity validation and HTTP/1.1 upgrades over TLS using the supervised listener. Do not copy gRPC's h2-only ALPN. Ensure failed preparation cleans already-created Redis tasks and resources even when application serving never begins. Forward optional facade features with weak dependency activation for `jwt`. Keep owned socket futures abortable at shared drain timeout.
- [ ] Run suites and existing HTTP/gRPC runtime regression tests. Commit as `feat(socket): integrate tls and shared application lifecycle`.

## Task 11: Static inspection and CLI command

**Files:** Create socket `src/runtime/inspection.rs`; modify common `src/inspection.rs`, `src/lib.rs`; CLI `src/{command,inspection,project,render,lib}.rs`, `src/output/{mod,model}.rs`; create CLI `tests/socket_inspection.rs`, fixture `tests/fixtures/inspection/socket/`; update exhaustive report literals and protocol fixtures.

**Interfaces:** Add `InspectionKind::Socket`, a defaultable `socket_endpoints` field in reports, and neutral `SocketEndpointReport`/handler records. Introduce a separate neutral socket inspection descriptor so existing gRPC descriptor callback signatures remain compatible. `furnace socket` supports existing package/bin selection and text/JSON conventions.

- [ ] Write parser/render/JSON tests, selected-root and renamed/standalone consumer fixtures, unsupported-report handling, and a fixture whose provider panics if instantiated. Configure unavailable Redis and nonexistent TLS paths; inspection must still succeed when static metadata/configuration is valid. Verify old reports deserialize with empty socket data.
- [ ] Run `cargo test -p furnace-rs-cli --test socket_inspection` and relevant CLI parser/report unit tests; confirm expected missing-command/report failures.
- [ ] Wire static descriptors, reports, doctor checks, CLI selection, direct standalone transport dependency detection, and facade exports. Preserve existing private protocol compatibility for prior report kinds; recognize socket support explicitly rather than treating absent new fields as proof of support. Add socket scope/policy metadata without touching runtime room state.
- [ ] Run new and existing `grpc_inspection`, `inspection_cli`, and `json_cli` tests. Commit as `feat(cli): inspect websocket gateways and endpoints`.

## Task 12: Consumer matrix, example, documentation, and release integration

**Files:** Extend facade `tests/{feature_matrix,path_resolution}.rs` and socket consumer manifests; create `example/socket-transport/{Cargo.toml,Cargo.lock,README.md,furnace.toml,src/,tests/}`; create `docs/socket.md`, both new crate READMEs; update root/facade/docs/example READMEs; modify `.github/workflows/{ci,beta-publish,stable-publish}.yml`.

**Interfaces:** Independent example uses the same public macros/context/hub APIs; no private test-only route injection. Redis integration runs in an explicit CI service/job or installs the executable for local-isolated fixture tests. Add macro crate before transport crate before facade to publish ordering; do not publish during execution.

- [ ] Write consumer checks for standalone, renamed, facade, socket-only/JWT/TLS/Redis combinations, and conditional registrations. Assert basic socket trees omit Redis, JWT, gRPC, database, and Furnace HTTP routing extras; existing default/JWT-only trees omit socket support. Write runnable example protocol tests covering join, live send, and cursor resume with two server instances.
- [ ] Run focused consumer/example tests before completing exports/docs, then implement the missing public integration and example. Document all approved settings, signatures, built-in wire messages/error codes, room create/delete semantics, authorization, ownership boundaries, queue behavior, credentials, origins, TLS/proxy use, Redis outages, lease delays, retention, and duplicates.
- [ ] Add CI feature checks, actual Redis integration, standalone example verification, and MSRV coverage. Include docs/package checks for the new crates and update publish order without changing remote release state.
- [ ] Run the final verification commands below once, resolving failures with focused reruns. Review the branch against the approved spec, including the five review-focus conditions. If the chosen execution method includes an independent reviewer, request that review and address verified findings before final verification.
- [ ] Commit as `docs(socket): document and verify websocket transport` and report changes, test evidence, and remaining limitations. Do not merge or publish.

## Final verification commands

```bash
cargo fmt --all --check
cargo test --locked -p furnace-rs-socket --no-default-features
cargo test --locked -p furnace-rs-socket --all-features
cargo test --locked -p furnace-rs-socket-macros
cargo check --locked -p furnace-rs-socket --no-default-features --features jwt
cargo check --locked -p furnace-rs-socket --no-default-features --features tls
cargo check --locked -p furnace-rs-socket --no-default-features --features redis
cargo check --locked -p furnace-rs --no-default-features --features socket,runtime-tokio
cargo check --locked -p furnace-rs --no-default-features --features socket,jwt,socket-tls,socket-redis,runtime-tokio
cargo test --locked --manifest-path example/socket-transport/Cargo.toml --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo test --locked --workspace --all-features --doc
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps
cargo package --locked --workspace --no-verify
```

Run workspace and example checks on Rust 1.94 as well when the toolchain is
available; CI must include it. Protocol tests require permission to bind local
listeners. Redis tests require the actual isolated server and should not report
success after skipping unavailable infrastructure. Distinguish baseline failures
from new regressions with targeted evidence. Use package/fixture commands for
intermediate task verification rather than repeating the entire workspace suite.

## References verified while planning

- [Axum WebSocketUpgrade](https://docs.rs/axum/0.8.9/axum/extract/ws/struct.WebSocketUpgrade.html): upgrade limits and native socket integration.
- [redis-rs async APIs](https://docs.rs/redis/latest/redis/aio/): separate Pub/Sub and command connections; reconnect behavior must be reviewed before selecting a manager for non-retryable publishes.
- [Redis XREAD](https://redis.io/docs/latest/commands/xread/): independent stream reads after a supplied ID, used for replay and live continuation.

## Review and execution handoff

All twelve tasks implement one cohesive approved transport subsystem; later
tasks depend on the foundation and storage contracts above. Review the concrete
signatures, cauldron option names, and Redis replay algorithm in this plan before
execution. Choose native execution in this session or subagent-driven execution.
Native execution is recommended because the tasks share evolving Rust contracts
and workspace-wide macro/runtime integration; it avoids repeated context handoff.
