# WebSocket server transport for Furnace

Date: 2026-10-17
Branch: `feat/websocket-transport`
Status: Approved design; implementation and final verification complete.

## Intent and success criteria

Add standard WebSocket server endpoints to Furnace through a new
`furnace-rs-socket` crate. Applications can choose managed gateways or handlers
that own the socket. Gateways integrate with dependency injection, selected
cauldrons, Passport admission, rooms, broadcasting, and optional Redis-backed
live or replayable delivery. The separate listener participates in the existing
application startup, readiness, drain, and provider shutdown lifecycle.

The user requested design clarification before implementation, without silently
choosing design defaults. The decisions below consolidate the conversation.
The complete written design must be reviewed before implementation planning.
Details elaborated here beyond the conversational decisions are proposals for
that review, not permission to implement unreviewed behavior.

Success includes standalone and facade consumption, actual WebSocket client
interoperability, cross-instance Redis behavior, CLI inspection, documentation,
and a runnable example. Existing HTTP and gRPC behavior must continue to work.

## Scope and boundaries

- Standard WebSocket using Axum's WebSocket support internally.
- Server endpoints only, on a separate configured listener.
- Native TLS and TLS termination at a reverse proxy are both supported.
- Raw text/binary gateway endpoints and named JSON event gateway endpoints.
- Each endpoint explicitly selects raw or event mode; text content does not
  implicitly switch protocols.
- Direct endpoints take ownership of the socket and implement their own loop.
- Dynamic application-created rooms, individual sends, room sends, and broadcasts.
- Optional Redis: in-memory live delivery for one instance; Redis for
  cross-instance delivery and replay.
- One configured Redis endpoint, including authentication and `rediss://` TLS.
- Sentinel discovery and cluster-aware routing are outside this version.
- Replay is for named JSON room events, with client-held cursors.

Raw socket ownership does not run gateway dispatch, built-in room commands,
managed heartbeats, or replay. The listener's incoming message limit still
applies, and the owning task remains supervised by the application runtime.

## Crates and integration

`furnace-rs-socket` owns transport configuration, route construction, socket
types, dispatch, admission, room operations, delivery backends, and runtime
integration. `furnace-rs-socket-macros` supplies registration attributes,
re-exported by the transport crate and main facade.

The main `furnace-rs` crate exposes an optional `socket` feature. Follow existing
Furnace feature boundaries for optional Passport/JWT, TLS, and Redis support;
their exact public feature spellings are proposed below for written review.
Socket applications use the existing neutral core and shared runtime contracts.
Using the socket transport must not implicitly install the Furnace HTTP
application routes or require the gRPC transport.

Add the transport identifier `socket` to explicit transport selection. Keep
existing default HTTP selection behavior. A socket listener requires an
explicit address. Validate selected registrations and configuration before
opening listeners. All enabled transports share application lifecycle ownership.

## Public registration and handler API

The agreed attribute structure is:

```rust,ignore
#[socket_gateway(path = "/chat", mode = "events")]
impl ChatGateway {
    // #[on_connect]
    // #[on_disconnect]
    // #[on_event("chat.send")]
    // #[on_join]
}

#[socket_gateway(path = "/raw", mode = "raw")]
impl RawGateway {
    // #[on_text]
    // #[on_binary]
}

// #[socket_endpoint(path = "/custom")] marks an owned-socket handler.
```

Gateways and owned-socket endpoints use DI and explicit cauldron membership,
following the existing transport-provider model. Only registrations reachable
from the selected application root are served or inspected. Reject duplicate
paths, duplicate event names, reserved event names, incompatible mode handlers,
and invalid handler signatures before serving.

`SocketContext` exposes connection identity, handshake authentication, sending,
room operations, and broadcasting to managed handlers. Ordinary services must
also be able to obtain a DI-managed sending interface for application-originated
room messages and broadcasts.

Gateway processing is selectable per gateway: sequential by default, or bounded
concurrent execution. Sequential dispatch preserves incoming application
message order per connection. Different connections run independently.
Concurrent handlers may complete in a different order. Transport control,
membership changes, and replay transitions must maintain their own consistency
regardless of application-handler concurrency.

Owned-socket endpoints receive the native Axum WebSocket plus handshake context.
Taking ownership transfers receive/send/close responsibility to the handler.

## Handshake authentication and origins

Passport/JWT protection is optional and runs before upgrade. There are no
per-event JWT guards in this version. Preserve the authenticated principal in
the connection context for handler and room-join decisions.

Support Bearer authorization headers, a configured authentication cookie, and
a configured query-token parameter. Reject multiple supplied credential sources,
even when their values match. Unrelated cookies do not count. Invalid credentials
fail immediately; never fall back to another source. Treat repeated or malformed
authentication fields as invalid rather than selecting one silently.

