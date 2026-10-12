# All-crate security audit — 2–3 October 2026

Reviewed all nine workspace crates at `16413c65`, followed by a targeted CLI
patch in the working tree. One new issue was confirmed and patched: Unix CLI
inspection and development control directories inherited permissive umasks.
No additional production vulnerability was confirmed in the other eight crates.

## Coverage

| Crate | Source modules and security boundaries reviewed |
| --- | --- |
| `furnace-rs-core` | Configuration sources/interpolation, secret wrappers and error formatting; catalog/registry, injectors, rooted scope/import/export rules, graph validation/planning, preflight and auto-configuration; construction and lifecycle rollback/shutdown. |
| `furnace-rs-core-macros` | Configuration parsing/validation generation, secret fields, managed construction, cauldron/main/test expansion, dependency crate resolution. |
| `furnace-rs-common` | JWT algorithms/keys/claims/size limits, Passport credential parsing and policies, cookies, CORS, extraction/validation/rejections, routing/seals/scopes, HTTP configuration/lifecycle, inspection publication, logging. |
| `furnace-rs-common-macros` | Generated routes and guard ordering, explicit skipped guards, Passport adapters/principals, input validation, Rust token generation. |
| `furnace-rs-persistence` | Typed/native options, checked timeouts, PostgreSQL scheme selection, connection tracing, safe retained errors, factory, readiness and native pool shutdown. |
| `furnace-rs-cli` | Command/project selection, Cargo/process arguments, inspection transport, dev shutdown/shadow executable, state/watch loop, scaffolding names/paths/publication, reports and error formatting. |
| `furnace-rs-testing` | Fixture construction/overrides/lifecycle, HTTP dispatch, response buffering/assertions, test failure diagnostics. |
| `furnace-rs` | Public reexports, feature gates, runtime/bootstrap and declaration integration. |
| `furnace-rs-extra` | Current core reexport surface; no independent runtime implementation. |

The scope includes source review, inspection of security-related tests, targeted
reproduction, independent review of the patch, workspace tests and lint checks,
and RustSec scans of every checked-in Cargo lockfile. The machine-readable
verification result includes a source inventory and hash for each crate.

## Confirmed issue: CLI transport directories accessible to other local users

Both `inspection.rs` and `process.rs` used `tempfile::tempdir()` for directories
described or relied upon as private. Locked `tempfile` 3.27.0 creates directories
with `0777 & !umask`; its documentation explicitly distinguishes this from
private temporary files. An isolated Rust reproduction and the new regression
confirmed:

| Child umask | Previous directory permissions | Patched permissions |
| --- | --- | --- |
| `000` | `0777` | `0700` |
| `002` | `0775` | `0700` |
| `022` | `0755` | `0700` |
| `077` | `0700` | `0700` |

Under `002`, another user in the writable group can modify the directory's
contents; under `000`, any local user can. This permits shutdown-marker creation
and inspection report/token tampering. The shadow executable copy and execution
also occur inside this directory: writable access creates a symlink/replacement
race with potential victim-user file corruption or execution replacement. Those
race outcomes were identified from source, not reproduced end to end. Under
`022`, files are observable by other users, but directory write attacks are not
enabled. This is a conditional local filesystem issue, not a remote HTTP flaw.

The shared crate-private constructor now requests `0700` through
`tempfile::Builder::permissions` on Unix. The dependency applies that mode on
directory creation, before control files or executables are written. There is
no post-creation chmod window and no process-global umask mutation. Both real
transport paths use the constructor and retain their existing ownership/cleanup.
Non-Unix behavior is preserved; Windows ACLs were not experimentally verified.

The regression runs checks in independent child processes under all four
umasks. It checks actual directory permissions and owner read/write access. A
completion marker emitted after the child assertions prevents an empty libtest
selection from being counted as a successful check; a separate regression
exercises that failure mode. Independent review confirmed the final fix.

Before the permissions patch, three of four checks failed. After the patch, the
stress profile passed 800 checks in 200 rounds with zero failures. This measures
filesystem correctness, not HTTP throughput or actual cross-user race wins.

```sh
CARGO_TARGET_DIR=/tmp/furnace-audit-target \
  python3 benchmark/tool/cli_security.py --profile stress \
  --output /tmp/furnace-cli-security.json
```

Results: [before](results/2026-10-02-cli-security-before.json) and
[after](results/2026-10-02-cli-security-stress.json). The runner rejects missing
or duplicate results, incomplete counts, reported failures and nonzero Cargo
exits. Its elapsed time includes Cargo compilation; it is not a performance
threshold.

## Dependencies

A freshly fetched [RustSec advisory database](https://github.com/rustsec/advisory-db)
at `117edb3bed98e9be112f277b7615eea3252e7c43` contained 1,280 advisories.
`cargo audit` reported zero known vulnerabilities and no advisory warnings in
the workspace's 317 locked dependencies. All seven checked-in lockfiles passed:
workspace, three examples, two CLI fixtures, and the Axum benchmark target.
The audit explicitly used `--no-yanked`; registry yank status was not checked.

Full machine-readable results are in
[the dependency audit](results/2026-10-02-all-crates-dependency-audit.json).
This establishes no matches against that advisory snapshot, not that every
dependency is free of undisclosed vulnerabilities.

## Verification

The [final verification result](results/2026-10-02-all-crates-verification.json)
records commands, counts, source fingerprints,
and cleanup status. The initial workspace run caught a package-content mismatch
because the newly added CLI source file was not yet in the tracked payload.
Including that file in the Git index resolved the mismatch; the final workspace
run passed the package-content and release checks.

The all-feature workspace run passed 769 unit/integration/doc tests and one
nested process fixture. Four PostgreSQL tests remained explicitly ignored in
this run. The unchanged core/persistence sources retain the earlier real-server
validation: four PostgreSQL 16.15 integration tests and all three live benchmark
database checks passed; see [that result](results/2026-10-02-infrastructure-postgres-validation.json).
It is previous evidence, not a new database run in this audit.

Workspace Clippy passed for all targets/features with warnings denied. Workspace
formatting and diff whitespace checks passed. The new permission regression and
benchmark integrity tests also passed. The Python suite ran 38 tests: 35 passed
and three live PostgreSQL checks were skipped, with their prior validation
referenced above. Primary build artifacts and Python example
builds were directed to `/tmp`; package verification can create repository package
artifacts, which are removed after checks finish.

## Observations and limits

These are separate hardening/operational observations, not additional confirmed
production vulnerabilities:

- Stock HTTP serving does not populate `ConnectInfo`, so Passport remote-address
  metadata may be absent. Policies must handle its optional value.
- Native graceful HTTP shutdown has no drain deadline. Pending application
  handlers can delay shutdown; no bounded-drain contract is currently promised.
- Asymmetric key files are read during validation and again during service
  construction. Replacement requires control of trusted deployment files.
- Test helpers intentionally include response bodies/headers and explicit error
  sources in assertion/shutdown diagnostics, and buffer responses without a
  practical size bound. Tests using live secrets or endless upstream streams can
  disclose data in CI logs or exhaust the runner; these are in-process test
  helpers, not production request handlers.
- Scaffold staging assumes a trusted invocation directory; inspection consumes
  reports from an application whose build/code already executes locally. Process
  termination manages immediate children rather than complete descendant trees.

No coverage-guided fuzzing, independent cryptographic review, Windows runtime
testing, load/cancellation exhaustion testing, or end-to-end cross-user race
attack was performed. The audit does not claim exhaustive vulnerability absence.
Existing Bearer syntax, core source-redaction, timeout and database tracing fixes
remain covered by their prior regressions and this workspace test run.
