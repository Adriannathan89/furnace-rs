# Socket transport example

Run from this directory:

```sh
cargo run
```

The separate listener serves event gateway `/chat`, raw text `/raw`, and owned
text/binary echo `/owned` on port 3001. From a frontend with an allowed origin:

```js
const socket = new WebSocket("ws://127.0.0.1:3001/chat");
socket.onmessage = ({ data }) => console.log(JSON.parse(data));
socket.onopen = () => socket.send(JSON.stringify({
  event: "furnace.join", id: "join", data: { room: "lobby" }
}));
// After furnace.ok:
// socket.send(JSON.stringify({event:"chat.send",data:{text:"hello"}}));
```

For two instances, start Redis, uncomment `socket.redis` in `furnace.toml`, run
`cargo run --features redis`, then start another instance with a different
`socket.address` in a second configuration directory. Both use namespace
`socket-example`. Messages to `lobby` cross instances. To retain history, also
uncomment `demo.replay = true`; the example retains 1000 events for up to one hour.
Save each `chat.message` cursor after processing and reconnect with
`data: {room:"lobby",after:savedCursor}`. Replay precedes the join acknowledgement.

For protected `/chat`, enable `jwt`, add a strong `passport.secret` to configuration,
and run `cargo run --features jwt -- token` to issue a five-minute demo token.
Connect using `?access_token=...`, cookie `access_token`, or a Bearer header from a
client that supports custom headers. Supply only one credential source. The lobby
join hook only permits `lobby`. `/raw` and `/owned` remain public in this example.

For WSS, enable `tls` and configure the certificate/private-key paths shown in
`furnace.toml`. A terminating proxy can instead forward upgrades to the ordinary
backend listener. See the [full transport guide](../../docs/socket.md) for Origin
policy, message envelopes, delivery limits, outage behavior, and DI sending APIs.
