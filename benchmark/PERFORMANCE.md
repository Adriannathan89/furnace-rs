# Performance benchmarks

This is the central index for FURNACE performance measurements. The current
full-application comparison is dated 2026-10-16; older measurements remain
linked below with their original versions, clients, and limitations.

## Current comparison — 2026-10-16

This run compares the complete FURNACE 2.0.0-beta.3, Axum 0.8.9, and Fiber 2.52.15 example applications. The standalone Rust runner sends all measured HTTP traffic with reqwest 0.13.4.

## Workloads and setup

The benchmark includes Hello World, posts CRUD, and auth. Posts uses the explicit SeaORM repository in [`example/posts-crud`](../example/posts-crud/src/post/repository.rs), which implements create, list, find, update, and delete directly. It does not use the generated entity CRUD contract from the separate [`example/posts-crud-storage`](../example/posts-crud-storage) example. The repository's `#[storage]` marker only registers the injected connection provider.

The FURNACE Hello World configuration has no debug-mode setting. The benchmark also runs FURNACE in release mode. Request-path logs were removed from the FURNACE auth example and the corresponding Axum and Fiber handlers. A separate auth smoke check passed all expected responses (401, 401, 200, 200) and captured no request-log lines for any framework: [request-logging-check.json](results/2026-10-16-reqwest/request-logging-check.json).

The host is Linux on an AMD Ryzen 5 6600H (6 physical cores, 12 logical CPUs). The reqwest client ran on CPUs 0, 2, 4, and 6; each application server ran on CPU 8; isolated PostgreSQL 16.15 ran on CPU 10. SMT siblings were not used. Fiber used `GOMAXPROCS=1`; Tokio worker overrides were unset. Each app had its own database with the same posts migration and a ten-connection pool. All three posts tables were empty after the benchmark.

FURNACE and Axum were built with Rust 1.99.0; Fiber used Go 1.22.2. CPU frequency and turbo were not pinned. These numbers compare the full example applications and their database/JWT implementations under this setup, rather than isolating framework overhead.

## Load shapes

Schema 1 uses a fixed 64 active workers. Schema 2 ramps each workload through 128 and 256 workers. It stops after two successive doublings yield less than 5% median throughput gain for every framework. No latency target was provided, so throughput plateau is the capacity signal. It is not the maximum number of sockets or real-world users an application can accept.

| Schema | Scenario | Operations per framework/run | Workers | Runs per framework/level |
| --- | --- | ---: | ---: | ---: |
| 1 | Hello World | 50,000 requests | 64 | 5 |
| 1 | Posts CRUD | 2,000 CRUD cycles | 64 | 5 |
| 1 | Auth | 25,000 login/profile pairs | 64 | 5 |
| 2 | Hello World | 512 requests per worker | 128, 256 | 3 |
| 2 | Posts CRUD | 64 CRUD cycles per worker | 128, 256 | 3 |
| 2 | Auth | 256 login/profile pairs per worker | 128, 256 | 3 |

Each CRUD cycle performs five HTTP requests, and each auth pair performs two. Throughput is measured in HTTP responses per second; p95 is for one complete Hello request, CRUD cycle, or auth pair. The runner fully consumes response bodies, checks every status and body, uses persistent HTTP/1.1 connections, disables redirects and retries, and warms up each trial before measurement. The workload is closed loop: each worker starts its next operation as soon as the previous one completes, with no think time.

## Schema 1 — 64 workers

Values are median responses per second and median p95 operation latency across five runs.

| Framework | Hello responses/s | Hello p95 | Posts responses/s | CRUD p95 | Auth responses/s | Auth p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| FURNACE beta.3 | 67,848 | 1.18 ms | 6,033 | 17.50 ms | 35,199 | 2.89 ms |
| Axum | 70,766 | 1.23 ms | 7,105 | 12.15 ms | 45,584 | 2.22 ms |
| Fiber | 69,291 | 1.69 ms | 5,474 | 31.62 ms | 34,808 | 3.36 ms |

All 45 schema 1 trials passed. They completed 1,650,000 measured HTTP responses across the three frameworks and workloads.

## Schema 2 — throughput ramp

Each cell shows `responses/s / p95 latency`. All trials passed through 256 active workers; each framework completed the two-step plateau criterion by that level.

