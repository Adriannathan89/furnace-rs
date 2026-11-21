# Redis caching and rate limiting

`furnace-rs-extra` adds two endpoint attributes backed by Redis, so counters
and cached responses are shared by every instance of an application.

A runnable project is in [`example/cache-rate-limit`](../../example/cache-rate-limit/).

## Dependencies

```toml
[dependencies]
furnace = { package = "furnace-rs", version = "=2.0.0", features = ["extra"] }
```

## Registration

Import `ExtraCauldron` into the root cauldron, then give the Redis URL to the
application builder. A rooted application rejects `provide` for a type that no
imported cauldron registers, so the cauldron is required. Only `redis://` and
`rediss://` URLs are accepted. The connection opens lazily on first use, so an
unreachable Redis does not block startup.

```rust,ignore
use furnace::prelude::*;

#[cauldron(imports = [ExtraCauldron], controllers = [PostController])]
struct AppCauldron;

let mut builder = Furnace::builder_with_config(config);
builder.root::<AppCauldron>()?;
builder.provide(RedisExtra::new("redis://127.0.0.1:6379")?)?;
let application = builder.build().await?;
let router = build_router(&application)?;
serve_router(application, router, "127.0.0.1:3000").await?;
```

`Furnace::burn::<AppCauldron>()` builds its own builder and cannot receive a
provided `RedisExtra`; use the low-level path above. If an application
imports `ExtraCauldron` but never provides the instance, it still starts: the
placeholder fails every Redis call, so both features fail open and each request
logs `RedisExtra was not provided`. A controller using either attribute fails
router construction when `RedisExtra` is not registered at all.

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
- On a guarded endpoint a client is the credential the Passport guard
  validated (the Bearer token or the guard's cookie), hashed with SHA-256. Other
  headers, a different scheme casing, or extra whitespace do not change it. On a
  public endpoint a client is the peer IP, so made-up credentials cannot create
  new buckets. Without a peer address all requests share one bucket.
- `X-Forwarded-For` is not trusted: behind a proxy every client shares the
  proxy's address on public endpoints.

## Failure behavior

If Redis errors at request time, both features log a warning and let the
request through (fail open). Each Redis call is bounded to 250 ms, so a stalled
or unreachable Redis adds at most that per call (a cache miss makes up to two:
the read and the write). Undecodable cache entries count as misses.

## Limits and testing

- Applications sharing one Redis and identical controller module paths share
  rate-limit counters, and cache keys carry no application namespace at all
  (two applications serving the same path share entries); there is no key
  prefix option. Give each application its own Redis database or instance.
- Redis Sentinel and cluster are not supported.
- Tests can supply `RedisExtra::with_backend(MemoryBackend::default())`, or
  implement `Backend` for another store.
