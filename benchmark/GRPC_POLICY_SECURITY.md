# F01 conditional gRPC policy regression

This defensive comparison verifies the finding against the recorded beta source
and checks the patch using the same local regression fixture. It sends no requests
to an external service and opens no network sockets. Credentials and payloads are
synthetic test data.

## Measured comparison

Baseline: `bd8ade6aa4759ad5072dd82a63b7c2006c31affc` (`2.0.0-beta.2`).
After: the patched working tree on `test/beta-v2-validation-test`.
Source and fixture fingerprints, commands, exit codes, and complete test output
are recorded in [the JSON evidence](results/2026-10-15-grpc-policy-before-after.json).
Both snapshots use the same 200-round fixture.
Production fingerprints cover the core, common, and gRPC crates and their macros,
workspace manifests/lockfile, protobuf test support, and the shared fixture.

| Check | Before | After | Required behavior |
| --- | ---: | ---: | --- |
| Active RPC retains service guard | false | true | true |
| Anonymous guarded calls rejected | 0 / 200 | 200 / 200 | 200 / 200 |
| Authenticated guarded calls succeed | 200 / 200 | 200 / 200 | 200 / 200 |
| Explicitly public calls succeed without credentials | 200 / 200 | 200 / 200 | 200 / 200 |
| Guarded handler entries | 400 | 200 | authenticated calls only |
| Security regression assertion | fails | passes | passes |

The baseline failure is the expected, measured policy mismatch. A compilation
failure, missing result marker, missing control case, or incomplete Rust test
cannot count as reproduced evidence. The comparison command succeeds only when
the baseline exhibits the precise original mismatch and the patch satisfies all
checks. Its `passed` field means the comparison was verified; the baseline's
`security_contract_passed` remains false.

## Cause and patch

The service macro applied a method's `cfg` condition to callable-method metadata
but emitted its policy override unconditionally. A disabled public implementation
could therefore change the policy used for the active conditional implementation.

The patch applies identical conditional gates to the method list and policy
overrides, including gates nested in `cfg_attr`. Policy declarations inside
`cfg_attr` are explicitly rejected: place `grpc_method` directly on the method and
condition the method itself. Active duplicate overrides remain rejected during
scope validation. Generated helper definitions also live in a private constant
scope, fixing F12's collisions between qualified types with equal final names.

Additional regressions cover nested conditional attributes, disabled references
to absent guard types, active public replacements, and distinct qualified service
types. The comparison fixture lives in
[grpc_conditional_policy.rs](../crates/furnace-rs-grpc/tests/grpc_conditional_policy.rs).

## Reproduce the evidence

```sh
python3 -m unittest discover -s benchmark/tool -p test_grpc_policy_security.py
python3 benchmark/tool/grpc_policy_security.py --rounds 200 \
  --output /tmp/furnace-grpc-policy-before-after.json
```

Requirements: Python 3.12+, Git baseline history, Rust, and cached Cargo
dependencies. The runner creates a temporary archive of the baseline, adds the
current fixture, and leaves its production source unchanged. It compares that
archive with current source, including uncommitted patches, using locked offline
Cargo tests. The baseline build cache is retained under `target/grpc-policy-before`.

This is an admission correctness workload. Build-inclusive durations in the JSON
are not RPC latency or throughput measurements. The evidence demonstrates the
specific conditional-policy failure and its correction; it makes no claim of
exhaustive protocol or security coverage.
