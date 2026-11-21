# furnace-rs-extra

Optional Redis-backed endpoint caching and rate limiting for furnace-rs.

~~~rust,ignore
#[get("/posts")]
#[rate_limit(limit = 30, per = 60)]
#[cache(ttl = 120)]
async fn list(&self) -> HttpResult<Json<Vec<Post>>> { /* ... */ }
~~~

The attributes are applied by `#[controller]` and need a `RedisExtra` service
supplied to the application builder. See the
[usage guide](../../docs/guides/extra.md) for registration, keys, limits, and
failure behavior.

## Features

| Feature | Provides |
| --- | --- |
| `cache` (default) | `CacheState`, `cache_middleware` |
| `rate-limit` (default) | `RateLimitState`, `rate_limit_middleware` |

`RedisExtra`, the `Backend` trait, and `MemoryBackend` are always available.
The facade's `extra` feature enables this crate and `http`.

## Dependency boundary

Unlike the framework-neutral core, this crate depends on `furnace-rs-common`
(HTTP/Axum) and the `redis` client, which is why it is optional. The attribute
macros live in `furnace-rs-common-macros` behind its `extra` feature.

## Source layout

- `backend.rs`: `Backend` trait and `MemoryBackend`.
- `redis_backend.rs`: lazy Redis implementation (Lua fixed-window counter).
- `service.rs`: `RedisExtra` and Redis URL validation.
- `caching.rs`, `limiting.rs`: the two Axum middlewares.
- `identity.rs`: credential fingerprint and query canonicalization.

## Tests

~~~sh
cargo test -p furnace-rs-extra
cargo test -p furnace-rs --features extra   # attribute and compile-fail tests
~~~

The suite uses `MemoryBackend`; the Redis implementation is not exercised
against a live server.
