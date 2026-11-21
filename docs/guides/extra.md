# Redis caching and rate limiting

`furnace-rs-extra` adds two endpoint attributes backed by Redis, so counters
and cached responses are shared by every instance of an application.

## Dependencies

```toml
[dependencies]
furnace = { package = "furnace-rs", version = "=2.0.0", features = ["extra"] }
```

## Registration

Give the Redis URL to the application builder. Only `redis://` and `rediss://`
URLs are accepted. The connection opens lazily on first use, so an unreachable
Redis does not block startup.

```rust,ignore
use furnace::prelude::*;

let mut builder = Furnace::builder_with_config(config);
builder.root::<AppCauldron>()?;
builder.provide(RedisExtra::new("redis://127.0.0.1:6379")?)?;
let application = builder.build().await?;
let router = build_router(&application)?;
serve_router(application, router, "127.0.0.1:3000").await?;
```

`Furnace::burn::<AppCauldron>()` builds its own builder and cannot receive a
provided `RedisExtra`; use the low-level path above. A controller that uses
either attribute fails router construction when `RedisExtra` was not provided.

## Attributes

Place them under the HTTP verb attribute in a `#[controller]` implementation.
Arguments must be positive integer literals.

```rust,ignore
#[get("/posts")]
#[rate_limit(limit = 30, per = 60)]   // 30 requests per 60 seconds
#[cache(ttl = 120)]                    // seconds
async fn list(&self) -> HttpResult<Json<Vec<Post>>> { /* ... */ }
```

Order of effect: Passport guard, then rate limit, then cache, then handler.
Cache hits still count against the limit.

### `#[cache(ttl = N)]`

- Only `#[get]` endpoints. Only 2xx responses are stored: status,
  `Content-Type`, and body.
- Responses with `Set-Cookie`, bodies over 1 MiB, and bodies of unknown size
  (streams) are passed through uncached.
- The key is method, path, sorted query string, and a SHA-256 fingerprint of
  the `Authorization` and `Cookie` headers (or `anon`). One caller never sees
  another's entry. The fingerprint is per credential, not per user, so a user
  with two tokens has two entries. Raw credentials are never stored or logged.
- Entries expire after the TTL. There is no invalidation API.

### `#[rate_limit(limit = N, per = S)]`

- Fixed window per endpoint and client. Over the limit the response is
  `429` with `Retry-After`, and every response carries `X-RateLimit-Limit`
  and `X-RateLimit-Remaining`.
- On a guarded endpoint a client is its credential fingerprint. On a public
  endpoint it is the peer IP, so made-up credentials cannot create new buckets.
  Without a peer address all requests share one bucket.
- `X-Forwarded-For` is not trusted: behind a proxy every client shares the
  proxy's address on public endpoints.

## Failure behavior

If Redis errors at request time, both features log a warning and let the
request through (fail open). Undecodable cache entries count as misses.

## Limits and testing

- Applications sharing one Redis and identical controller module paths share
  rate-limit counters; there is no key prefix option.
- Redis Sentinel and cluster are not supported.
- Tests can supply `RedisExtra::with_backend(MemoryBackend::default())`, or
  implement `Backend` for another store.
