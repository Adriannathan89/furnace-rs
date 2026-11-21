# furnace-rs-extra Cache and Rate Limiting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `#[cache(ttl = N)]` and `#[rate_limit(limit = N, per = S)]` endpoint attributes backed by a Redis service registered with `builder.provide(RedisExtra::new(url)?)`.

**Architecture:** `furnace-rs-extra` holds a `Backend` trait (Redis + in-memory impls), the `RedisExtra` service, and two Axum `from_fn_with_state` middlewares. The controller macro (in `furnace-rs-common-macros`, behind a new `extra` feature) parses the attributes and appends `.route_layer(...)` calls in the existing `HandlerAdapter.guard_layer` slot, so the Passport guard added later by `protect()` stays outermost: guard → rate limit → cache → handler.

**Tech Stack:** Rust 2024, Axum 0.8 (re-exported by `furnace-rs-common`), `redis` 0.32 (same feature set as `furnace-rs-socket`), `sha2`, `tokio`, `tracing`.

**Spec:** `docs/superpowers/specs/2026-11-21-extra-cache-rate-limit-design.md`

## Global Constraints

- Work on branch `feat/extra-cache-rate-limit`. Commit messages are plain: **no `Co-Authored-By` trailer, no attribution lines**.
- Workspace lints apply: `missing_docs = "deny"`, `unsafe_code = "forbid"`, `clippy::all = "deny"`. Every `pub` item needs a doc comment.
- `rust-version = "1.94"`, edition 2024. Reuse workspace dependency versions where one exists (`tokio`, `tracing`, `axum`, `tower`).
- Cache: GET only; stores status, `Content-Type`, body of 2xx responses; skips responses with `Set-Cookie`; skips bodies over 1 MiB (1_048_576 bytes) or of unknown upper size hint; TTL-only expiry.
- Cache key: `furnace:cache:{method}:{path}?{sorted query}:{credential}`; `credential` is `anon` or the hex SHA-256 of the `Authorization` and `Cookie` header values. Raw credentials are never stored or logged.
- Rate limit: fixed window per `(route, identity)`; identity = credential fingerprint on a guarded endpoint, else peer IP from `ConnectInfo<SocketAddr>`, else the literal `unknown`. Over limit → `429`, `Retry-After`, `X-RateLimit-Limit`, `X-RateLimit-Remaining`. Key: `furnace:rl:{route}:{identity}`.
- Both features fail open on any backend error (log with `tracing::warn!`, continue).
- Redis URL: only `redis://` or `rediss://`, no `#` (same rule as `furnace-rs-socket/src/config.rs`).
- Attribute arguments (`ttl`, `limit`, `per`) must be positive integer literals; `cache` only on `#[get]`; each attribute at most once per endpoint; unknown keys are errors.
- Out of scope: invalidation API, `X-Forwarded-For`, Sentinel/cluster, `furnace.toml` registration, real-Redis tests.

## Review Focus

Each line is pinned by a test in the task named in brackets.

1. `?a=1&b=2` and `?b=2&a=1` must hit the same cache entry [Task 3].
2. A cached `Authorization: Bearer A` response must never be served to `Bearer B` or to no header [Task 3].
3. A backend that errors on every call must still return the handler's normal response, for both features [Tasks 3, 4].
4. Corrupt bytes stored under a cache key must be treated as a miss, not a panic or a 500 [Task 3].
5. A public (unguarded) endpoint must ignore `Authorization` for rate-limit identity, so rotating garbage tokens does not mint new buckets [Task 4].

---

### Task 1: Extra crate skeleton, `Backend` trait, `MemoryBackend`, `RedisExtra`

**Files:**
- Modify: `crates/furnace-rs-extra/Cargo.toml`
- Create: `crates/furnace-rs-extra/src/backend.rs`, `src/service.rs`, `src/identity.rs`
- Modify: `crates/furnace-rs-extra/src/lib.rs`

