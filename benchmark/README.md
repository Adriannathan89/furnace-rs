# Benchmark documentation

This directory contains repeatable performance measurements, security and
availability regressions, and their machine-readable evidence. Each report
records its date, source version, setup, results, and limits.

## Performance

[PERFORMANCE.md](PERFORMANCE.md) is the central performance report. It includes
the latest Rust reqwest comparison, schema 1 and schema 2 results, reproduction
guidance, and links to historical comparisons. For CLI setup and options, see
the [furnace-bench guide](runner/README.md).

## Security and availability

[SECURITY.md](SECURITY.md) indexes security findings, fixes, reproductions, and
raw evidence. Focused reports cover gRPC policy admission, Bearer parsing,
database and JWT behavior, HTTP timeouts, infrastructure errors, and the
all-crate review. General performance numbers do not substitute for these
correctness checks.

## Tools and evidence

- `runner/` is the standalone Rust performance client and its CLI.
- `targets/` contains the FURNACE, Axum, and Fiber benchmark applications.
- `tool/` contains security regression tools and retained Python runners for
  reproducing historical measurements.
- `results/` stores the raw JSON and transcripts linked by the reports.

Historical performance documents remain available as detailed archives:
[COMPARISON.md](COMPARISON.md) and [REPORT.md](REPORT.md). Their results use
older application versions and a Python HTTP client; the reports preserve their
original methodology and should not be compared directly with the current
reqwest measurements.
