# furnace-rs-socket

Standard WebSocket server transport for Furnace, backed by Axum. Provides
DI-managed raw and JSON-event gateways, owned-socket endpoints, handshake
Passport/JWT, dynamic rooms, broadcasting, and optional Redis live/replay delivery.
The separate listener joins Furnace's shared startup and shutdown lifecycle.

No default features. Optional features: `jwt`, `tls`, and `redis`. The main
`furnace-rs` facade exposes `socket`, `socket-tls`, and `socket-redis`.

Read the [transport guide](../../docs/socket.md) and
[runnable example](../../example/socket-transport/README.md).

Run `cargo test -p furnace-rs-socket --all-features`. Redis integration tests
require `redis-server` and create isolated temporary instances.
