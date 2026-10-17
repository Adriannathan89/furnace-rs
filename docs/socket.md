# WebSocket server transport

`furnace-rs-socket` serves standard WebSocket connections through Axum on a
separate listener. It integrates with Furnace DI, cauldrons, Passport, startup,
and shutdown. Browser `WebSocket` clients work directly; this is not Socket.IO.

## Dependencies and startup

Facade applications enable `socket` and `runtime-tokio`. Optional facade features
are `socket-tls`, `socket-redis`, and `jwt`. The standalone transport has no default
features; its optional features are `tls`, `redis`, and `jwt`.

```toml
[dependencies]
furnace = { package = "furnace-rs", version = "=2.0.0", default-features = false, features = ["socket", "runtime-tokio"] }
serde_json = "1"
```

Select the transport and explicitly configure its address in `furnace.toml`:

```toml
[furnace]
transports = ["socket"]
[socket]
address = "127.0.0.1:3001"
namespace = "my-application"
origins = ["https://app.example.com"]
```

Select `http`, `grpc`, and `socket` together to launch their independent listeners
under one application lifecycle. The usual HTTP default remains unchanged when
`furnace.transports` is omitted. Socket-only applications must select `socket`.

For native composition use `build_socket_routes`, `RuntimeRoutes::with_socket`,
`RuntimeBindings::with_socket`, and `serve_transports`. Import `RuntimeSocketExt`
to enable these builder methods. Provider construction precedes runtime startup;
listener admission follows successful application readiness. Shutdown stops
upgrades, closes managed connections, drains tasks, then stops providers.

## Managed gateways

```rust,ignore
use furnace::prelude::*;
use serde_json::Value;

struct Chat;
impl Injector for Chat {
    type Dependencies = ();
    async fn inject(_: ()) -> furnace::core::Result<Self> { Ok(Self) }
}

#[socket_gateway(path = "/chat", mode = "events")]
impl Chat {
    #[on_connect]
    async fn connected(&self, c: SocketContext) -> SocketResult<()> {
        c.hub().create_room("lobby", RoomMode::Live).await
    }
    #[on_join]
    async fn authorize_join(&self, _: SocketContext, room: String) -> SocketResult<()> {
        if room == "lobby" { Ok(()) } else { Err(SocketError::new("join_denied")) }
    }
    #[on_event("chat.send")]
    async fn send(&self, c: SocketContext, data: Value) -> SocketResult<()> {
        c.hub().emit_room("lobby", "chat.message", data).await.map(|_| ())
    }
}

#[cauldron(root, socket_gateways = [Chat])]
struct App;

#[furnace::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Furnace::burn::<App>().await?;
    Ok(())
}
```

Only gateways/endpoints selected by the root cauldron are served. Existing
imports, exports, DI dependencies, and root validation apply. Gateway structs
must implement `Injector`, either directly or through existing Furnace provider
attributes. Register owned endpoints in `socket_endpoints = [...]`.

Supported methods are async shared `&self` methods returning `SocketResult<()>`:

| Attribute | Method arguments after `&self` |
| --- | --- |
| `on_connect`, `on_disconnect` | `SocketContext` |
| `on_event("name")` | `SocketContext`, `serde_json::Value` |
| `on_text` | `SocketContext`, `String` |
| `on_binary` | `SocketContext`, `axum::body::Bytes` |
| `on_join` | `SocketContext`, `String` room name |

Each endpoint chooses `mode = "events"` or `mode = "raw"`. Raw gateways interpret
text/binary themselves; Furnace does not detect JSON events inside raw text.
Transport-generated errors close raw connections without inserting JSON frames.
Lifecycle hooks work in either mode. Duplicate paths, event names, handlers,
unsupported signatures, and reserved application event names are rejected.

Processing is sequential per connection by default. Enable bounded concurrency
with `processing = "concurrent", max_concurrency = 16` on the gateway attribute.
Concurrent handlers may complete out of order. Other connections always run
independently. Transport control frames remain responsive during handler work.
Inbound work waiting for a busy handler is also bounded by the configured queue
count/byte limits; exhausting that admission capacity closes the connection.