**Interfaces:**
- Produces (all `pub`, re-exported from the crate root):
  - `type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'a>>`
  - `struct BackendError(String)` implementing `Display` + `std::error::Error`, with `BackendError::new(impl Into<String>)`
  - `struct WindowHit { pub count: u64, pub reset_secs: u64 }`
  - `trait Backend: Send + Sync + 'static { fn get<'a>(&'a self, key: &'a str) -> BackendFuture<'a, Option<Vec<u8>>>; fn set_ex<'a>(&'a self, key: &'a str, value: Vec<u8>, ttl: Duration) -> BackendFuture<'a, ()>; fn incr_window<'a>(&'a self, key: &'a str, window: Duration) -> BackendFuture<'a, WindowHit>; }`
  - `struct MemoryBackend` (`Default`, `Clone`), uses `tokio::time::Instant` so `tokio::time::pause()` controls expiry
  - `struct RedisExtra` (`Clone`): `fn with_backend(backend: impl Backend) -> Self`, `pub(crate) fn backend(&self) -> &Arc<dyn Backend>`
  - `pub(crate) fn fingerprint(headers: &http::HeaderMap) -> Option<String>` and `pub(crate) fn canonical_query(query: Option<&str>) -> String` in `identity.rs`
- Cargo features: `cache`, `rate-limit`, `default = ["cache", "rate-limit"]`.

- [ ] **Step 1: Cargo.toml.** Add dependencies `furnace-rs-common = { path = "../furnace-rs-common", version = "=2.0.0", features = ["http"] }` (the `extra` feature is added in Task 5), `redis = { version = "0.32", default-features = false, features = ["tokio-rustls-comp", "tls-rustls-webpki-roots", "script", "connection-manager"] }`, `rustls = { version = "0.23.46", default-features = false, features = ["ring", "std", "tls12"] }`, `sha2 = "0.10"`, `tokio.workspace`, `tracing.workspace`. Dev-dependencies: `tokio` with `test-util`, `tower.workspace`, `serde_json.workspace`. Verify: `cargo check -p furnace-rs-extra` compiles.
- [ ] **Step 2: Write failing tests** in `backend.rs` `#[cfg(test)]` (use `#[tokio::test(start_paused = true)]`):
  - `incr_window_counts_within_window`: three `incr_window("k", 60s)` calls return counts 1, 2, 3 and `reset_secs` in `1..=60`.
  - `incr_window_resets_after_window`: after count 2, `tokio::time::advance(61s)`, next call returns count 1.
  - `set_ex_expires`: `set_ex("k", vec![1], 10s)`, `get` is `Some(vec![1])`; after `advance(11s)` it is `None`.
  - In `identity.rs`: `canonical_query(Some("b=2&a=1")) == "a=1&b=2"`, `canonical_query(None) == ""`; `fingerprint` is `None` with neither header, equal for equal headers, different for different `Authorization` values, and a 64-char lowercase hex string otherwise.
  - In `service.rs`: `RedisExtra::with_backend(MemoryBackend::default())` constructs (smoke).
- [ ] **Step 3: Run `cargo test -p furnace-rs-extra`** — expect FAIL (types undefined).
- [ ] **Step 4: Implement** the interfaces above. `MemoryBackend`: `Mutex<HashMap<String, (Vec<u8>/count, Instant expiry)>>`; `incr_window` sets expiry on first hit and reports remaining seconds rounded up, minimum 1. `fingerprint`: SHA-256 over `authorization` value, a `0x00` byte, then `cookie` value (missing = empty), hex-encoded.
- [ ] **Step 5: Run `cargo test -p furnace-rs-extra`** — expect PASS. Run `cargo clippy -p furnace-rs-extra --all-targets` clean.
- [ ] **Step 6: Commit** `feat(extra): add backend trait, in-memory backend and RedisExtra service`.

---

### Task 2: `RedisBackend` and `RedisExtra::new`

**Files:**
- Create: `crates/furnace-rs-extra/src/redis_backend.rs`
- Modify: `crates/furnace-rs-extra/src/service.rs`, `src/lib.rs`