Validate every supplied Origin against an explicitly configured allowlist.
Clients without an Origin header are allowed. An empty allowlist allows no
supplied origins. Origin validation applies before upgrade, including when
cookie authentication is used. Credentials and token-bearing URLs must not
appear in client errors, diagnostic output, or transport logs.

Native TLS loads server identity before readiness. Proxy termination accepts
ordinary WebSocket traffic on the backend listener; deployment documentation
explains forwarding upgrades and keeping the backend appropriately scoped.
Forwarded headers do not by themselves establish a trusted client identity.

## Named-event wire protocol

Client application event:

```json
{"event":"chat.send","id":"request-1","data":{"text":"hello"}}
```

`event` selects the handler, `data` carries the JSON payload, and optional `id`
correlates replies. Reserve the `furnace.` event prefix for transport commands.

Built-in join and leave:

```json
{"event":"furnace.join","id":"join-1","data":{"room":"chat:42","after":"1720000000000-0"}}
{"event":"furnace.leave","id":"leave-1","data":{"room":"chat:42"}}
```

`after` is an optional resume cursor. Built-in commands return `furnace.ok` or
`furnace.error`, preserving the request ID. Errors expose stable codes. A
successful join acknowledgement follows replay completion and marks the
transition to live delivery for that room.

Replayable server room event:

```json
{"event":"chat.message","room":"chat:42","cursor":"1720000000001-0","data":{"text":"hello"}}
```

Clients save cursors after processing messages. Cursor values are interpreted
within the requested room and its current lifetime; a deleted/recreated room
reports expired history for cursors older than that lifetime. Live events do
not promise resumability.
Global broadcasts and direct connection sends are live; replay is room history.

Malformed JSON, unknown application events, and handler errors return
`furnace.error` while keeping the connection open. Internal failures expose a
generic message and log useful server-side details. WebSocket protocol
violations, oversized messages, heartbeat failure, and exhausted send queues
close the connection. Raw gateways do not receive JSON error envelopes.

## Dynamic rooms and authorization

Application code creates rooms with delivery settings. Each room is either
live or replayable; the mode is fixed for the room lifetime. Replayable rooms
must explicitly specify both maximum age and maximum message count. Remove
history as either bound is exceeded. Room names and broadcasts are scoped to
the gateway and configured application namespace.

Redis stores room definitions for distributed consistency. Creating a room
with identical settings is idempotent; conflicting settings return an error.
Settings change only by deleting and recreating the room. In-memory rooms use
the same public creation and conflict semantics within their process.

Clients have built-in join and leave commands. An application `on_join` hook
must approve each join. Without that hook, deny the join. Reconnects and replay
requests are authorized again. Once approved, a client may access any retained
history for that room, including history predating its first membership.
Applications needing a narrower policy must enforce it in the hook.

Room creation is an application operation, not an implicit consequence of an
arbitrary client-supplied room name. Leave removes the requesting connection
from the room. Disconnect removes all memberships for that connection.

Deletion is permitted only when a room is empty across all active instances.
Use Redis-backed instance membership leases refreshed every 5 seconds and
expired after 15 seconds, both configurable. Stale leases can briefly delay
deletion after a crash. An instance that loses its lease stops room delivery
until membership is registered again. Deletion, room creation, and membership
changes must coordinate atomically so a concurrent join cannot bypass the
empty-room rule. Deleting a room also removes its retained history.

## Delivery and Redis behavior

Live rooms use Redis Pub/Sub for cross-instance fan-out. Messages missed during
a disconnect are not retained or replayed. In-memory live rooms require no Redis.

Replayable rooms retain messages in Redis Streams and expose their IDs as
resume cursors. Readers independently consume room history; a shared worker
consumer group must not distribute a room message to only one instance.
Preserve room stream order through catch-up and subsequent live delivery,
without a history-to-live gap. No ordering guarantee spans different rooms.

A replay request returns retained entries strictly after the supplied cursor.
An expired cursor returns an explicit history-expired result, without silently
skipping missing history or joining the client as if recovery had succeeded.
Malformed cursors return an invalid-cursor result. Redis stream IDs alone do not
encode room identity; clients must keep their cursors associated with the room.

Furnace does not store per-client acknowledgements or durable client sessions.
Clients may receive duplicates across reconnects and must handle them. Replay
depends on retention and the Redis deployment's persistence; it is not an
exactly-once delivery guarantee. Concurrent publishers must not append entries
in one order and deliver them in a different order. Redis failure after an
uncertain publish must not trigger automatic message retries.

