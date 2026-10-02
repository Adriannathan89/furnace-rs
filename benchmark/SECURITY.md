# Bearer credential parsing hardening

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
