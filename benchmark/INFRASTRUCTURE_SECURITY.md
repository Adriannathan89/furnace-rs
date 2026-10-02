# Core and PostgreSQL connector security review

The second review covered core diagnostic/configuration handling, provider
construction and lookup, lifecycle failure wrapping, and the persistence
connector's configuration, connection, readiness, shutdown, and error paths.
It found three reproducible issues. This is a targeted implementation review;
it does not establish that every application or dependency is vulnerability-free.

## 1. Core error debug formatting exposed retained causes

`furnace-rs-core::Error` derived `Debug`, recursively formatting its boxed source.
A safe framework diagnostic with a database URL stored only in a native source
error exposed the password through `{:?}` and `{:#?}`. Debug logs and Rust's
default termination output can therefore disclose otherwise hidden details.

The custom `Debug` implementation now shows authored diagnostics and whether
a source exists, without formatting the source. `Display` continues showing
diagnostics. Typed causes remain available through explicit `source()` access.
Applications remain responsible for keeping their authored diagnostic messages
and explicit source logging free of credentials. The regression exercises a
nested error and confirms that typed source access still works.

## 2. Unrepresentable database timeouts caused a panic

The connector accepted `u64::MAX` timeout seconds and native `Duration::MAX`.
SQLx 0.9.0 adds acquisition timeouts to an `Instant` without checking overflow;
the native acquire-timeout reproduction panicked with
`overflow when adding duration to instant`, before attempting a connection.

Typed connect/acquire/idle/lifetime settings now use checked deadline arithmetic
and return a value-free `out_of_range` issue attributed to the setting's source.
The connector also validates these native options immediately before connecting,
including options supplied through `from_options` or changed through
`options_mut`, even for lazy pools. Failures return `InvalidConfiguration`.
Valid durations and explicitly disabled idle/lifetime policies remain supported.

This addresses a configuration-triggered availability issue. The reproduction
requires control of configuration or connector options; it is not evidence that
an ordinary HTTP request can change those values. This check rejects deadlines
the platform cannot represent, rather than imposing an arbitrary timeout ceiling.
Application callbacks that override SQLx pool options directly remain responsible
for the options they produce.

## 3. Native connection tracing exposed the database URL

SeaORM 2.0.3 instruments PostgreSQL connection establishment with a trace span
that records `ConnectOptions`, whose debug output includes the complete URL.
With a tracing subscriber enabled, a connector call disclosed the URL password.
This was reproduced with a malformed port, so no database or network was needed.

The native connection future now runs with a scoped `NoSubscriber` dispatcher
on each poll. This suppresses credential-bearing connection spans and events.
The application subscriber is restored immediately outside the future; tests
check events both before and after connecting. Configured statement logging on
the returned connection remains native behavior. Connection-handshake telemetry,
including tracing from callbacks polled inside that operation, is intentionally
suppressed. This protection covers calls through Furnace's connector; direct
SeaORM calls and explicitly printing native options bypass this boundary.

## Reproduction and benchmark evidence

Each finding had a failing regression before its fix. The repeated Rust
workload under `crates/furnace-rs-persistence/tests/security_benchmark.rs` is a persistence integration
test, so workspace test runs execute its security contracts automatically.

| Contract | Reproduction failures | Final stress checks | Final failures |
| --- | ---: | ---: | ---: |
| Core source redaction | 2 / 2 | 200 | 0 |
| Typed database timeout rejection | 8 / 8 | 800 | 0 |
| Native database timeout rejection without panic | 2 / 2 | 200 | 0 |
| Connection tracing and restored application tracing | 2 / 2 | 200 | 0 |
| Valid configuration controls | 0 / 8 | 800 | 0 |

