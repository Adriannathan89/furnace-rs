# Validated extractor error redaction

Date: 2026-10-09. Branch: `feat/security-finder`.
Audited baseline: `b8e8ae72504cad986355c11ceecab76f9370a6f2`.
Affected package: `furnace-rs-common` 1.0.1 source, `http` feature.

## Confirmed issue and prerequisites

`ValidatedJson`, `ValidatedQuery`, and `ValidatedPath` classify native Axum
deserialization failures into safe HTTP 422 validation responses. The native
error text can originate in an application's custom Serde implementation. The
baseline recognizes text shaped like `missing field \`…\`` and copies the
substring between the backticks into `error.issues[].path`.

An unauthenticated caller who can reach a route with an affected custom
deserializer can trigger a rejection that discloses private data from that
deserializer's error message. Authentication requirements depend on the route.
The framework itself does not fetch a secret: the leak requires the native
error to contain private data in this particular missing-field shape. Ordinary
derived deserialization of public schema fields is not itself a secret leak.
This is an information disclosure, not a demonstrated authentication bypass.

## Reproduction and measured baseline

The regression fixture `MissingFieldMessageInput` implements Serde's map
visitor and returns this synthetic private diagnostic:

```rust
Err(serde::de::Error::custom(
    "missing field `PRIVATE_SERVER_SENTINEL`",
))
```

The fixture is used by three real Axum routes. The handlers panic if invoked,
so the tests also require rejection before handler execution. Requests are
`POST /input` with JSON `{}`, `GET /?value=1`, and `GET /1` with a path capture.
No real credentials, external services, or databases are used.

With the new regression tests installed and production code still at the
baseline, this command ran **three tests and failed all three**, exit 101:

```sh
cargo test -p furnace-rs-common --test validated_extractors \
  missing_field_custom_error -- --nocapture
```

The [captured baseline output](evidence/2026-10-09-extractor-error-baseline.txt)
records the three leaking responses and expected assertion failures.

For each request, HTTP 422 contained the sentinel in its response path:

```json
{"error":{"code":"validation_error","message":"input validation failed","issues":[{"source":"body","path":["PRIVATE_SERVER_SENTINEL"],"code":"required","message":"required value is missing"}]}}
```

The query and path cases returned the same leaked path with `source` set to
`query` and `path`, respectively. JSON adds its native line/column suffix;
that suffix did not prevent the disclosure.

To independently reproduce the baseline after this patch, create a disposable
checkout of the baseline and copy only the current regression test file into
it. The filter selects the three new tests, without needing the other test
expectation changes:

```sh
scratch=$(mktemp -d /tmp/furnace-extractor-repro.XXXXXX)
git archive b8e8ae72504cad986355c11ceecab76f9370a6f2 | tar -x -C "$scratch"
cp crates/furnace-rs-common/tests/validated_extractors.rs \
  "$scratch/crates/furnace-rs-common/tests/validated_extractors.rs"
(cd "$scratch" && cargo test --locked -p furnace-rs-common \
  --test validated_extractors missing_field_custom_error -- --nocapture)
```

## Patch method

In `src/validation/rejection.rs`, classify native missing-field error text
with boolean predicates instead of returning strings to append to response
paths. Remove the query/path fallback that turns native message text into
field metadata. JSON retains its structured `serde_path_to_error` containing
path, and query/path failures retain structured paths or keys when provided
by the native extractor. No native error substring is promoted into a field
name by the missing-field classifier.

All three fixed reproductions return HTTP 422 with the same fixed `required`
code/message and an empty root path. The sentinel is absent. The full
`validated_extractors` integration suite ran **25 tests, all passed**, exit 0:

```sh
cargo test --locked -p furnace-rs-common --all-features --test validated_extractors
```

The [captured patched output](evidence/2026-10-09-extractor-error-patched.txt)
includes the three redaction regressions and the nested containing-path control.

## Compatibility and limits

Missing field names recovered only from error strings are no longer included
in validation paths, even for normal derived Serde implementations. At the
root, such a missing-field issue has `path: []`; nested structured context is
retained. Existing tests for normal missing JSON/query/path fields were
updated to assert this intentional response contract change. Conversion
errors and post-deserialization validator paths retain their existing behavior.

Applications still own the safety of explicit `ValidationIssue::custom`
messages and manually supplied paths. Native Axum extractors remain governed
by Axum's response behavior. The patch does not claim to sanitize arbitrary
application responses or hide public, structured request/schema keys.

## Final verification

- `cargo test --locked --workspace --all-features`: exit 0, including doc
  tests. Seven PostgreSQL integration tests remained explicitly ignored
  because no `FURNACE_TEST_DATABASE_URL` was configured; no live database
  coverage is claimed for this patch.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  exit 0.
- `cargo fmt --all --check` and `git diff --check`: exit 0.
- An independent code-review agent reported no material issues with the fix,
  regression tests, compatibility notes, or reproduction instructions.

The initial workspace build exhausted disk space while generating debug
artifacts. After removing regenerable Cargo debug artifacts, verification
completed with `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and
`CARGO_INCREMENTAL=0` set only for the build/test commands. No repository build
profile or source configuration was changed for this workaround.

## Investigated hypothesis that was not patched

A separate loopback HTTP/2 probe sent an empty DATA frame every second to test
whether empty frames could renew Furnace's body idle deadline. The response
timed out after approximately ten seconds, and the server-side body observer
received no empty frames. Source inspection confirmed that locked `h2` 0.4.19
discards empty nonterminal DATA frames in `proto/streams/recv.rs`. The probe's
reachability assertion failed, so it did not establish a Furnace vulnerability.
Its temporary test was removed; no timeout patch or vulnerability claim was
added for this hypothesis.
