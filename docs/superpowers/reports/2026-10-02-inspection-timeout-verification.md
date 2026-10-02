# Inspection protocol test timeout verification

Date: 2026-10-02
Branch: `refactor/furnance-injector`
Baseline: `c02c714`
Status: Fixed and verified locally.

The success protocol test used 100 ms for both acknowledgement and report deadlines. Real child startup can exceed this under CI or coverage load. A new platform-neutral delayed_success fixture waits 250 ms before acknowledgement and another 250 ms before its valid report. Its regression test failed with the same FURNACE203 acknowledgement timeout before the fix, then passed.

Protocol tests now use the normal 2-second handshake and 10-second report budgets with their existing 5 ms polling interval. The intentional missing-report test overrides only its report deadline to 100 ms. Negative tests assert the specific protocol error message as well as FURNACE203, preventing startup timeout from masquerading as rejection of a bad token, version, or JSON report. Production timeout constants retain their existing values.

Verification:

- Rust 1.99 focused inspection tests: seven pass, including both immediate and delayed success.
- Full Rust 1.99 workspace coverage gate: exit zero, 89.10% line coverage against the required 85%; 724 passing results across 129 summaries, including nested runs. All 71 CLI library tests pass. Four database tests are intentionally ignored in the general suite; no persistence code changed.
- Rust 1.94 focused inspection tests: seven pass.
- Strict workspace Clippy, formatting, diff checks, and all nine package payload checks: exit zero.

The coverage command was `cargo +1.99.0 llvm-cov --locked --offline --workspace --all-features --ignore-filename-regex '(^|/)tests/ui/' --fail-under-lines 85`. The MSRV command was `cargo +1.94.0 test --locked --offline -p furnace-rs-cli --lib inspection::tests`.

Local evidence: `/tmp/furnace-inspection-delay-red.log`, `/tmp/furnace-inspection-delay-green.log`, `/tmp/furnace-inspection-coverage-final.log`, `/tmp/furnace-inspection-msrv-final.log`, `/tmp/furnace-inspection-clippy-final.log`, and `/tmp/furnace-inspection-packages.log`. This task does not push or rerun remote CI.