`SocketContext` exposes `id()`, `handshake()`, `hub()`, `send(Message)`,
`emit(event, data)`, `join(room, after)`, and `leave(room)`. `emit` preserves the
current request correlation ID. To send from an ordinary service, register
`SocketHub` as an explicit DI provider and inject its clone, then call
`hub.gateway("/chat")`. The runtime uses that same application-owned instance.
Without an explicit hub provider the runtime creates its own internal hub.

## Owned socket endpoints

```rust,ignore
#[socket_endpoint(path = "/custom")]
impl CustomEndpoint {
    async fn serve(&self, handshake: SocketHandshake, mut socket: furnace::socket::WebSocket)
        -> SocketResult<()>
    {
        while let Some(Ok(message)) = socket.recv().await {
            // Application-defined protocol and socket lifecycle.
        }
        Ok(())
    }
}
```

The implementation must have exactly one async handler method. Furnace enforces
handshake admission and incoming message size, then transfers socket ownership.
The application owns reads, writes, heartbeat, and buffering. Managed event
dispatch, room commands, replay, and managed heartbeat do not run for this socket.
The handler task remains supervised and can be cancelled at shutdown timeout.

## Handshake authentication and browser origins

With `jwt`, add `guard = Protection` to either endpoint attribute, using an
existing `#[guard]` policy and Passport strategy. Admission runs before upgrade;
there are no per-event JWT checks. Use
`context.handshake().authenticated::<Principal>()` to extract the principal.
Token expiry is checked at admission; room access is decided by the join hook.

Protected endpoints accept exactly one credential from:

- `Authorization: Bearer <token>`.
- Cookie `access_token` (override with `cookie = "cookie_name"`).
- Query parameter `access_token` (override with `query = "parameter_name"`).

Multiple sources or repeated authentication values fail even when identical.
Invalid credentials never fall back to another source. Other cookies are fine.
Browsers cannot set the Authorization header with their native WebSocket
constructor; cookies and query tokens cover that case. Treat query tokens as
secrets in reverse-proxy access logs as well as application logs. Library errors
and inspection output omit credentials; strategy metadata receives no auth
headers or query string. Applications retain access to original handshake
metadata and must avoid logging it indiscriminately.

A supplied Origin must exactly match `socket.origins`; an empty list accepts no
supplied origins. Origin-less clients are permitted. Matching includes scheme,
host, and port. Configure browser origins as `https://host[:port]`, without a
trailing slash. Cookie authentication does not bypass this policy.

## Event wire format

Application request:

```json
{"event":"chat.send","id":"request-1","data":{"text":"hello"}}
```

`id` is optional. Raw endpoints have no envelope requirement. Event-mode client
messages cannot supply server-owned `room` or `cursor` metadata.

Client room commands use the reserved `furnace.` prefix:

```json
{"event":"furnace.join","id":"join-1","data":{"room":"lobby"}}
{"event":"furnace.leave","id":"leave-1","data":{"room":"lobby"}}
```

Commands receive `furnace.ok` or `furnace.error`, preserving `id`. Error payloads
contain `code` and `message`. Codes include `invalid_message`, `unknown_event`,
`join_denied`, `room_not_found`, `room_not_empty`, `room_conflict`,
`backend_unavailable`, `invalid_cursor`, `history_expired`, and `internal_error`.
Malformed JSON, unknown events, and recoverable handler failures keep the socket
open. Internal errors expose a generic message. Protocol/size violations,
heartbeat timeout, and full send queues close the connection. Raw endpoints do
not receive Furnace JSON error envelopes.

## Dynamic rooms and broadcasts

Rooms are created by application code; client join requests do not create them.
An `on_join` hook must approve every join; absence of a hook denies access.
Clients leaving or disconnecting lose membership. Room names and broadcasts are
scoped to the gateway path and application namespace.

`SocketGatewayHub` provides:

- `create_room(name, RoomMode)` and `delete_room(name)`.
- `send(connection_id, Message)` for a local connection.
- `send_room(name, Message)` for live text/binary room delivery.
- `emit_room(name, event, data)` for JSON room delivery, returning an optional cursor.
- `broadcast(Message)` for live delivery to every connection on that gateway.

Broadcast includes the sender when it is connected to that gateway. Identical
room definitions are idempotent; conflicting settings fail. A room can be deleted
only when empty across all active instances. Deletion removes history. Changing
mode/retention requires deleting and recreating the empty room.

## Redis and replay

In-memory live rooms work without Redis. Cross-instance delivery and replay
require `redis`/`socket-redis` and an explicit shared namespace:

