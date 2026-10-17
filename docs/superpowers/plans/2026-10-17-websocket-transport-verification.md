# WebSocket transport verification

Implementation branch: `feat/websocket-transport`.
Implementation commits: `ce1d387` through `45dcb22`.

The implementation covers all twelve planned areas: configuration and protocol,
registration and cauldron scope, supervised owned sockets, macros, managed
execution, Passport admission, local rooms, distributed live rooms, replay,
TLS and shared lifecycle, static CLI inspection, and consumer/release integration.
See the [transport guide](../../socket.md) and
[standalone example](../../../example/socket-transport/README.md).

## Verification

Workspace validation is complete. The workspace run passed the facade and CLI
suites through the release policy tests, where it found stale package expectations
and missing socket version records. Those failures were fixed and all 25 release
tests passed. Focused runs then completed every remaining workspace package and
CLI suite, without repeating already-passing tests. Existing explicitly ignored
tests remain ignored.

The following checks passed:

- Socket package with all features: **39 tests**, including actual WebSocket,
  Redis, replay, authentication, TLS, queue and cancellation tests.
- Socket package without default features; separate JWT, TLS and Redis checks;
  facade socket-only and socket/JWT/TLS/Redis checks.
- Workspace Clippy, all targets and features, with warnings denied.
- Workspace documentation tests and warning-denying API documentation build.
- Rust **1.94** workspace check and independent example check, all features.
- Independent example, all features: **2 integration tests**, including two-server
  cursor replay.
- Package archives for the workspace; package content validation for **14 crates**.
- Release automation: **25 tests**, including both new crate version records,
  dependency publication order, and nested lockfile updates.
- Formatting and whitespace checks.

## Independent review

One fresh-context branch review found five Important issues. All were addressed
in one fix pass; there were no Critical findings, deferred minors, or declined
judgments. The regressions were observed failing before their fixes:

| Finding | Regression evidence |
| --- | --- |
| Raw errors injected JSON | `raw_handler_errors_close_without_injecting_json`, `raw_generation_retirement_never_injects_json` |
| Concurrent leave raced with replay join | `leave_waits_for_inflight_replay_join_and_stops_its_reader` |
| Rejoining interleaved old/new readers | `replay_rejoin_stops_old_reader_before_catchup` |
| Redis recovery retained a lost generation | `restoration_retires_generation_lost_between_info_and_renewal`, `new_generation_join_does_not_inherit_stale_members` |
| Blocked connect hooks stalled controls | `blocked_connect_hook_keeps_controls_responsive_and_cancels_on_peer_close` |

Room race regressions use an actual isolated Redis server and test-only
notification barriers at the relevant transitions. Raw/connect regressions use
actual WebSocket connections. Tests do not claim exhaustive network-fault
injection for every possible Redis reply-loss timing.

## Recorded implementation rulings

1. Reserved application event names are validated during registration rather
   than envelope parsing, because built-in events must parse. Cost if wrong:
   reserved registrations could escape validation; registration tests cover it.
2. TLS and mixed-listener lifecycle checks were performed after the final server
   existed. Cost: those edge cases stayed unverified during the earlier task.
3. Typed gateway adapters and dispatch were implemented together because their
   generated interfaces depend on each other. Cost: a broader review boundary.
4. Redis operations share one atomic Lua script so lifetime and retention rules
   remain together. Cost: the script requires review across multiple operations.
5. Redis **6.2+** is required for native exact age trimming with `XTRIM MINID`.
   Cost: deployments on older Redis versions need an upgrade.

The assigned branch remains local; no merge, push, or publication was requested.
