# furnace-rs-socket-macros

Registration attributes re-exported by `furnace-rs-socket` and the Furnace facade.
`socket_gateway` generates typed DI adapters and static metadata for managed
connection/event methods. `socket_endpoint` registers one owned-socket handler.
Applications normally depend on the transport or facade, not this crate directly.

See the [WebSocket guide](../../docs/socket.md).
