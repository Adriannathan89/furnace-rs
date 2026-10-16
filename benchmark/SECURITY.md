# Security findings, fixes, and evidence

This page consolidates benchmark security findings, fixes, and evidence. It
summarizes the checks and links to regression tests, raw results, and the
separate all-crate audit. Each summary states the limits of its evidence;
passing a workload is not a claim that the framework has no other
vulnerabilities.

## Finding and evidence index

| Area | Fix or result | Summary and evidence |
| --- | --- | --- |
| Bearer credential parsing | Require the RFC 6750 ASCII-space grammar and validate token characters and padding before authentication. The before/after reproduction is below. | [This report](#bearer-credential-parsing-hardening), [before/after JSON](results/2026-10-02-security-before.json), [patched JSON](results/2026-10-02-security-after.json) |
| Conditional gRPC admission policy (F01) | Apply the same compile-time conditions to active methods and their policy overrides; reject unsupported conditional policy declarations. | [F01 summary](#f01-conditional-grpc-admission-policy), [before/after JSON](results/2026-10-15-grpc-policy-before-after.json), [regression fixture](../crates/furnace-rs-grpc/tests/grpc_conditional_policy.rs) |
| Core errors and PostgreSQL connector | Redact retained causes in ordinary error formatting; reject invalid timeout bounds and suppress credential-bearing connection tracing. | [Core and connector summary](#core-errors-and-postgresql-connector), [core/timeout baseline](results/2026-10-02-infrastructure-core-and-timeouts-before.json), [tracing baseline](results/2026-10-02-infrastructure-tracing-before.json), [patched evidence](results/2026-10-02-infrastructure-security-after.json), [live PostgreSQL validation](results/2026-10-02-infrastructure-postgres-validation.json) |
| Database, JWT, and inventory regressions | Preserve redacted database failures, JWT algorithm binding, and safe catalog construction while recovering from database faults. | [Database, JWT, and inventory summary](#database-jwt-and-inventory-regressions), [before/after JSON](results/2026-10-06-database-inventory-security-before-after.json) |
| HTTP request deadlines | Bound incomplete headers and stalled body reads while preserving safe timeout responses and connection behavior. | [HTTP runtime summary](#http-runtime-request-deadlines), [before/after JSON](results/2026-10-03-http-runtime-security-before-after.json) |
| Local CLI control-directory permissions | Create sensitive Unix control directories with owner-only `0700` permissions at creation. The test checks multiple umasks in child processes. | [All-crate report](ALL_CRATES_SECURITY_AUDIT.md), [before](results/2026-10-02-cli-security-before.json), [patched stress run](results/2026-10-02-cli-security-stress.json) |
| Cookie, extractor, peer-address, and native pool checks | Record focused fixes and their test evidence in the repository security review. | [Security audit index](../docs/SECURITY_AUDIT.md), [cookie validation](../docs/security/2026-10-10-cookie-attribute-injection.md), [extractor redaction](../docs/security/2026-10-09-extractor-error-redaction.md), [peer address](../docs/security/2026-10-10-http-peer-address.md), [pool capacity](../docs/security/2026-10-10-native-pool-capacity.md), [scaffold privacy](../docs/security/2026-10-10-scaffold-directory-privacy.md) |
| Dependency audit | Check every committed lockfile against the recorded RustSec advisory snapshot; the report documents snapshot and yank-status limits. | [All-crate report](ALL_CRATES_SECURITY_AUDIT.md), [dependency audit JSON](results/2026-10-02-all-crates-dependency-audit.json), [verification JSON](results/2026-10-02-all-crates-verification.json) |

The repository-wide reporting policy and current scope are in
[`docs/SECURITY.md`](../docs/SECURITY.md). These benchmark records preserve
reproductions and results for specific fixes.

## F01: conditional gRPC admission policy

The <code>2.0.0-beta.2</code> baseline (<code>bd8ade6</code>) emitted policy
overrides even when the method they described was disabled by <code>cfg</code>.
A disabled public implementation could therefore weaken the active conditional
implementation's admission policy. The patch applies matching conditions to
callable-method metadata and policy overrides, including conditions nested in
<code>cfg_attr</code>. Conditional <code>grpc_method</code> helper declarations
are rejected explicitly, and active duplicate overrides remain errors. Related
macro work puts generated helpers in a private constant scope to avoid F12
collisions between qualified types with equal final names.

| Check, 200 rounds | Baseline | Patched | Expected |
| --- | ---: | ---: | --- |
| Active RPC retains its service guard | No | Yes | Yes |
| Anonymous guarded calls rejected | 0 / 200 | 200 / 200 | 200 / 200 |
| Authenticated guarded calls succeed | 200 / 200 | 200 / 200 | 200 / 200 |
| Explicitly public calls work without credentials | 200 / 200 | 200 / 200 | 200 / 200 |
| Guarded handler entries | 400 | 200 | Authenticated calls only |
| Security regression assertion | Fails | Passes | Passes |

The baseline failure was the specific policy mismatch; authenticated and public
controls passed. The comparison uses the same local fixture on both source
snapshots, with synthetic credentials, no network sockets, and no external
requests. It is an admission correctness check, not an RPC performance test.
Full commands, outputs, exit codes, and source fingerprints are in the
[before/after JSON](results/2026-10-15-grpc-policy-before-after.json). The
regressions are in [grpc_conditional_policy.rs](../crates/furnace-rs-grpc/tests/grpc_conditional_policy.rs)
and [grpc_conditional_attributes.rs](../crates/furnace-rs-grpc/tests/grpc_conditional_attributes.rs).

## Core errors and PostgreSQL connector

The connector review reproduced three issues and patched them:

| Finding | Reproduced behavior | Correction and boundary |
| --- | --- | --- |
| Core error source disclosure | Derived Debug recursively printed a retained native cause containing a database password. | Custom ordinary Debug shows authored diagnostics and source presence without rendering the cause. Typed source access remains available; authored messages and explicit source logging remain the application's responsibility. |
| Unrepresentable database deadlines | <code>u64::MAX</code> seconds or native <code>Duration::MAX</code> could overflow SQLx Instant arithmetic and panic before connecting. | Check typed settings and native options immediately before connecting, including lazy pools and options changed through <code>options_mut</code>; reject with a safe configuration error. This addresses configuration-controlled values, not request-controlled input, and does not impose an arbitrary upper limit. Callbacks that directly produce SQLx options remain responsible for them. |
| Connection trace credential disclosure | SeaORM's connection span formatted options containing the full URL, including its password. | Poll the connector's native establishment future under a scoped <code>NoSubscriber</code>, restoring the application's subscriber outside it. Handshake telemetry is suppressed; configured statement logging remains. Direct SeaORM calls and explicit option logging are outside this boundary. |

The stress evidence covers 2,200 assertions: source redaction (2 baseline
failures, then 200 checks with no failure), typed deadline rejection (8 baseline
failures, then 800 checks), native deadline rejection (2 baseline failures,
then 200 checks), tracing suppression/restoration (2 baseline failures, then
200 checks), and valid-configuration controls (8 baseline controls pass, then
800 checks pass). These are correctness counts, not performance measures.

The subsequent isolated PostgreSQL 16.15 validation passed four integration
tests for native CRUD and transaction behavior, invalid-credential redaction,
pool shutdown after stop failure, and readiness before HTTP binding followed
by graceful shutdown. Three database recovery workloads also passed against the
temporary server. Raw baseline, patched, and live-server records are linked in
the index above; the [PostgreSQL validation JSON](results/2026-10-02-infrastructure-postgres-validation.json)
records the environment and cleanup.

At the earlier verification point, the all-feature workspace suite passed 785
tests with 7 ignored; Clippy with warnings denied and formatting checks passed;
and the benchmark tool passed 42 tests with 3 PostgreSQL checks skipped. The
later live-server run passed all 34 benchmark Python tests without skips and
all 22 checks in the independent security smoke workload.

## Database, JWT, and inventory regressions

The inventory registry accepted a constructor output whose concrete type did
not match its descriptor. The fix rejects it during build with FURNACE004,
leaves the registry unchanged after invalid insertion, and permits a later
valid insertion. Native and configured zero idle/lifetime maintenance durations
were also accepted; the patch rejects zero before pool creation while retaining
valid durations and native None settings. The regression verifies rejection;
it does not measure the CPU impact of a zero maintenance interval.

The JWT algorithm-binding and PostgreSQL recovery controls already passed on
the baseline and stayed green after the patch: forged HS256 tokens using RSA
public-key bytes are rejected, valid RSA tokens still work, and native pools
recover from a terminated idle connection, acquisition timeout, and query
failure/cancellation. These are retained controls, not new JWT, retry, or query
replay implementations.

The recorded comparison confirmed three targeted baseline failures and 26
passing patched tests. PostgreSQL recovery ran in a temporary local cluster
using fixture-only credentials; no deployment database was used. The result
records commands, output, revisions, source fingerprints, and environment in
the [before/after JSON](results/2026-10-06-database-inventory-security-before-after.json).
Recovery coverage is limited to the named failures; it does not measure
prolonged network partitions, server restarts, or sustained-load capacity.

## HTTP runtime request deadlines

Before the patch, incomplete HTTP/1 headers and a handler's read of a stalled
request body could remain pending beyond a 12-second observation window. The
header fix is 7a19295; the body fix is 12b2ac6. Initial protocol detection and
requests have a ten-second deadline, HTTP/1 header parsing uses Hyper's timer,
and pending body reads get a separate ten-second idle deadline renewed by
frame progress. Successful handlers have no added execution deadline.

| Check | Before | Patched behavior |
| --- | --- | --- |
| Incomplete headers | Connection still open at 12 seconds | Closes within the observation window; follow-up health request returns 200 |
| Complete headers, stalled JSON body | No response at 12 seconds | HTTP 408 after the ten-second idle deadline; HTTP/1 connection closes and follow-up health request returns 200 |
| Stalled HTTP/2 body | Not measured in baseline | Safe per-request 408 without a forbidden Connection header; another request on the same connection returns 200 |
| Handler longer than ten seconds and graceful shutdown | Not measured in baseline | Handler completes and shutdown drains it |

The patched controls also preserve JSON success and errors, body-size limits,
HTTP/2 behavior, upgrades, bind failure reporting, and lifecycle ordering. A
timeout before response headers produces the fixed request_timeout error;
after response streaming begins it terminates the stream. Uploads that continue
making progress may run longer than ten seconds: this is an idle timeout, not a
total duration or minimum-rate limit.

The recorded patched suite passed 25 tests; each baseline reproduction failed
its specific stall assertion as expected. These single-request loopback checks
do not measure connection-flood behavior, resource exhaustion, server capacity,
or process-crash resistance. Full test output, fixture provenance, and source
fingerprints are in the [before/after JSON](results/2026-10-03-http-runtime-security-before-after.json).

## Run security regression comparisons

Commands below use local fixtures or synthetic credentials. The finding
summaries above state the main checks and limitations; raw result files retain
the exact commands and runner output.

```sh
python3 benchmark/tool/build_furnace.py --offline --example protected-route
python3 benchmark/tool/security.py --profile smoke \
  --output /tmp/furnace-security-smoke.json
python3 benchmark/tool/grpc_policy_security.py --rounds 200 \
  --output /tmp/furnace-grpc-policy-before-after.json
python3 benchmark/tool/database_inventory_security.py \
  --output /tmp/furnace-database-inventory-security.json
python3 benchmark/tool/http_runtime_security.py \
  --output /tmp/furnace-http-runtime-security.json
python3 benchmark/tool/infrastructure_security.py --profile stress \
  --output /tmp/furnace-infrastructure-security.json
python3 benchmark/tool/cli_security.py --profile stress \
  --output /tmp/furnace-cli-security.json
```

The database recovery and timeout comparisons use isolated local resources.
Do not point fault workloads at a production database.

## Security suite coverage

| Suite | Checks and evidence |
| --- | --- |
| Passport and HTTP auth | Signed and invalid JWT controls; Bearer syntax, token characters and padding; duplicate or combined credentials; generic 401 responses and connection recovery. The current focused finding and before/after counts are in [Bearer parsing](#bearer-credential-parsing-hardening). |
| gRPC policy admission | Runs guarded, authenticated, and explicitly public in-process calls against the baseline failure and patched method-gate behavior. See the [F01 summary](#f01-conditional-grpc-admission-policy). |
| Database, JWT, inventory, and infrastructure | Checks redacted error causes and URLs, representable timeout configuration, pool and startup failure behavior, JWT algorithm binding, and recovery controls. See the [database/inventory summary](#database-jwt-and-inventory-regressions) and [infrastructure summary](#core-errors-and-postgresql-connector). |
| HTTP runtime deadlines | Checks bounded header/body reads, timeout responses, and connection behavior against the historical implementation and patched source. See the [HTTP runtime summary](#http-runtime-request-deadlines). |
| Local CLI files and dependencies | Checks owner-only control directories under varied umasks and audits committed Cargo lockfiles against a recorded advisory database snapshot. See the [all-crate audit](ALL_CRATES_SECURITY_AUDIT.md). |

Raw JSON reports and transcripts are linked from the finding summaries and
index. Results describe the tested code and environment; they do not establish
exhaustive security coverage.

### Legacy HTTP security and availability workloads

The Python runner in `benchmark/tool/run.py` is retained for FURNACE-only
correctness and fault checks. It is separate from the current Rust performance
comparison. Cases include:

| Case | Stress profile | Check |
| --- | --- | --- |
| `validation` | 3,000 POSTs, 32 clients | Empty fields, malformed JSON, and oversized invalid bodies return validation errors. |
| `oversized` | 32 POSTs, 8 clients | 3 MiB JSON bodies receive a structured 413 response. |
| `oversized-reuse` | 8 POSTs, 4 clients | A 413 signals connection closure and the next request succeeds on a fresh connection. |
| `body-limit-boundary` | 96 POSTs and 96 follow-up GETs, 8 clients | Exact 2 MiB boundaries return the expected 422/413 responses and clients recover. |
| `aborted-upload` | 256 partial uploads, 256 logins, and 256 protected GETs, 32 clients | Closing an incomplete upload does not prevent later login or protected reads. |
| `jwt` | 6,000 GETs, 64 clients | Valid credentials work; missing and malformed tokens get a Bearer 401. |
| `database-failure` | One startup failure | An unavailable database prevents the listener from binding and does not print the URL password. |
| `database-connect-timeout` | One stalled handshake | Startup fails near the configured deadline without opening the HTTP listener or leaking credentials. |
| `database-query-timeout-recovery` | One query timeout and recovery | A PostgreSQL statement timeout returns a safe error; CRUD works after releasing the table lock. |
| `database-tcp-stall-recovery` | One stalled reply and recovery | Pool-acquire timeouts keep HTTP alive and CRUD resumes after the proxy recovers. |

The focused auth suite's `smoke`, `stress`, and `extended` profiles run 2, 200,
and 1,000 repetitions per probe. Database recovery cases need an isolated
PostgreSQL database and the posts schema. Never run fault workloads against
production data. For a quick HTTP-only pass:

```sh
python3 benchmark/tool/run.py --profile stress \
  --case hello --case connection-churn --case validation --case oversized \
  --case oversized-reuse --case body-limit-boundary --case aborted-upload \
  --case jwt --case database-failure --case database-connect-timeout \
  --output /tmp/furnace-http-correctness.json
```

Use `python3 benchmark/tool/run.py --help` for the legacy runner's full case
list. The current security tool commands are maintained above in this page.

## Bearer credential parsing hardening

The review found a permissive parsing boundary in
`crates/furnace-rs-common/src/passport/guard.rs`: `split_whitespace()` treated
tabs as valid separators between the Bearer scheme and token. A correctly
signed access JWT in `Authorization: Bearer<TAB>token` reached the protected
handler and returned 200.

[RFC 6750 section 2.1](https://www.rfc-editor.org/rfc/rfc6750.html#section-2.1)
requires one or more ASCII spaces between the scheme and token, with a defined
token alphabet and optional trailing `=` padding. Accepting a broader grammar
can create different credential interpretations between an application and
an upstream gateway or filter. This is a potential parser-differential issue;
the reproduction does not demonstrate an exploit against a gateway or access
without a valid JWT. No CVE or severity rating is assigned.

Passport now strips surrounding HTTP optional whitespace, requires ASCII
spaces inside the credentials, and checks the Bearer alphabet and padding
placement before invoking an authentication adapter. Scheme matching remains
case-insensitive, multiple spaces remain accepted, and duplicate Authorization
fields remain rejected. Native and generated guards share this parser.

Rust regressions check malformed credentials, token characters, padding,
duplicate headers, and accepted syntax. The route regression also checks that
rejected credentials never invoke the strategy or handler and that responses
contain only the generic unauthorized envelope and Bearer challenge.

## Measured reproduction

The same local release auth example was rebuilt before each implementation
check. Both runs used commit `97dad450d309866a0f1ea979def236acdb03f06b`
with working-tree test additions; the second build includes the parser patch.

| Smoke workload | Before patch | After patch |
| --- | ---: | ---: |
| HTTP responses | 92 | 92 |
| Expected 401 responses | 40 | 40 |
| Actual 401 responses | 34 | 40 |
| Actual 200 responses | 58 | 52 |
| Contract errors | 6 | 0 |

The six errors were two requests for each of `Bearer<TAB>token`,
`Bearer<SPACE><TAB>token`, and `Bearer<TAB><SPACE>token`. Each returned 200
before the patch and 401 afterwards. Valid credentials continued returning
the expected profile. The broader JWT probes test existing protections;
they are not additional newly discovered vulnerabilities.

Raw results are in `results/2026-10-02-security-before.json` and
`results/2026-10-02-security-after.json`. The before result came from the
initial benchmark draft; its `rejected_requests` field means the expected
rejection count, renamed to `expected_rejections` in the final tool. The final
tool also records the binary hash and strengthens the forged-signature,
algorithm, and oversized-token fixtures. Latency and throughput from this small correctness run
are not evidence of a performance change.

The stress result in `results/2026-10-02-security-stress.json` runs every
probe 200 times with 16 clients: 9,200 HTTP responses, 4,000 expected 401s,
5,200 successful controls/follow-ups, and zero contract errors.

## Re-run

```sh
python3 benchmark/tool/build_furnace.py --offline --example protected-route
python3 benchmark/tool/security.py --profile smoke --output /tmp/furnace-security-smoke.json
python3 benchmark/tool/security.py --profile stress --output /tmp/furnace-security-stress.json
cargo test --offline -p furnace-rs-common --all-features --test passport_bearer
python3 -m unittest discover -s benchmark/tool -p 'test_*.py'
```

The security tool specifically selects the staged local-source binary. Using
the legacy stress runner's previously built example binaries would not verify
this patch. Socket-based checks require permission to bind loopback sockets.

## Verification

- `cargo test --offline --workspace --all-features` passed 760 tests, including
  CLI consumers, request regressions, and documentation examples. Four
  PostgreSQL integration tests were ignored because no isolated database was configured.
- `cargo test --offline -p furnace-rs-common --all-features` passed all 312 tests.
- `cargo clippy --offline -p furnace-rs-common --all-features --all-targets -- -D warnings` passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- The benchmark Python suite ran 29 tests successfully, with three existing
  PostgreSQL integration checks skipped because no isolated database was configured.
- Final security smoke and stress workloads passed all response contracts.

The local-source auth Cargo lockfile is retained under
`targets/furnace-rs/locks/protected-route.lock` to pin the dependencies used
for these release builds.
