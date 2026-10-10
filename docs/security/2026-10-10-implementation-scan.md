# Furnace implementation scan, 2026-10-10

Baseline: `bc16a4fc20a7a03ff13cda98de93121fbd2a8541` (1.0.2 source).
Branch: `feat/security-improve`. This is a source review with targeted executed
regressions, not a guarantee that every vulnerability has been found.

## Coverage and method

Reviewed all 130 Rust files under `crates`, excluding separate test directories
and build outputs, and including the persistence runtime example. The
[inventory](evidence/2026-10-10-source-inventory.json) records each baseline
path, byte count, and SHA-256 digest.

| Area | Rust files | Reviewed behavior |
| --- | ---: | --- |
| CLI | 22 | Arguments, subprocesses, inspection transport, output, watching, scaffold publication |
| Common | 40 | HTTP protocols and bodies, extractors, errors, validation, cookies, JWT, Passport, guards, scope, CORS, logging |
| Common macros | 14 | Routes, guards, strategies, extraction, generated metadata |
| Core | 31 | Registration, dependency construction, scoped resolution, inventory, graph validation, lifecycle |
| Core macros | 8 | Provider/controller registration and conditional fields |
| Persistence | 8 | Typed/native configuration, connection creation, errors, factories, lifecycle, example |
| Testing | 5 | Application harnesses and HTTP test utilities |
| Facade / extra | 2 | Public exports and utilities |

Also inspected six embedded CLI templates, relevant manifests, documentation,
and existing regression tests. Release and CI scripts received supplementary
inspection; they are outside the complete Rust-file inventory claim.

Traced input to its validation and sinks, checked error disclosure, authorization
and type boundaries, reviewed panic and resource paths, and examined generated
code. For each bounded patch below, ran a new regression against the unpatched
implementation to establish failure, then against the patched implementation.

## Patched findings

| Finding | Demonstrated failure and limits | Report |
| --- | --- | --- |
| Cookie attribute injection | Accepted Path/Domain strings alter serialized policy; requires application-supplied untrusted attributes; no browser exploit measured | [Writeup](2026-10-10-cookie-attribute-injection.md) |
| Scaffold directory privacy | Permissive umasks create group/world-writable staging; cross-user tampering requires a shared traversable parent; no different-UID exploit executed | [Writeup](2026-10-10-scaffold-directory-privacy.md) |
| Native zero pool capacity | Database-free dependency panic; requires trusted native configuration control | [Writeup](2026-10-10-native-pool-capacity.md) |
| Missing socket peer metadata | Native extractors return HTTP 500; Passport sees absent metadata; no application authorization bypass established | [Writeup](2026-10-10-http-peer-address.md) |

## Remaining functional findings

- Conditional controller fields: `#[cfg(any())] disabled: MissingDependency`
  disappears from the inner struct while generated dependencies and initializer
  fields still reference it. A real authored `#[controller]` probe failed with
  E0425/E0560 for both `cfg` and `cfg_attr`; a unit-controller control compiled.
  This needs a separate design for conditional dependency shapes, so this
  security patch batch leaves it unchanged. No security impact was demonstrated.
  [Probe source](evidence/2026-10-10-controller-cfg-probe.rs),
  [failure](evidence/2026-10-10-controller-cfg-failure.txt),
  [control](evidence/2026-10-10-controller-cfg-control.txt).
- Cookie name round trips: response encoding accepts names containing spaces,
  but request parsing decodes the name and rejects the space. Returning such a
  cookie can cause request rejection. This requires an application choosing
  such a name; changing the existing name policy is outside this batch.

## Checked hypotheses and limits

No signature or default authorization bypass was reproduced. Reviewed JWT
algorithm/key binding, access/refresh separation, duplicate credentials,
fail-closed guard composition, missing scope extensions, and response redaction.
Checked provider constructor type identities and graph invariants. CLI argument
execution uses separate arguments; existing-destination and symlink publication
checks retain their protections. Existing private inspection transports were
distinguished from the scaffold directory defect.

Rejected or unproven hypotheses are not reported as vulnerabilities: conditional
Passport strategy metadata (the real compiled fixture
[passes before patching](evidence/2026-10-10-strategy-baseline-pass.txt)), empty
HTTP/2 data-frame timeout evasion, native minimum pool sizes exceeding maximum
(upstream clamps them), enormous pool allocation, watcher flooding, and hostile
output from the application explicitly selected for development execution.
Rust's conditional item processing matters: expansion-only probes do not
establish reachability. The controller-field hypothesis was checked separately
through a real authored invocation before recording it above.

Tests run on Linux. There was no live database, cross-platform runtime test,
different-UID attack, browser-cookie experiment, fuzzing campaign, or new
dependency advisory audit. Peer addresses identify the immediate TCP peer;
trusted proxy interpretation remains an application concern.

## Verification

Targeted before/after logs are linked from each writeup. Final verification:

```sh
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --locked --workspace --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
```

- Workspace tests: exit 0; 144 harness summaries total 808 passed, zero failed,
  seven ignored PostgreSQL tests. This aggregate includes nested harnesses;
  it is not a count of distinct test cases. [Complete log](evidence/2026-10-10-workspace-tests.txt).
- Clippy with warnings denied: exit 0. [Log](evidence/2026-10-10-clippy.txt).
- Formatting and whitespace checks: exit 0.
- Independent review: no important or critical issues in the four confirmed
  patches. Its claim corrections were applied: the strategy hypothesis was
  withdrawn, and controller fields were verified through a real invocation.

The first workspace attempt hit the tracked-file archive policy because the
new compiler characterization fixture was untracked. Staging that fixture
resolved the mismatch; the complete rerun above passed. No policy was relaxed.
Builds disable debug information and incremental compilation to reduce local
disk use; this changes build artifacts, not feature coverage.
