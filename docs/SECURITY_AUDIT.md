# furnace-rs 1.0.0 security audit

Status: **Released at 2026-10-3**. The audit changes
were committed before release preparation at `a847743`. This document brings
together the authentication, core/database, and all-crate reviews without
changing their historical evidence.

The current released version is **1.0.2**; see its [release notes](releases/1.0.2.md). This document preserves
the 1.0.0 audit snapshot; subsequent HTTP deadline, database maintenance, and
inventory output fixes are documented in the [1.0.1 release notes](releases/1.0.1.md).
The [extractor error-redaction report](security/2026-10-09-extractor-error-redaction.md)
covers the additional fix released in 1.0.2.

## Reviewed scope

All nine workspace crates were reviewed: facade, core, both procedural-macro
crates, common/HTTP/authentication, persistence, CLI, testing, and extra.
The [all-crate report](../benchmark/ALL_CRATES_SECURITY_AUDIT.md) records the per-crate
coverage, source inventory, threat boundaries, dependency scan and limitations.
The release candidate retains Rust edition 2024, MSRV 1.94 and SeaORM minimum
2.0.0; public APIs change from the MADS 0.x family as described in the
[migration guide](importance/furnace-rs-migration.md).

## Findings included in 1.0.0

| Finding | Reproduction and impact | Patch and evidence |
| --- | --- | --- |
| Permissive Bearer separators | Internal tabs with a valid signed access JWT reached a protected handler. Potential gateway/parser differential; no signature forgery or gateway exploit established. | Enforce ASCII-space Bearer grammar before strategy/handler execution; reject malformed/duplicate credentials. [Auth review](../benchmark/SECURITY.md), commit `6edc43f`. |
| Core error source disclosure | Derived `Debug` recursively rendered a retained native source containing a database password. | Custom source-free ordinary debug formatting; explicit typed source access retained. [Infrastructure review](../benchmark/INFRASTRUCTURE_SECURITY.md), commit `16413c6`. |
| Unrepresentable database deadlines | Configurable `u64::MAX`/native `Duration::MAX` caused a SQLx `Instant` overflow panic before connection. Requires control of config/options. | Check typed and native deadlines before connecting; safe typed rejection. [Infrastructure review](../benchmark/INFRASTRUCTURE_SECURITY.md), commit `16413c6`. |
| Native connection trace disclosure | SeaORM's connection span recorded complete options, including URL credentials, with tracing enabled. | Scope `NoSubscriber` to polls of the native establishment future; restore surrounding tracing. [Infrastructure review](../benchmark/INFRASTRUCTURE_SECURITY.md), commit `16413c6`. |
| Unix CLI control-directory access | Directory modes inherited umasks; other local users could read or, with group/world write access, tamper with transport/control contents. Executable replacement races were source-derived, not executed end to end. | Apply owner-only `0700` during directory creation; isolate umask tests in children. [All-crate review](../benchmark/ALL_CRATES_SECURITY_AUDIT.md), commit `a847743`. |

No CVE assignment or independent certification is claimed. These are targeted
confirmed implementation issues and explicitly qualified impact assessments.

## Recorded verification before release preparation

- Authentication stress: **9,200 HTTP responses**, zero contract errors;
  malformed cases return generic 401/Bearer responses and valid controls work.
- Core/database stress: **2,200 contract checks**, zero failures; tests cover
  source redaction, typed/native deadline rejection, connection tracing, and
  valid configuration controls.
- Unix CLI stress: **800 permission/access checks**, zero failures, across
  umasks `000`, `002`, `022`, `077`. Child completion evidence prevents a
  zero-test subprocess from appearing successful.
- Final all-crate workspace run: **769 unit/integration/doc tests plus one
  nested process fixture** passed; four real PostgreSQL tests were ignored in
  that run. The Python suite ran 38 tests: 35 passed, three live DB checks skipped.
- Separate real PostgreSQL 16.15 validation: **four database tests and all 34
  then-existing Python tests passed with no skips**, including all three live
  DB checks. The server required localhost TCP password authentication, used
  disposable databases, and was removed afterwards.
- All seven checked-in Cargo lockfiles had zero known vulnerability/advisory
  warning matches against the recorded RustSec snapshot. The workspace had 317
  locked dependencies; registry yanks were not checked.

These counts describe their recorded pre-release source snapshots, not newly
executed 1.0.0 checks. Current release validation is recorded separately in
[the readiness guide](releases/1.0.0.md) and its linked evidence.

Primary evidence: [auth stress](../benchmark/results/2026-10-02-security-stress.json),
[infrastructure stress](../benchmark/results/2026-10-02-infrastructure-security-stress.json),
[CLI stress](../benchmark/results/2026-10-02-cli-security-stress.json),
[PostgreSQL validation](../benchmark/results/2026-10-02-infrastructure-postgres-validation.json),
[all-crate verification](../benchmark/results/2026-10-02-all-crates-verification.json),
and [dependency audit](../benchmark/results/2026-10-02-all-crates-dependency-audit.json).

## Verification of the prepared 1.0.0 tree

On 2026-10-03, the candidate passed **770 workspace tests plus one nested fixture**
on both Rust 1.96.0 and the Rust 1.94.0 MSRV. Each workspace run ignored four
live database tests; a separate isolated PostgreSQL 16.15 server with SCRAM
password authentication passed all four and **all 38 Python tests with no skips**.
The server and its data directory were removed afterwards.

The candidate also passed all-target/all-feature Clippy and Rustdoc with warnings
denied, formatting, eight feature-boundary checks, all three locked examples,
package-content checks and creation of all nine 1.0.0 archives. SeaORM 2.0.0
compatibility passed 22 persistence tests in an isolated workspace copy.
Security smoke workloads passed 22 core/database checks and eight Unix CLI
checks. The eight-lockfile advisory scan found zero known vulnerabilities at
its recorded snapshot; registry yank status was excluded.

These new results supplement the preserved stress reports above. They do not
claim a fresh 1.0.0 HTTP stress run or completed remote platform/coverage gates.
The [release guide](releases/1.0.0.md#local-verification) links the five new
JSON evidence files, including source/configuration fingerprints and exact
local commands. Publication and published-dependency resolution remain pending.

## Limits and operational responsibilities

Connection-establishment telemetry and inline native callback traces are
suppressed to avoid credential-bearing spans; subsequent statement logging
retains native behavior. Authored diagnostic text and explicit source logging
remain application responsibilities. Arbitrary trusted SQLx pool callbacks can
override validated options after Furnace's check.

No independent cryptographic review, coverage-guided fuzzing, end-to-end
cross-user executable race, or exhaustive cancellation/resource-exhaustion
analysis was performed. Windows ACL behavior was not experimentally verified.
Pending handlers can delay native graceful shutdown; stock Passport peer-address
metadata may be absent. Test assertion helpers can print response secrets and
buffer unbounded in-process streams, so test code should use synthetic data.

The results do not establish universal production readiness. See the
[security policy](SECURITY.md) for reporting and the release guide for remote
platform, coverage, registry and publication gates.
