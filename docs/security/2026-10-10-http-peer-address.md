# HTTP socket peer propagation

Baseline: `bc16a4fc20a7a03ff13cda98de93121fbd2a8541` (1.0.2 source).
Affected implementation: `furnace-rs-common/src/server.rs::serve_http`.

## Reproduction and impact

The runtime discarded the address returned by `TcpListener::accept` and
dispatched requests without `ConnectInfo<SocketAddr>`. A native Axum handler
requiring that extractor returned HTTP 500 on both HTTP/1 and HTTP/2. Passport's
`request_metadata` also reads this extension, so its strategy context could not
obtain the transport peer through this server path.

The two loopback regression tests record the client's actual local socket
address, send deliberately different `Forwarded` and `X-Forwarded-For` values,
and require the handler to return the socket address. Before the patch, both
tests failed with HTTP 500 and the missing-extension rejection. This proves
missing metadata and native extractor failure; it does not establish an
authorization bypass in an application-defined IP policy.

```sh
cargo test --locked -p furnace-rs-common --lib socket_peer_is_available -- --nocapture
```

## Patch method and compatibility

Retain the accepted peer address in each connection's service closure and insert
`axum::extract::ConnectInfo(peer_addr)` before dispatch. The source is the socket,
not request headers. Existing timeout and shutdown machinery stays on the same
path. The peer is the immediate transport peer; when deployed behind a proxy,
this is the proxy's address. No trusted-forwarding policy is introduced.

After patching, both regression tests pass with HTTP 200 and the exact recorded
socket address. Native extractors and Passport's existing metadata reader now
receive the same trusted transport extension.

Evidence: [baseline](evidence/2026-10-10-peer-red.txt),
[patched](evidence/2026-10-10-peer-green.txt).
