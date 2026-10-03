# HTTP runtime timeout hardening: before and after

The blue-team review found two connection-retention risks in the running HTTP
server. Incomplete headers had no active Furnace deadline. After complete
headers arrived, a handler reading an incomplete body could also wait without
a body-read deadline. These waits can retain sockets and request tasks.

The header fix is commit `7a19295`; the body fix is commit `12b2ac6`. Both were
verified with isolated loopback regression tests. No connection flood, file
descriptor exhaustion, memory exhaustion, or process crash was measured.

## Measured comparison

Recorded on 2026-10-03 at 14:26:32 UTC, using locked offline Cargo debug builds
with all `furnace-rs-common` features enabled. Baselines were temporary Git
archives with the current regression fixture added to their test module;
their production implementations were unchanged. The working branch was not
reset or switched.

| Contract | Before | After `12b2ac6` |
| --- | --- | --- |
| Incomplete HTTP/1 headers | Still open after the 12-second observation window on `af49c24`; regression failed as expected | Connection closes within the observation window; configured deadline is 10 seconds; follow-up health returns 200 |
| Complete headers, incomplete JSON body | No completed response after 12 seconds on `7a19295`; regression failed as expected | HTTP 408 with connection closure after the configured 10-second idle deadline; follow-up health returns 200 |
| Idle sockets and partial protocol prefaces | Not separately measured in this baseline run | All three fixtures expire within 12 seconds |
| Stalled HTTP/2 body | Not measured in the baseline | Safe HTTP 408 without a forbidden `Connection` header; another request on the same HTTP/2 connection returns 200 |
| Successful JSON / malformed JSON / custom 8-byte body limit | Not measured in the baseline | Expected 200 / 400 / 413 responses preserved |
| Body frames making progress | Not measured in the baseline | Frames renew the idle deadline; application pauses do not expire ready frames |
| Handler execution and shutdown | Not measured in the baseline | A handler running beyond 10 seconds completes successfully; graceful shutdown drains it |

The baseline health assertions completed before each expected timeout failure.
The patched suite also covers HTTP/2 requests, lifecycle ordering, bind failures,
CORS preflight, and structured shutdown errors.

| Measurement stage | Revision | Tests passed | Tests failed | Cargo exit | Rust test duration | Including compilation |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Header reproduction | `af49c24` | 0 | 1 expected | 101 | 12.00 s | 34.681 s |
| Body reproduction, header fix already present | `7a19295` | 0 | 1 expected | 101 | 12.01 s | 38.279 s |
| Patched runtime and controls | `12b2ac6` | 25 | 0 | 0 | 11.01 s | 11.335 s |

The comparison runner returned zero: both specific baseline failures were
confirmed and all required patched controls passed. Each reproduction used
one incomplete request and a normal health follow-up. Patched tests run in
parallel, so their total duration is not a per-request latency measurement.
Different compilation caches also make the build-inclusive times unsuitable
for performance comparisons.

Raw commands, full test output, revisions, source fingerprints, fixture
provenance, and runner fingerprint are retained in
[the JSON result](results/2026-10-03-http-runtime-security-before-after.json).

## Behavior and scope

Initial requests, including automatic protocol detection, have a ten-second
deadline. HTTP/1 header parsing also uses Hyper's timer. A pending body read has
a separate ten-second idle deadline that renews on frame progress. Successful
handlers have no new execution deadline. Existing body limits remain in place.

Body timeouts produce the fixed JSON error `request_timeout` and HTTP 408 when
the handler has not sent response headers. The affected HTTP/1 connection
closes; HTTP/2 handles the timeout per request. If a response is already
streaming the request body, a later timeout terminates that stream instead of
changing its status. An upload continuing to make progress can run beyond ten
seconds; the body policy does not impose a total upload duration or minimum rate.

These results confirm bounded waits for the tested stalls. They do not measure
server capacity under sustained traffic or establish a process-crash fix.

## Re-run

Requires Python 3.12 or newer, cached Rust dependencies, the baseline commits
in local Git history, and permission to bind loopback sockets. No database or
deployment credentials are needed.

```sh
python3 benchmark/tool/http_runtime_security.py \
  --output /tmp/furnace-http-runtime-security.json

cargo test --offline --locked -p furnace-rs-common --all-features --lib server::
```

The runner rejects missing test results, unexpected baseline failures, successful
baseline tests, missing patched controls, and nonzero patched Cargo exits. Its
default baseline revisions are `af49c24` for headers and `7a19295` for bodies;
the after measurement uses the current checkout and records its HEAD and source
fingerprint. Temporary snapshots are removed on completion.