With Redis configured, startup connection and subscription preparation must
succeed before the socket listener becomes ready. At runtime, keep connections
and local membership through outages, but reject room sends and broadcasts.
Direct sends to local connections can continue. Reconnect automatically and
resume room operations only after subscriptions and membership are restored.
Replayable room consumers recover retained history from their stream cursor;
live rooms cannot recover missed messages. Do not silently substitute local-only
delivery for a distributed operation. A successful publish does not acknowledge
receipt or processing by every client.

## Limits, heartbeat, and shutdown

All listed limits are configurable; reject invalid values during preflight.

| Setting | Default |
| --- | --- |
| Maximum incoming message | 1 MiB |
| Outgoing queue | 256 messages and 4 MiB total, whichever fills first |
| Concurrent handler limit when enabled | 16 per connection |
| Heartbeat ping interval | 30 seconds |
| Pong timeout after ping | 10 seconds |
| Graceful shutdown timeout | 10 seconds |
| Membership lease refresh | 5 seconds |
| Membership lease expiry | 15 seconds |

Disconnect slow clients rather than silently dropping outbound messages. Clients
in replayable rooms can reconnect using their last processed cursor, subject to
retention. Bound replay fetching and concurrent handler admission; do not build
unbounded side queues outside the configured send queue.

Managed connections send heartbeat pings and close when the matching pong does
not arrive within the timeout. Owned-socket handlers manage their own heartbeat
and outbound buffering.

On shutdown, stop accepting upgrades, initiate managed connection close, stop
new handler admission, drain supervised tasks within the timeout, then abort
and join remaining tasks before provider shutdown. Supervise owned-socket tasks,
Redis tasks, heartbeat tasks, and replay workers through the shared runtime.

## Inspection, documentation, and example

Extend neutral inspection metadata and the CLI to report selected socket paths,
modes, providers, handshake policies, and event handlers. Static inspection must
not bind listeners, resolve providers, or connect to Redis. It cannot list
dynamic runtime room memberships as static declarations.

Provide a transport guide and a runnable example demonstrating event and raw
endpoints, owned sockets, DI, handshake credential sources, join authorization,
in-memory rooms, two instances with Redis, live/replayable delivery, reconnect
cursors, TLS, and proxy deployment. Clearly distinguish live loss, replay
retention, client processing, and successful publish.

## Concrete spellings proposed for written review

These complete the public naming details that were not individually selected
in conversation; reviewing this document includes reviewing these proposals:

- Transport/facade feature: `socket`; companion facade features `socket-tls`
  and `socket-redis`. Existing facade `jwt` forwards optional socket admission.
- Standalone optional features: `jwt`, `tls`, and `redis`; no default features.
- CLI command: `furnace socket`, alongside existing route/gRPC inspection.
- Configuration namespace: `socket`, with `socket.address`, `socket.origins`,
  `socket.namespace`, and nested `socket.tls`, `socket.redis`, and limit settings.
- Credential defaults: query parameter `access_token`, cookie `access_token`;
  both names configurable on protected endpoints.
- Built-in error payload: `data.code` and `data.message`, including
  `history_expired`, `invalid_cursor`, `join_denied`, `room_not_found`,
  `backend_unavailable`, `invalid_message`, `unknown_event`, and `internal_error`.
- Concurrent handlers use gateway settings `processing = "concurrent"` and
  `max_concurrency`; omit them for sequential processing.

## Verification requirements

Use protocol-level integration tests with actual WebSocket clients, meaningful
macro compile-pass/fail tests, and isolated Redis tests against an actual Redis
server. Cover:

- Raw, JSON, owned-socket, text, binary, close, ping/pong, and message limits.
- Sequential order and bounded concurrency; no hidden unbounded queues.
- Standalone, renamed dependency, facade, and optional feature combinations.
- Selected cauldron scope, DI, duplicate routes/events, and preflight failures.
- All credential sources, conflicts, invalid fields, origin policy, and redaction.
- Native TLS and ordinary backend connections used behind a terminating proxy.
- In-memory rooms and distributed sends without duplicate local fan-out.
- Join denial, replay authorization, retention expiry, count/age limits,
  cross-instance reconnect, and replay/live ordering during concurrent publishes.
- Redis startup failure, outage, resubscription, leases, crash cleanup,
  room-definition conflicts, and atomic empty-room deletion.
- Shared lifecycle, readiness failure rollback, bounded shutdown, and task cleanup.
- CLI output and side-effect-free inspection, guide examples, and consumer builds.

Run formatting, relevant package tests, feature-matrix checks, and workspace
regression checks appropriate to the final changes. Report exact evidence and
any unavailable infrastructure before claiming implementation complete.