### Hello World

| Framework | 64 workers | 128 workers | 256 workers |
| --- | ---: | ---: | ---: |
| FURNACE beta.3 | 67,848 / 1.18 ms | 65,291 / 2.79 ms | 61,308 / 5.22 ms |
| Axum | 70,766 / 1.23 ms | 69,254 / 2.52 ms | 59,438 / 5.97 ms |
| Fiber | 69,291 / 1.69 ms | 69,111 / 3.43 ms | 63,474 / 6.12 ms |

### Posts CRUD

| Framework | 64 workers | 128 workers | 256 workers |
| --- | ---: | ---: | ---: |
| FURNACE beta.3 | 6,033 / 17.50 ms | 5,493 / 35.11 ms | 5,226 / 73.12 ms |
| Axum | 7,105 / 12.15 ms | 6,246 / 24.65 ms | 5,828 / 52.17 ms |
| Fiber | 5,474 / 31.62 ms | 5,696 / 68.21 ms | 5,929 / 147.51 ms |

### Auth

| Framework | 64 workers | 128 workers | 256 workers |
| --- | ---: | ---: | ---: |
| FURNACE beta.3 | 35,199 / 2.89 ms | 35,373 / 4.79 ms | 32,924 / 8.96 ms |
| Axum | 45,584 / 2.22 ms | 41,257 / 4.00 ms | 39,948 / 7.43 ms |
| Fiber | 34,808 / 3.36 ms | 33,566 / 6.99 ms | 32,099 / 11.77 ms |

All 54 schema 2 trials passed. Across both schemas, all 99 measured trials completed **6,294,864 HTTP responses** with no contract failures. Hello and auth servers used about one full core during the ramp. Posts server CPU stayed below 70% of one core and the reqwest client below 31%; PostgreSQL CPU and waits were not sampled separately, so the database-side limit is not identified. The reported plateau reflects this closed-loop workload and CPU allocation.

## Raw evidence

- [Schema 1 fixed-64 comparison](results/2026-10-16-reqwest/schema1-64.json)
- [Schema 2 capacity summary](results/2026-10-16-reqwest/schema2-capacity.json)
- [Hello World ramp at 128](results/2026-10-16-reqwest/schema2-hello-128.json), [at 256](results/2026-10-16-reqwest/schema2-hello-256.json)
- [Posts CRUD ramp at 128](results/2026-10-16-reqwest/schema2-posts-128.json), [at 256](results/2026-10-16-reqwest/schema2-posts-256.json)
- [Auth ramp at 128](results/2026-10-16-reqwest/schema2-auth-128.json), [at 256](results/2026-10-16-reqwest/schema2-auth-256.json)
- [Auth request-log silence check](results/2026-10-16-reqwest/request-logging-check.json)

## Reproduce the Rust reqwest comparison

The standalone runner uses Rust, Tokio, and reqwest for all measured HTTP
traffic. Build it and the three app targets from the repository root:

```sh
cargo build --release --locked --manifest-path benchmark/runner/Cargo.toml
benchmark/runner/target/release/furnace-bench build
```

For posts CRUD, provision three disposable PostgreSQL databases, apply
`example/posts-crud/migrations/001_create_posts.sql` to each, then set
`BENCH_FURNACE_DATABASE_URL`, `BENCH_AXUM_DATABASE_URL`, and
`BENCH_FIBER_DATABASE_URL`. The runner does not create or drop databases.

Run the fixed 64-worker comparison with:

```sh
benchmark/runner/target/release/furnace-bench compare \
  --scenario hello --scenario posts --scenario auth --profile extended \
  --concurrency 64 --runs 5 --seed 20261016 \
  --output /tmp/furnace-schema1-64.json
```

Other useful commands are:

```sh
# Run the FURNACE-only HTTP and PostgreSQL correctness workload.
benchmark/runner/target/release/furnace-bench stress --profile stress \
  --output /tmp/furnace-stress.json

# Measure an endpoint that is already running.
benchmark/runner/target/release/furnace-bench measure --scenario hello \
  --url http://127.0.0.1:3000 --operations 50000 --concurrency 64 \
  --seed 20261016 --output /tmp/furnace-single.json

# Override the operations and concurrency for a comparison.
benchmark/runner/target/release/furnace-bench compare --scenario hello \
  --operations 200000 --concurrency 128 --runs 5 --seed 20261016 \
  --output /tmp/furnace-hello-128.json
```

