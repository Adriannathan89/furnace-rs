# furnace-rs-common

`furnace-rs-common` is the integration boundary between the framework-neutral
[furnace-rs-core](../furnace-rs-core/README.md) and the web/authentication
ecosystem. It owns optional Axum, JWT, cookie, Passport,
validation, REST-error, and CORS behavior.

Application authors normally use the
[`furnace-rs` facade](../furnace-rs/README.md). Depend on `furnace-rs-common` directly when
building or testing an integration boundary, or when a tool needs the private
HTTP inspection contract used by `furnace-rs-cli`.

## Feature model

The crate itself has default feature `http`. The public `furnace-rs`
facade disables those defaults and maps its own features explicitly.

| Feature | Adds | Requires |
| --- | --- | --- |
| `http` | Axum 0.8 routing/server delivery, route/controller contracts, extractors, validation, REST errors, CORS, and native Axum/Tower re-exports. | `furnace-rs-core` and the HTTP dependency set. |
| `logger` | Tracing-based application logging. | Tracing dependencies. |
| `jwt` | `JwtService`, claims, validation profiles, algorithms, keyrings, and JWT auto-configuration. | No Axum or database dependency. |
| `cookies` | Strict cookie extraction and checked response-cookie composition. | Implies `http` and enables the cookie/Passport macro support. |

Passport route guards and managed strategies are available when `http + jwt`
are selected. Cookie-backed guards additionally need `cookies`. The
Persistence is supplied separately by `furnace-rs-persistence`; this crate has no
database feature or automatic database-to-HTTP error conversion.

## How this crate fits the runtime

`furnace-rs-common` receives a constructed core application and a rooted module
scope:

~~~text
furnace-rs-core application
        │
        ▼
HttpApplicationScope
        │
        ├── route/controller catalog validation
        ├── guard and Passport preflight
        ├── generated + native Axum router composition
        ├── application-wide CORS configuration
        └── JWT/cookie integration lifecycle
~~~

Route metadata is validated before generated registrars install routes.
Controllers are resolved once while the router is built; request handling uses
the captured application-scoped handles. The standard server path configures
the final router, starts lifecycle hooks, waits for infrastructure readiness,
binds the listener, serves, and then shuts down in reverse order.

The JWT service can be used without HTTP. Passport adds a typed strategy and
principal layer on top of verified JWT claims, with guard policy resolved
before requests. Native Axum routes remain available as an escape hatch and do
not silently become FURNACE-managed routes.

## Public areas

| Area | Main APIs |
| --- | --- |
| Routing | `controller`, `Sealable`, `seal(skip)`, `get`/`post`/`put`/`patch`/`delete`, `build_router`, `configure_router`, `serve_router` |
| Requests | Native Axum extractors plus `ValidatedJson`, `ValidatedQuery`, and `ValidatedPath` |
| Errors | `BadRequest`, `Unauthorized`, `Forbidden`, `NotFound`, `Conflict`, `ValidationError`, `InternalError` |
| Authentication | `JwtService`, `PassportStrategy`, `PassportPrincipal`, `Authenticated`, `PassportGuard` |
| Cookies | `CookieJar` and checked response-cookie composition |
| Configuration | Core `Config`/`Configuration` values consumed by HTTP, CORS, and Passport integration |

## Dependencies

First-party dependencies:

- `furnace-rs-core` is always present.
- `furnace-rs-common-macros` is optional and is enabled by the HTTP/Passport macro
  features that need it.

Important external dependencies are grouped by responsibility:

- HTTP: `axum`, `axum-extra`, `tower`, `tower-http`, `tokio`, and
  platform-specific `rustix`.
- Serialization and extraction: `serde`, `serde_json`,
  `serde_path_to_error`, and `serde_urlencoded`.
- JWT and keys: `jsonwebtoken` and `base64`.
- Cookies: `cookie`.

`furnace-rs` depends on this crate for application integrations. `furnace-rs-cli` also
depends on it directly with `http` and default features disabled for private
application inspection.

## Source layout

- `src/route.rs` and `src/router.rs` — route metadata, validation, registrar
  dispatch, and generated router construction.
- `src/server.rs`, `src/server_config.rs`, and `src/cors.rs` — standard
  startup, explicit serving, binding, and outer router configuration.
- `src/http_scope.rs` and `src/inspection.rs` — rooted HTTP selection and
  side-effect-free inspection reports.
- `src/extract/` and `src/validation/` — native/validated request extraction and
  ordered input issues.
- `src/response.rs` — safe REST response envelopes and error mapping.
- `src/jwt/`, `src/passport/`, and `src/cookie.rs` — authentication, guard
  policy, principals, strategies, and cookie handling.
- `src/lib.rs` — feature gates, public exports, and hidden contracts consumed by
  generated code and the CLI.

## Tests and contributor workflow

Run the focused integration tests with:

~~~sh
cargo test -p furnace-rs-common --all-features
~~~

HTTP, validation, auth, and lifecycle tests can run without external services.
The `furnace-rs-persistence` PostgreSQL acceptance suite is ignored by default and
requires PostgreSQL 16 through `FURNACE_TEST_DATABASE_URL`. Route and macro consumer behavior may also
require the fixtures under `crates/furnace-rs/tests/ui`.

See the [architecture reference](../../docs/ARCHITECTURE.md), the
[CLI contract](../../docs/CLI.md), and the
[Passport example](../../docs/examples/passport_jwt.md) before changing a
public integration contract.

Controllers without `impl Sealable` are public. Implement the trait to attach
one guard; `#[seal(skip)]` on an endpoint bypasses that guard's authentication
and policy checks. Protection applies per HTTP method, including when public
and protected methods share a path.