**Interfaces:**
- Consumes: `Backend`, `BackendFuture`, `BackendError`, `WindowHit`, `RedisExtra::with_backend` (Task 1).
- Produces: `RedisExtra::new(url: &str) -> Result<RedisExtra, RedisExtraError>`; `RedisExtraError` (`Display` + `Error`, one variant for invalid URL). `pub(crate) struct RedisBackend`.

- [ ] **Step 1: Write failing tests** in `service.rs`: `RedisExtra::new` returns `Err` for `""`, `"http://x"`, `"redis://h:6379/#frag"`, `"localhost:6379"`; returns `Ok` for `"redis://127.0.0.1:6379"` and `"rediss://u:p@h:6379"` **without connecting** (connection is lazy).
- [ ] **Step 2: Run** `cargo test -p furnace-rs-extra service` — expect FAIL.
- [ ] **Step 3: Implement** `RedisBackend` holding `redis::Client` and a lazily initialised `tokio::sync::OnceCell<redis::aio::ConnectionManager>`; each call clones the manager (`get_or_try_init`). `get` = `GET`, `set_ex` = `SET key value PX ttl_ms`, `incr_window` = `redis::Script` with this body (the algorithm the signature does not determine):
  ```lua
  local c = redis.call('INCR', KEYS[1])
  if c == 1 then redis.call('PEXPIRE', KEYS[1], ARGV[1]) end
  local ttl = redis.call('PTTL', KEYS[1])
  if ttl < 0 then redis.call('PEXPIRE', KEYS[1], ARGV[1]); ttl = tonumber(ARGV[1]) end
  return {c, ttl}
  ```
  Map `reset_secs = max(1, ceil(ttl_ms / 1000))`. Map every redis error to `BackendError`. Before building the client for a `rediss://` URL, install the ring rustls provider only when none is set — copy `ensure_crypto_provider` from `crates/furnace-rs-socket/src/rooms/redis.rs:549`.
- [ ] **Step 4: Run tests + clippy** — expect PASS. (The Lua script and live Redis I/O are intentionally not run here; spec risk.)
- [ ] **Step 5: Commit** `feat(extra): add Redis backend and URL-validated RedisExtra::new`.

---

### Task 3: Cache middleware

**Files:**
- Create: `crates/furnace-rs-extra/src/caching.rs` (feature `cache`)
- Modify: `crates/furnace-rs-extra/src/lib.rs`
- Test: `crates/furnace-rs-extra/tests/cache.rs`

**Interfaces:**
- Consumes: `RedisExtra::backend`, `fingerprint`, `canonical_query` (Task 1).
- Produces: `#[derive(Clone)] pub struct CacheState` with `pub fn new(extra: &RedisExtra, ttl_secs: u64) -> CacheState`; `pub async fn cache_middleware(State(state): State<CacheState>, request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response`.

- [ ] **Step 1: Write failing tests** in `tests/cache.rs`. Build `Router::new().route("/p", get(handler)).route_layer(from_fn_with_state(CacheState::new(&extra, 60), cache_middleware))` with a handler that increments an `AtomicUsize` and returns `Json`; send requests with `tower::ServiceExt::oneshot`. Tests:
  - `second_get_is_served_from_cache`: two GET `/p` → handler count 1, identical bodies and `Content-Type`.
  - `query_order_does_not_matter` (Review Focus 1): `/p?a=1&b=2` then `/p?b=2&a=1` → count 1.
  - `different_credentials_do_not_share` (2): `Bearer A`, then `Bearer B`, then none → count 3; repeating `Bearer A` → still 3.
  - `set_cookie_is_not_cached`: handler adds `Set-Cookie` → two requests, count 2.
  - `oversized_body_is_not_cached`: 1 MiB + 1 byte `Vec` body → count 2.
  - `non_get_bypasses`: HEAD and POST-on-a-`any` route do not read or write the backend (use a recording backend).
  - `failing_backend_fails_open` (3): backend returning `Err` for every call → handler's normal 200 response, count increments each request.
  - `corrupt_entry_is_a_miss` (4): pre-seed the exact key via `MemoryBackend::set_ex` with `[0xFF]` → handler runs, 200.
  - `non_2xx_is_not_cached`: handler returns 404 → count 2.