For the schema 2 ramp, run each workload at 128 and 256 workers. Set operations
to workers × 512 for Hello World, workers × 64 for CRUD cycles, or workers × 256
for auth pairs; use three runs per level. The report records each output and the
throughput-plateau rule used to stop the ramp. See the [runner guide](runner/README.md)
for CLI behavior, response contracts, CPU accounting, and limitations.

The runner supports repeatable `--framework` and `--scenario` options;
`--case` is an alias for `--scenario`. `compare` defaults to all three
frameworks and the hello/posts/auth workloads. `--operations`, `--concurrency`,
`--runs`, and `--seed` select the measured load and randomized schedule. A
fresh server process starts for each framework/workload trial. Readiness,
warmup, statuses, bodies, transport completion, and response contracts must
pass before results are accepted. JSON includes each run, response counts,
nearest-rank p50/p95/p99, medians across runs, startup and CPU time, trial order,
and source/binary hashes.

The Linux CPU meter samples process thread accounting outside the timed
interval; `100%` represents one core, and unavailable readings are `null`.
Affinity is optional. To isolate the client and server, launch the runner under
`taskset` and set `BENCH_SERVER_CPUS`; inspect `lscpu --parse=CPU,CORE` to avoid
placing SMT siblings in both partitions. The recorded run was closed loop with
no think time and did not pin CPU frequency. The client and app share one host,
so the result is an end-to-end application comparison.

To replay a successful Rust report with the historical Python HTTP client, use
the same database URLs and CPU placement:

```sh
python3 benchmark/tool/python_reference.py \
  --reference /tmp/furnace-schema1-64.json \
  --output /tmp/furnace-python-reference.json
```

The adapter checks binary and operation-order hashes. It exists to compare the
old request client; current framework measurements should use the Rust runner.

The CLI can also measure already-built dependency-graph fixtures. These are
separate from the current three-app comparison and require the historical graph
binaries:

```sh
benchmark/runner/target/release/furnace-bench compare \
  --framework furnace-rs-1.0.2 --framework furnace-rs-beta \
  --scenario hello --scenario auth --scenario di-graph-noop \
  --scenario di-graph-traverse --dependency-counts 1,16,64,256 \
  --profile stress --runs 5 --seed 20261016 \
  --output /tmp/furnace-di-reqwest.json
```

## Historical performance reports

These reports preserve their original test dates and Python-client methods; do
not compare their absolute rates directly with the current Rust reqwest run:

- [FURNACE, Axum, and Fiber comparison](COMPARISON.md): 2025-09-25 app
  comparison and 2026-10-11/12 declarative-DI measurements. Raw results are
  [2025-09-25](results/2026-09-25-furnace-axum-fiber-stress.json),
  [2026-10-11](results/2026-10-11-di-runtime-comparison.json), and
  [2026-10-12](results/2026-10-12-di-randomized-comparison.json).
- [Historical HTTP workload report](REPORT.md): 2025-09-24/25 measurements
  for FURNACE 0.9.0/0.9.1, including CRUD, connection churn, validation, and
  recovery behavior. Its raw results remain under `results/2026-09-24-*`.
- Security and availability correctness comparisons are collected separately
  in [SECURITY.md](SECURITY.md); their correctness timings are not performance
  benchmarks.

The archived Python comparison tools can also reproduce older measurement
methods when their framework versions, databases, and host setup are available:

```sh
python3 benchmark/tool/build_furnace.py --offline
python3 benchmark/tool/compare.py --profile stress --runs 5 \
  --output /tmp/furnace-python-comparison.json
python3 benchmark/tool/build_di_comparison.py \
  --dependency-counts 1,16,64,256
python3 benchmark/tool/di_comparison.py --profile stress --runs 5 \
  --dependency-counts 1,16,64,256 --seed 20261012 \
  --output /tmp/furnace-python-di.json
```

These legacy rates include Python client overhead and use older benchmark
targets; their reports retain the actual versions, setup, raw results, and
measurement limits.
