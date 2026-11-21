# Cache and rate limiting

Redis-backed `#[cache(ttl = N)]` and `#[rate_limit(limit = N, per = S)]` on controller
endpoints, from `furnace-rs-extra` (the facade's `extra` feature).

- `GET /articles` is limited to 20 requests per minute per client and cached for 30 seconds.
  The handler sleeps 300 ms to stand in for a slow query.
- `POST /auth/login` allows 5 attempts per minute per client IP.

Requires Rust 1.94 or newer. From this directory, either use the built-in in-memory backend
(one process only):

```sh
cargo run
```

or share state through Redis:

```sh
docker run --rm -p 6379:6379 redis:7
REDIS_URL=redis://127.0.0.1:6379 cargo run
```

In another terminal:

```sh
# Slow the first time (~300 ms), then instant, with the same "generated" number.
curl -s -w '  %{time_total}s\n' http://127.0.0.1:3006/articles
curl -s -w '  %{time_total}s\n' http://127.0.0.1:3006/articles

# Six wrong attempts: five 401 responses, then 429 with Retry-After.
for i in 1 2 3 4 5 6; do
  curl -s -o /dev/null -w '%{http_code}\n' -X POST http://127.0.0.1:3006/auth/login \
    -H 'content-type: application/json' -d '{"password":"nope"}'
done
curl -si -X POST http://127.0.0.1:3006/auth/login \
  -H 'content-type: application/json' -d '{"password":"correct-horse"}' | head -8
```

`cargo test` runs the same scenario in process against the in-memory backend.

## How it fits together

- `#[cauldron(root, imports = [ExtraCauldron], ...)]` registers the `RedisExtra` slot. A rooted
  application rejects `builder.provide(..)` for a type no cauldron registers.
- `main` builds with the low-level builder, because `Furnace::burn` cannot receive a provided
  `RedisExtra`: `Furnace::builder()`, `root`, `provide(RedisExtra::new(url)?)`, `build`,
  `build_router`, `serve_router`.
- Order of effect on one endpoint: Passport guard (none here), rate limit, cache, handler.
- If Redis is unreachable, both features log a warning and let requests through.
- Behind a reverse proxy every client shares the proxy's address, so IP-based limits on public
  endpoints become one shared bucket. See the [guide](../../docs/guides/extra.md).