- [ ] **Step 2: Run** `cargo test -p furnace-rs-extra --test cache` — expect FAIL.
- [ ] **Step 3: Implement** `cache_middleware`: bypass unless `GET`; key per Global Constraints; on `get` hit decode and rebuild the response; on miss run `next`, then cache only if status is 2xx, no `Set-Cookie`, and `body.size_hint().upper()` is `Some(n)` with `n <= 1 MiB`; buffer with `axum::body::to_bytes(body, 1 MiB)`; `set_ex` failure is logged and ignored. Value encoding (private fns `encode(status, content_type, body) -> Vec<u8>` / `decode(&[u8]) -> Option<(StatusCode, Option<HeaderValue>, Bytes)>`): `u16 BE status`, `u16 BE content-type length` (0 = none), content-type bytes, body; `decode` returns `None` on any short or inconsistent input.
- [ ] **Step 4: Run tests + clippy** — expect PASS.
- [ ] **Step 5: Commit** `feat(extra): add Redis-backed response cache middleware`.

---

### Task 4: Rate-limit middleware

**Files:**
- Create: `crates/furnace-rs-extra/src/limiting.rs` (feature `rate-limit`)
- Modify: `crates/furnace-rs-extra/src/lib.rs`
- Test: `crates/furnace-rs-extra/tests/rate_limit.rs`

**Interfaces:**
- Consumes: `RedisExtra::backend`, `WindowHit`, `fingerprint` (Task 1).
- Produces: `#[derive(Clone)] pub struct RateLimitState` with `pub fn new(extra: &RedisExtra, limit: u64, per_secs: u64, guarded: bool, route: &str) -> RateLimitState`; `pub async fn rate_limit_middleware(State(state): State<RateLimitState>, request: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response`.

- [ ] **Step 1: Write failing tests** in `tests/rate_limit.rs` (same router pattern as Task 3; inject `ConnectInfo` by inserting `axum::extract::ConnectInfo(SocketAddr)` into request extensions):
  - `allows_up_to_limit_then_429`: limit 2 → 200, 200, 429; the 429 has `Retry-After` (integer ≥ 1), `X-RateLimit-Limit: 2`, `X-RateLimit-Remaining: 0` and JSON body `{"error":{"code":"too_many_requests","message":"rate limit exceeded"}}`; allowed responses carry `X-RateLimit-Remaining` 1 then 0.
  - `window_resets` (`start_paused`): after the 429, `advance(per + 1s)` → 200 again.
  - `ips_are_separate`: two different `ConnectInfo` addresses have independent counts.
  - `missing_peer_uses_unknown_bucket`: no `ConnectInfo` → requests share one bucket.
  - `public_endpoint_ignores_authorization` (5): `guarded = false`, limit 2, three requests each with a different `Authorization` value from the same IP → third is 429.
  - `guarded_endpoint_separates_credentials`: `guarded = true`, limit 1, `Bearer A` then `Bearer B` from the same IP → both 200; `Bearer A` again → 429.
  - `failing_backend_fails_open` (3): every request → 200 without rate-limit headers.
  - `routes_do_not_share_counters`: two states with different `route` strings on one backend.
- [ ] **Step 2: Run** `cargo test -p furnace-rs-extra --test rate_limit` — expect FAIL.
- [ ] **Step 3: Implement** `rate_limit_middleware` per Global Constraints. `remaining = limit.saturating_sub(count)`.
- [ ] **Step 4: Run tests + clippy** — expect PASS.
- [ ] **Step 5: Commit** `feat(extra): add Redis-backed rate-limit middleware`.

---

### Task 5: Attribute macros and controller wiring

