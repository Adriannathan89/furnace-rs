# Protobuf transport example

This independent Rust 1.94 project serves and consumes unary, server streaming,
client streaming, and bidirectional RPCs. It generates its own protobuf code and
file descriptors with a vendored build-time compiler; system `protoc` is unnecessary.

From this directory, create a local signing secret and start the server:

```sh
umask 077
printf 'DEMO_JWT_SECRET=%s\n' "$(openssl rand -hex 32)" > .env
cargo run
```

In another terminal, from the same directory:

```sh
cargo run -- client
```

The client resolves `Remote` through DI, calls its native generated `client()` API,
and obtains a fresh Bearer credential from `Tokens` for each call. This example
mints local demo credentials with the same application key. Production token
providers can retrieve credentials from their issuer using the same async trait.
Credentials are never printed or committed.

`Unary` is explicitly public. The other three methods require the reader role,
`demo:use` permission, and an authored predicate, then receive the typed principal.
Health Check/Watch is public. Reflection is explicitly enabled and uses the same
root guard; reflection authorization does not authorize invoking business methods.

```sh
cargo test
cargo +1.94.0 check --locked
```

The integration test proves all four shapes, rejected missing credentials,
explicit Bearer precedence over the provider, public health, and guarded reflection.
Only synthetic test signing material appears in the test source.

## Mixed HTTP/gRPC mode

Change `[furnace]` to `transports = ["http", "grpc"]`, then run:

```sh
cargo run --features http
```

The same application serves HTTP `/health` on port 3000 and gRPC on port 50051.
Both listeners share one DI graph and lifecycle. Missing transport selection in
an HTTP build preserves the HTTP-only default.

## Local verified TLS

Create a local CA and a certificate for `localhost`:

```sh
mkdir -p certs
umask 077
openssl req -x509 -newkey rsa:2048 -nodes -keyout certs/ca.key -out certs/ca.pem -days 1 -subj '/CN=Furnace demo CA'
openssl req -newkey rsa:2048 -nodes -keyout certs/server.key -out certs/server.csr -subj '/CN=localhost'
printf 'subjectAltName=DNS:localhost\nbasicConstraints=CA:FALSE\nextendedKeyUsage=serverAuth\n' > certs/server.ext
openssl x509 -req -in certs/server.csr -CA certs/ca.pem -CAkey certs/ca.key -CAcreateserial -out certs/server.pem -days 1 -extfile certs/server.ext
```

Add these tables to `furnace.toml` and change the remote endpoint to
`https://127.0.0.1:50051`:

```toml
[grpc.server.tls]
certificate = "certs/server.pem"
private_key = "certs/server.key"

[grpc.clients.remote.tls]
custom_ca_files = ["certs/ca.pem"]
custom_ca_only = true
domain_name = "localhost"
```

Paths resolve relative to their configuration source. Custom-only trust isolates
the demo CA; omitting that setting adds configured CAs to the default WebPKI roots.
Hostname and chain verification remain enabled. Generated certificate material
is ignored by Git.

See [the transport guide](../../docs/grpc.md) for configuration, manual composition,
client deadlines, native cancellation, authenticated stream lifetime, and CLI inspection.