```toml
[socket.redis]
url = "redis://user:password@127.0.0.1:6379"
```

Redis 6.2 or later is required for exact stream-age trimming.
Use `rediss://` for verified Redis TLS. Redis TLS uses the existing Rustls crypto
provider, installing ring only when no provider has been configured. One endpoint is supported; Sentinel
and cluster-aware discovery/routing are outside this version.

Live rooms use Pub/Sub. Disconnected clients and instances miss live messages.
Replayable rooms use Redis Streams:

```rust,ignore
hub.create_room("orders", RoomMode::Replayable {
    max_age: std::time::Duration::from_secs(3600),
    max_messages: 10_000,
}).await?;
```

Both retention limits are explicit and positive. Events expire as either limit
is exceeded. Replay applies to named JSON room events, not raw frames, global
broadcasts, or direct sends. A replayable server event includes:

```json
{"event":"order.updated","room":"orders","cursor":"1720000000001-0","data":{"status":"ready"}}
```

Clients save each room's cursor after processing. Resume with:

```json
{"event":"furnace.join","id":"resume-1","data":{"room":"orders","after":"1720000000001-0"}}
```

Furnace authorizes the join again, replays retained entries after the cursor,
acknowledges catch-up with `furnace.ok`, then continues in room order.
Room transitions serialize per connection, including under concurrent processing.
Rejoining replaces the previous reader before catch-up; leaving waits for an
in-flight join and stops its reader before completing. A join
without `after` receives future messages. Approved clients may request any
retained history, including messages predating their first membership.
Applications needing narrower access must enforce it in their join hook.

Expired history returns `history_expired`; the application chooses how to
resynchronize. There is no silent jump over missing messages. Malformed/future
cursors fail. Keep cursors associated with their rooms: Redis IDs do not encode
room identity. Deleted/recreated rooms reject cursors predating their new lifetime.
Order is per room; there is no cross-room ordering. Clients should handle duplicate
processing around reconnects. Furnace does not retain client acknowledgements.
Redis persistence/replication settings and room retention bound recovery; a
successful publish does not acknowledge processing by every client.

Redis must be reachable and subscribed before readiness when configured. During
outages connections remain open, direct local sends work, and room/broadcast
sends return errors. Furnace reconnects and restores membership before resuming.
Uncertain publishes/appends are not automatically retried. Instance leases expire
stale memberships after a crash, so deleting an apparently empty room can wait
until the old lease expires. Old room generations cannot receive new-generation
traffic without joining again.

## Limits and TLS

| Setting under `socket` | Default |
| --- | --- |
| `max_message_bytes` | 1048576 |
| `queue_messages` | 256 |
| `queue_bytes` | 4194304 |
| `heartbeat_interval_ms` | 30000 |
| `pong_timeout_ms` | 10000 |
| `shutdown_timeout_ms` | 10000 |
| `lease_refresh_ms` | 5000 |
| `lease_expiry_ms` | 15000 |

Queue limits apply per managed connection; exceeding either disconnects the slow
client. Replayable clients can reconnect using their saved cursor within retention.
Lease refresh must be shorter than expiry. Concurrent handler limits are gateway
attribute settings, not listener-wide settings.

For native TLS enable `tls`/`socket-tls`:

```toml
[socket.tls]
certificate = "server-chain.pem"
private_key = "server-key.pem"
handshake_timeout_ms = 10000
```

Paths resolve relative to their winning configuration source. Identity is loaded
and validated before serving. For proxy TLS termination omit native TLS and
forward HTTP/1.1 WebSocket Upgrade/Connection headers to the separate backend
listener. Protect backend reachability and configure proxy timeouts to accommodate
heartbeats. Forwarded headers do not establish a trusted peer identity.

## Inspection and verification

`furnace socket` and `furnace socket --format json` list selected paths, modes,
providers, handlers, source locations, and handshake policies. Graph/doctor
inspection includes socket metadata without constructing providers, opening
listeners, loading TLS identities, or contacting Redis. Dynamic room membership
is runtime state, not static inspection output.

See [the runnable example](../example/socket-transport/README.md).
Run `cargo test -p furnace-rs-socket --all-features` for protocol, Passport, TLS,
rooms, and Redis tests. Redis tests start isolated `redis-server` processes and
require that executable; they do not silently skip unavailable infrastructure.