**Files:**
- Modify: `crates/furnace-rs-common-macros/Cargo.toml` (feature `extra = []`), `src/lib.rs`, `src/path.rs`, `src/controller/endpoints.rs`
- Create: `crates/furnace-rs-common-macros/src/controller/extras.rs`
- Modify: `crates/furnace-rs-common/Cargo.toml` (feature `extra = ["http", "dep:furnace-rs-common-macros", "furnace-rs-common-macros/extra"]`), `crates/furnace-rs-common/src/lib.rs`, `crates/furnace-rs-common/src/route.rs`
- Modify: `crates/furnace-rs-extra/Cargo.toml` (common dependency gains feature `extra`), `src/lib.rs`
- Test: `crates/furnace-rs/tests/extra_controller.rs` (`#![cfg(feature = "extra")]`), `crates/furnace-rs/tests/ui-extra/*.rs` + `.stderr`, `crates/furnace-rs/tests/extra_ui.rs`

**Interfaces:**
- Consumes: `CacheState::new`, `cache_middleware`, `RateLimitState::new`, `rate_limit_middleware`, `RedisExtra` (Tasks 1–4).
- Produces:
  - `furnace_rs_common_macros::{cache, rate_limit}` attribute macros (feature `extra`), built with `verb::outside_contract` like `seal`; re-exported as `furnace_rs_common::{cache, rate_limit}` and `furnace_rs_extra::{cache, rate_limit}`.
  - `path::extra_path() -> syn::Result<syn::Path>`: `crate_name("furnace-rs-extra")` → `::furnace_rs_extra` (`Itself` → `crate`), else `crate_name("furnace-rs")` → `::furnace_rs::extra`, else an error naming both packages.
  - `extras::take_extras(attrs: &mut Vec<Attribute>) -> syn::Result<EndpointExtras>` with `EndpointExtras { cache_ttl: Option<u64>, rate_limit: Option<(u64, u64)> }`, removing matching attributes by last path segment (`cache`, `rate_limit`) the way `seal` is removed at `endpoints.rs:135`.
  - `extras::layer_tokens(extras: &EndpointExtras, common: &Path, extra: &Path) -> TokenStream` producing, cache first then rate limit (so rate limit wraps cache):
    `.route_layer(#common::axum::middleware::from_fn_with_state(#extra::CacheState::new(&*__furnace_runtime.application().resolve::<#extra::RedisExtra>()?, #ttl), #extra::cache_middleware))` and the rate-limit equivalent passing `__furnace_routes.is_guarded()` and `__furnace_path` as the route.
  - `ValidatedRouteIter::is_guarded(&self) -> bool` (`#[doc(hidden)]`; `guard_occurrence.is_some()` under `jwt`, `false` otherwise).

- [ ] **Step 1: Write failing tests.**
  - `tests/extra_controller.rs` (facade, `--features extra`): an unsealed controller (copy the `Sealable` + `furnace_rs_testing::__private::test_fixture` pattern from `crates/furnace-rs-testing/tests/http_fixture.rs`) with `#[get("/c")] #[cache(ttl = 60)]` (counter-incrementing handler) and `#[get("/r")] #[rate_limit(limit = 1, per = 60)]`. With `.provide(RedisExtra::with_backend(MemoryBackend::default()))`: two GET `/c` → counter 1; GET `/r` → 200 then 429. A second test omits the `provide` and asserts the fixture run returns `Err` (missing service fails at router build, not at request time).
  - `tests/extra_ui.rs` + `tests/ui-extra/` trybuild compile-fail cases (follow `tests/seal_ui.rs` / `ui-seal`): `cache_on_post.rs`, `ttl_zero.rs`, `ttl_not_literal.rs`, `duplicate_cache.rs`, `unknown_key.rs`, `rate_limit_missing_per.rs`, `cache_outside_controller.rs`. Generate `.stderr` with `TRYBUILD=overwrite` and review each message reads sensibly.
