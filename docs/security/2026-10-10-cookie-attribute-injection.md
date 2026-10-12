# Checked cookie attribute delimiter injection

Baseline: `bc16a4fc20a7a03ff13cda98de93121fbd2a8541` (1.0.2 source).
Affected implementation: `furnace-rs-common/src/cookie.rs::validate_pending`.

## Reproduction and prerequisites

Checked response composition validated `SameSite`/`Secure` properties and HTTP
header syntax, but accepted semicolons inside `Path` and `Domain`. Those
attributes are emitted verbatim by the cookie dependency. For example:

```rust
Cookie::build(("session", "opaque"))
    .http_only(true)
    .secure(true)
    .same_site(SameSite::Strict)
    .domain("example.com; SameSite=None; Max-Age=31536000")
    .build()
```

A real Furnace `CookieJar` response accepted this value and emitted both the
intended `SameSite=Strict` and the injected `SameSite=None`/`Max-Age` attributes.
Reparsing the emitted header with locked `cookie` 0.18.1 observed `SameSite=None`.
The same delimiter problem affects `Path` and deletion cookies. This demonstrates
policy-changing serialized attributes, not a measured browser exploit.
Remote exploitation requires an application to construct Path/Domain from
untrusted input. Furnace does not automatically map request input into these
attributes, and this finding is not a default authentication or CSRF bypass.

```sh
cargo test --locked -p furnace-rs-common --no-default-features --features cookies --test cookie_response -- --nocapture
```

Before the fix, five new regression tests failed: add/removal for Path/Domain
and a matrix of ASCII-control attribute values. Baseline add/removal responses
returned HTTP 200 instead of the expected safe rejection.

## Patch method and compatibility

Reject semicolons and ASCII controls in Path/Domain before emitting the pending
cookie batch. The existing rejection is a redacted HTTP 500 with no Set-Cookie
fields, including when valid cookies precede the rejected cookie. Both addition
and deletion pass through this validation. Names and values retain their existing
percent encoding; valid attribute behavior remains covered by the cookie suites.

The patch does not add domain ownership, public-suffix, hostname normalization,
CSRF, or token policy enforcement. Applications continue to select appropriate
cookie attributes; Furnace prevents a supplied attribute string from adding
new serialized attributes.

Evidence: [baseline](evidence/2026-10-10-cookie-red.txt),
[patched](evidence/2026-10-10-cookie-green.txt).
