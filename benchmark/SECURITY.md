# Security findings, fixes, and evidence

This page is the security index for the benchmark and review records. It
collects confirmed findings, the corresponding fixes, and links to measured
reproductions, regression tests, and raw evidence. Each detailed report states
the limits of what its checks establish; passing a workload is not a claim that
the framework has no other vulnerabilities.

## Finding and evidence index

| Area | Fix or result | Detailed evidence |
| --- | --- | --- |
| Bearer credential parsing | Require the RFC 6750 ASCII-space grammar and validate token characters and padding before authentication. The before/after reproduction is below. | [This report](#bearer-credential-parsing-hardening), [before/after JSON](results/2026-10-02-security-before.json), [patched JSON](results/2026-10-02-security-after.json) |
| Conditional gRPC admission policy (F01) | Apply the same compile-time conditions to active methods and their policy overrides; reject unsupported conditional policy declarations. | [F01 report](GRPC_POLICY_SECURITY.md), [before/after JSON](results/2026-10-15-grpc-policy-before-after.json), [regression fixture](../crates/furnace-rs-grpc/tests/grpc_conditional_policy.rs) |
| Core errors and PostgreSQL connector | Redact retained causes in ordinary error formatting; reject invalid timeout bounds and suppress credential-bearing connection tracing. | [Infrastructure report](INFRASTRUCTURE_SECURITY.md), [core/timeout baseline](results/2026-10-02-infrastructure-core-and-timeouts-before.json), [tracing baseline](results/2026-10-02-infrastructure-tracing-before.json), [patched evidence](results/2026-10-02-infrastructure-security-after.json), [live PostgreSQL validation](results/2026-10-02-infrastructure-postgres-validation.json) |
| Database, JWT, and inventory regressions | Preserve redacted database failures, JWT algorithm binding, and safe catalog construction while recovering from database faults. | [Database and inventory report](DATABASE_INVENTORY_SECURITY.md), [before/after JSON](results/2026-10-06-database-inventory-security-before-after.json) |
| HTTP request deadlines | Bound incomplete headers and stalled body reads while preserving safe timeout responses and connection behavior. | [HTTP runtime report](HTTP_RUNTIME_SECURITY.md), [before/after JSON](results/2026-10-03-http-runtime-security-before-after.json) |
| Local CLI control-directory permissions | Create sensitive Unix control directories with owner-only `0700` permissions at creation. The test checks multiple umasks in child processes. | [All-crate report](ALL_CRATES_SECURITY_AUDIT.md), [before](results/2026-10-02-cli-security-before.json), [patched stress run](results/2026-10-02-cli-security-stress.json) |
| Cookie, extractor, peer-address, and native pool checks | Record focused fixes and their test evidence in the repository security review. | [Security audit index](../docs/SECURITY_AUDIT.md), [cookie validation](../docs/security/2026-10-10-cookie-attribute-injection.md), [extractor redaction](../docs/security/2026-10-09-extractor-error-redaction.md), [peer address](../docs/security/2026-10-10-http-peer-address.md), [pool capacity](../docs/security/2026-10-10-native-pool-capacity.md), [scaffold privacy](../docs/security/2026-10-10-scaffold-directory-privacy.md) |
| Dependency audit | Check every committed lockfile against the recorded RustSec advisory snapshot; the report documents snapshot and yank-status limits. | [All-crate report](ALL_CRATES_SECURITY_AUDIT.md), [dependency audit JSON](results/2026-10-02-all-crates-dependency-audit.json), [verification JSON](results/2026-10-02-all-crates-verification.json) |

The repository-wide reporting policy and current scope are in
[`docs/SECURITY.md`](../docs/SECURITY.md). These benchmark records preserve
reproductions and results for specific fixes.

## Run security regression comparisons

Commands below use local fixtures or synthetic credentials. Each detailed
report documents its prerequisites, exact checks, and limitations.

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

The database recovery and timeout comparisons use isolated local resources;
their detailed reports explain setup and cleanup. Do not point fault workloads
at a production database.

## Security suite coverage

| Suite | Checks and evidence |
| --- | --- |
| Passport and HTTP auth | Signed and invalid JWT controls; Bearer syntax, token characters and padding; duplicate or combined credentials; generic 401 responses and connection recovery. The current focused finding and before/after counts are in [Bearer parsing](#bearer-credential-parsing-hardening). |
| gRPC policy admission | Runs guarded, authenticated, and explicitly public in-process calls against the baseline failure and patched method-gate behavior. See the [F01 comparison](GRPC_POLICY_SECURITY.md). |
| Database, JWT, inventory, and infrastructure | Checks redacted error causes and URLs, representable timeout configuration, pool and startup failure behavior, JWT algorithm binding, and recovery controls. See the [database/inventory report](DATABASE_INVENTORY_SECURITY.md) and [infrastructure report](INFRASTRUCTURE_SECURITY.md). |
| HTTP runtime deadlines | Checks bounded header/body reads, timeout responses, and connection behavior against the historical implementation and patched source. See the [HTTP runtime report](HTTP_RUNTIME_SECURITY.md). |
| Local CLI files and dependencies | Checks owner-only control directories under varied umasks and audits committed Cargo lockfiles against a recorded advisory database snapshot. See the [all-crate audit](ALL_CRATES_SECURITY_AUDIT.md). |

Raw JSON reports and transcripts are linked from each detailed record. The
results describe the tested code and environment; they do not establish
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
list. The current security tool commands are maintained above and in each
detailed report.

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