- [ ] **Step 2: Run** `cargo test -p furnace-rs --features extra --test extra_controller` — expect FAIL.
- [ ] **Step 3: Implement** all items in Interfaces. `extras::take_extras` errors: `` `#[cache]` accepts only `ttl = <positive integer>` ``, `` `#[rate_limit]` requires `limit` and `per` positive integers ``, ``duplicate `#[cache]` ``. In `endpoints.rs`, call `take_extras` where `seal` is stripped; error ``` `#[cache]` is only valid on `#[get]` endpoints ``` after the verb is known, and ``` `#[cache]`/`#[rate_limit]` require an HTTP endpoint ``` if the method has no verb; pass `layer_tokens(...)` as `HandlerAdapter.guard_layer`. Gate the new code with `#[cfg(feature = "extra")]`; without the feature `guard_layer` stays empty. Add the `extra` feature plumbing to `furnace-rs-common` (re-export the two macros under `#[cfg(feature = "extra")]`).
- [ ] **Step 4: Run** `cargo test -p furnace-rs --features extra` and `cargo test -p furnace-rs-common-macros --features extra` — expect PASS. Then `cargo test --workspace` (default features) to confirm nothing else changed.
- [ ] **Step 5: Commit** `feat: add cache and rate_limit endpoint attributes`.

---

### Task 6: Facade wiring and documentation

**Files:**
- Modify: `crates/furnace-rs/Cargo.toml`, `crates/furnace-rs/src/lib.rs`, `crates/furnace-rs/tests/feature_matrix.rs` (only if it enumerates features)
- Modify: `crates/furnace-rs-extra/README.md`, `README.md` (crate map line 230, feature table line 267), `CHANGELOG.md`
- Create: `docs/guides/extra.md`

**Interfaces:**
- Consumes: everything above. Produces: `furnace_rs::prelude::{cache, rate_limit, RedisExtra, MemoryBackend, Backend}` under the `extra` feature.

- [ ] **Step 1: Facade.** Facade `extra` feature becomes `["http", "dep:furnace-rs-extra"]`; add the prelude re-exports (`#[cfg(feature = "extra")]`). Verify `cargo check -p furnace-rs --no-default-features --features extra` and `cargo check -p furnace-rs --features extra`.
- [ ] **Step 2: Docs.** Rewrite `furnace-rs-extra/README.md` (remove "no HTTP dependency / placeholder" claims; new dependency boundary; features `cache`, `rate-limit`). `docs/guides/extra.md`: registration example, both attributes, key and identity rules, fail-open behaviour, the non-goals, and this limitation: **`Furnace::burn::<App>()` builds its own builder and cannot receive a provided `RedisExtra`; use `Furnace::builder_with_config(..)` + `provide` + `build_router` + `serve_router`**, and that behind a proxy all clients share one rate-limit bucket. Update the root README crate/feature rows and add a `CHANGELOG.md` entry. Update the crate-level doc comment in `extra/src/lib.rs` (it still says "reserved").
- [ ] **Step 3: Full verification.** `cargo fmt --check`, `cargo clippy --workspace --all-targets --features furnace-rs/extra -- -D warnings`, `cargo test --workspace`, `cargo test -p furnace-rs --features extra`. Expected: all green.
- [ ] **Step 4: Commit** `docs: document furnace-rs-extra cache and rate limiting`.

---

## Self-Review

- **Spec coverage:** goals → Tasks 1–5; registration → Tasks 1–2, 6; cache rules → Task 3; rate-limit rules → Task 4; ordering → Task 5 (`guard_layer` slot, chain order); macro validation → Task 5; tests → each task; README boundary change → Task 6; missing-service diagnostic → Task 5 test.
- **Types:** `CacheState::new(&RedisExtra, u64)`, `RateLimitState::new(&RedisExtra, u64, u64, bool, &str)`, `Backend` method names are used identically in Tasks 1–5.
- **Known gaps (not tasks):** `RedisBackend` Lua and TLS are not run against a real Redis (decided); no shared key prefix, so two apps on one Redis share keys; `Furnace::burn` cannot receive the service (documented in Task 6).