The stress profile runs 200 rounds, totaling 2,200 checks without a database.
Raw results are stored in `benchmark/results/2026-10-02-infrastructure-*.json`.
Reproduction evidence is split into two stages: the initial core/timeout run,
and a tracing run after those first fixes but before the tracing fix. These are
working-tree builds based on commit `6edc43f`; the final report records a hash
of the relevant implementation and benchmark sources. Neither elapsed times
nor the different stages imply a production performance comparison.

```sh
python3 benchmark/tool/infrastructure_security.py --profile smoke
python3 benchmark/tool/infrastructure_security.py --profile stress \
  --output /tmp/furnace-infrastructure-security.json
cargo test --offline -p furnace-rs-persistence --all-features --test security_benchmark -- --nocapture
python3 -m unittest discover -s benchmark/tool -p 'test_*.py'
```

The Python runner rejects incomplete case counts, reported failures, missing or
ambiguous results, and nonzero Cargo exits. Its elapsed time covers the Rust
contracts and excludes Cargo compilation. Existing PostgreSQL integration tests
still require an isolated `FURNACE_TEST_DATABASE_URL` and are not replaced by
these checks.

## Final verification

- The initial full workspace suite with all features passed. Four existing PostgreSQL
  integration tests were ignored because no isolated database was configured.
- Core and persistence Clippy checks passed with all features and targets and
  warnings denied. Their builds with default features disabled also passed.
- The initial benchmark Python suite ran 34 tests successfully, with three existing
  PostgreSQL checks skipped because no isolated database was configured.
- Package-content and release-policy checks passed after placing the Rust
  benchmark in the persistence crate's packaged test directory.
- Final smoke and stress security workloads passed all contracts.

All repository Cargo `target` directories were removed after verification at
the user's request. Re-running compiled checks will rebuild their artifacts.

## Live PostgreSQL validation

The previously ignored/skipped database checks were subsequently run against a
real, isolated PostgreSQL 16.15 server. The temporary cluster listened only on
localhost, required SCRAM-SHA-256 password authentication for TCP connections,
and used separate persistence and benchmark databases. No system database or
service configuration was changed.

- All four PostgreSQL integration tests passed, with no ignored tests: native
  CRUD and commit/rollback, invalid credential rejection with source redaction,
  pool shutdown after an application stop failure, and database readiness before
  HTTP binding followed by graceful SIGTERM handling.
- All 34 benchmark Python tests passed, with no skips. The three live database
  checks verified table-lock acquisition/release, HTTP availability and query
  recovery after a statement timeout, and HTTP availability and pool recovery
  after a stalled PostgreSQL TCP reply.
- The security regression smoke workload passed all 22 checks again. Those
  contracts remain independent of a database; the integration checks above
  supply the real-server validation.

The persistence suite used the workspace's SeaORM 2.0.3. The staged posts CRUD
example used its declared SeaORM 2.0.0 and the patched Furnace crates from this
checkout. Builds and staged examples lived under `/tmp`; the Python build tests'
root was redirected to a temporary mirror to preserve their explicit target
directory behavior without rebuilding inside the repository.

The server was stopped and its cluster, credentials, staged examples, and build
artifacts removed after verification. Repository Cargo `target` directories
remain absent. Results, source hash, test counts, and cleanup status are recorded
in [the PostgreSQL validation result](results/2026-10-02-infrastructure-postgres-validation.json).

To rerun against an isolated PostgreSQL database, set
`FURNACE_TEST_DATABASE_URL` and run:

```sh
cargo test --offline --locked -p furnace-rs-persistence \
  --features sea-orm-postgres --test postgres -- --ignored --test-threads=1
```

For the benchmark database checks, apply
`example/posts-crud/migrations/001_create_posts.sql` to a separate test database,
set `FURNACE_TEST_BENCH_DATABASE_URL`, and set `FURNACE_TEST_BENCH_APP_ROOT` to a
root containing a built `example/posts-crud/target/debug/furnace-rs-example-posts-crud`:

```sh
python3 -m unittest discover -s benchmark/tool -p 'test_run.py'
```
