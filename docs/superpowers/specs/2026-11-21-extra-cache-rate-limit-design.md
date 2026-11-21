# furnace-rs-extra: endpoint caching and rate limiting

Date: 2026-11-21. Status: awaiting approval.

## Intent

Let an application protect and speed up individual HTTP endpoints with two
attributes, backed by Redis so limits and cache entries are shared across
instances:

```rust
#[get("/posts")]
#[rate_limit(limit = 30, per = 60)]   // 30 requests per 60 s
#[cache(ttl = 120)]                    // seconds
async fn list(&self) -> HttpResult<Json<Vec<Post>>> { ... }
```

The application registers the service by giving a Redis URL to the builder.

## Goals

- `#[cache(ttl = N)]` and `#[rate_limit(limit = N, per = S)]` on endpoints inside a `#[controller]` impl.
- Registration: `Furnace::builder().provide(RedisExtra::new("redis://..."))`.
- Unit-testable without Redis through a small backend trait with an in-memory implementation.

## Non-goals (v1)

- Cache invalidation API, stale-while-revalidate, `Cache-Control` negotiation. Expiry is TTL only.
- Trusting `X-Forwarded-For`. Behind a proxy all clients share the proxy IP.
- Redis Sentinel/cluster, per-attribute failure policy, sliding-window or token-bucket algorithms.
- Caching non-GET methods or non-2xx responses.
- `furnace.toml` registration (the user chose builder-only).

## Anti-goals

- One user must never be served another user's cached response.
- A Redis outage must never take the endpoint down.

## Approach (chosen)

Per-endpoint route layers emitted by the controller macro. `RouteGuard::protect()`
in `furnace-rs-common/src/route.rs` already adds the Passport guard to each
endpoint as a `route_layer`; the generated registrar adds the cache and
rate-limit layers the same way. Layer logic lives in `furnace-rs-extra`; the
macro only parses attributes and emits the wiring.

Rejected: macros that rewrite the handler body (cannot see the return type or
the request); path rules in `furnace.toml` (user asked for macros).

## Design

### Crates

- `furnace-rs-extra` gains features `cache` and `rate-limit` and now depends on
  `furnace-rs-common` (feature `http`), `redis` 0.32, `sha2`, `tokio`. This
  **breaks the README's current "depends only on core" boundary**; update the
  README. The facade `extra` feature enables `http`.
- No new macro crate: `cache` and `rate_limit` are exported from
  `furnace-rs-common-macros` behind a new `extra` feature, recognised by the
  controller macro the way `seal` is (a stray use outside a controller is a
  compile error). Generated code refers to `furnace_rs_extra` through
  `proc-macro-crate`, so `common` does not depend on `extra` (no cycle). Using the
  attributes without `extra` enabled gives a clear compile error.
  *(Changed from my earlier "new macros crate" sketch: one less crate.)*

### Service and backend trait

```rust
trait Backend: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set_ex(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()>;
    /// Atomically increments `key`, setting its expiry on first hit.
    /// Returns (count, seconds until the window resets).
    async fn incr_window(&self, key: &str, window: Duration) -> Result<(u64, u64)>;
}
```

`RedisBackend` (Lua `INCR` + `EXPIRE` for atomicity) and `MemoryBackend` (used
by tests). `ExtraCauldron` registers `RedisExtra` as a provider (a rooted builder rejects
`provide` for unregistered types). Its injector returns a placeholder whose
every call errors with a "was not provided" message, so apps that merely link
the crate (including complete-catalog builds) still start, and requests fail
open with a logged fix. Each backend call is bounded to 250 ms and a
timeout fails open like any other backend error. `RedisExtra::new(url)` validates the scheme (`redis://` or `rediss://`,
no `#`, as in `furnace-rs-socket`), holds a shared `Arc<dyn Backend>`, and is
supplied through `FurnaceBuilder::provide`. Router build resolves it from the
`ApplicationContext`; if an endpoint uses either attribute and the service is
missing, the build fails with a diagnostic (not a request-time error).
Connection is lazy so a down Redis does not block startup.

### Cache layer

- GET only; caches status, `Content-Type`, and body of 2xx responses.
- Not cached: responses with `Set-Cookie`, or a body over 1 MiB.
- Key: `furnace:cache:{method}:{path}?{query sorted}:{credential}` where
  `credential` is `anon` when the request has no `Authorization`/`Cookie`
  header, else the hex SHA-256 of those header values.
  - **Deviation from "principal":** `PassportPrincipal` exposes no identity and
    its type is application-defined, so a generic layer cannot read a user id.
    A credential fingerprint is safe (never shares across users) but is
    per-token, so one user with two tokens gets two entries. A future
    `PassportPrincipal::id()` default method could tighten this.
  - Raw credentials are never stored or logged.
- Backend error: log and call the handler (fail open).

### Rate-limit layer

- Fixed window per `(route, identity)`, via `Backend::incr_window`.
- Identity: on a *sealed* (guarded) endpoint, the credential the guard validated,
  hashed with SHA-256 (the guard installs a `ValidatedCredential` request
  extension after authentication; header fingerprints are NOT used because the
  guard ignores e.g. `Cookie` on Bearer routes, so they would mint buckets);
  otherwise the peer IP from `ConnectInfo<SocketAddr>` (already inserted by the
  server). An attacker cannot mint buckets with garbage headers on either kind
  of endpoint. If no peer address exists (e.g. in-process tests), use a
  single `unknown` bucket.
- Over limit: `429` with JSON error body, `Retry-After`, and
  `X-RateLimit-Limit` / `X-RateLimit-Remaining` on allowed responses.
- Backend error: log and allow (fail open).

### Ordering

guard → rate limit → cache → handler. Cache hits still count against the limit.

### Macro validation (compile time)

`limit`, `per`, `ttl` must be positive integer literals; `cache` is allowed only
on `#[get]`; each attribute at most once per endpoint; unknown keys are errors.

## Tests

- Unit: `MemoryBackend` window and TTL behaviour (with `tokio::time` pause),
  key construction (query sorting, anon vs credential), URL validation.
- Router-level via `furnace-rs-testing`/`tower::ServiceExt`: second cached GET
  doesn't invoke the handler; different credentials don't share entries;
  `Set-Cookie` and oversize bodies aren't cached; N+1th request gets 429 with
  `Retry-After`; failing backend still returns the handler's response.
- trybuild: compile-fail snapshots for bad attribute arguments.
- `RedisBackend` is **not** exercised against a real Redis (no Redis in tests,
  per decision); its Lua script is reviewed by hand only. Called out as risk.

## Left to the builder

Exact Lua script, redis client pooling choice (`redis` multiplexed connection
manager assumed), error/diagnostic code numbers, README and guide updates
(`docs/guides/extra.md`, facade README feature table, CHANGELOG).
