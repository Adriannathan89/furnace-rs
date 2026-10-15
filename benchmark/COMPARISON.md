# FURNACE vs native Axum vs Go Fiber v2 — local benchmark

Measured 2026-09-25. The [raw result](results/2026-09-25-furnace-axum-fiber-stress.json)
contains all five measured runs for every combination, including status counts,
client-observed p50/p95/p99 latency, and errors. All 420,000 measured HTTP
responses had the expected status, and content checks on valid-response paths
passed; there were zero measured transport or contract errors.

| Workload | Framework | Median responses/s | Range across 5 runs | Median p95 | Median p99 |
| --- | --- | ---: | ---: | ---: | ---: |
| Hello World | FURNACE 0.9.1 | 8,884 | 8,174–8,983 | 17.38 ms | 26.93 ms |
| Hello World | Axum 0.8.9 | 8,436 | 8,241–8,611 | 18.74 ms | 27.96 ms |
| Hello World | Fiber v2.52.15 | 8,432 | 8,269–8,574 | 18.29 ms | 27.26 ms |
| Posts CRUD | FURNACE 0.9.1 | 5,538 | 4,418–5,589 | 11.78 ms | 16.19 ms |
| Posts CRUD | Axum 0.8.9 | 5,851 | 4,111–5,928 | 11.10 ms | 15.04 ms |
| Posts CRUD | Fiber v2.52.15 | 5,668 | 5,410–5,782 | 11.20 ms | 15.28 ms |
| Login + protected JWT read | FURNACE 0.9.1 | 6,711 | 6,591–6,883 | 21.22 ms | 30.11 ms |
| Login + protected JWT read | Axum 0.8.9 | 6,861 | 6,771–6,921 | 21.07 ms | 29.42 ms |
| Login + protected JWT read | Fiber v2.52.15 | 6,482 | 5,856–6,709 | 22.16 ms | 30.64 ms |

The medians describe this host and client, not a universal ranking. Hello World
is nearly tied. FURNACE and Axum are also close for valid JWT traffic; Fiber's
median was lower on that workload in this run. The CRUD ranges are especially
wide for FURNACE and Axum, so their small median differences are not reliable
evidence of framework overhead. No framework error was found in the valid
request workloads.

## Method

- One local Linux host: AMD Ryzen 5 6600H (6 cores/12 threads), 14 GiB RAM;
  Rust 1.96.0, Go 1.22.2, Python 3.12.3. PostgreSQL 16.15 ran in a disposable
  Docker container. The source checkout was at `3efd8db` before benchmark
  files were added. FURNACE was built from this checkout's 0.9.1 source.
- Each framework ran alone on loopback HTTP/1.1, with release/optimized
  binaries. The same Python standard-library client drove all runs with
  persistent connections. Logger output went to a separate file for each
  process. Servers ran in FURNACE, Axum, Fiber order for each workload.
- Five measured runs followed contract checks and a warmup at the same
  concurrency. Hello World: 12,000 GETs/run at 64 clients. CRUD: 800
  create/read/update/delete/confirm-missing transactions/run at 32 clients,
  or 4,000 HTTP responses/run. Authentication: 6,000 valid login + protected
  profile pairs/run at 64 clients, or 12,000 responses/run. The table reports
  the median of each run's throughput and latency percentile, not a pooled
  percentile across runs.
- The three CRUD targets used the same `posts` schema in three separate
  databases within the same PostgreSQL instance. All used a maximum of ten
  database connections. Auth used HS256 JWTs, the same demo credentials, and
  per-success login/profile logging. JSON structure and important statuses
  were checked. Missing/malformed JWT checks require the FURNACE Passport 401
  status, `WWW-Authenticate: Bearer`, and unauthorized JSON envelope; Bearer
  scheme case is tested as well. Invalid fields, malformed JSON, unsupported
  content types, and missing posts are checked for status and shared error
  code outside the measured hot path; detailed validation/not-found payloads
  are not normalized across frameworks.

## Separate body-limit observation

A 3 MiB JSON upload to `/auth/login` was probed outside measured runs. FURNACE
and the native Axum target returned HTTP 413. With Fiber v2's 2 MiB body
limit, this Python client instead observed `ConnectionResetError` before
receiving an HTTP response. That is a real interoperability difference for
this client/target combination; the valid-request throughput result does not
test or erase it. The Axum benchmark target initially mapped this rejection
to 422; that adapter error was corrected before the final measurements.

## Interpretation and reproducibility

This is an **end-to-end application comparison**, not an isolated router
microbenchmark. The PostgreSQL paths use SeaORM/SQLx (FURNACE), SQLx (native
Axum), and Go `database/sql` with `lib/pq` (Fiber). The JWT libraries and
response encodings also differ. Python client throughput, local scheduling,
database cache/checkpoints, and logger I/O contribute to the numbers. No TLS,
HTTP/2, remote network, multi-host setup, or sustained soak was measured.
Five runs reduce but do not eliminate environmental noise; in particular,
several CRUD runs were slower than the rest without correctness failures.
The FURNACE example updates a post with a lookup followed by an update, while the
Axum and Fiber targets issue a single `UPDATE ... RETURNING`; the CRUD results
therefore do not use identical database round-trip counts.

FURNACE's existing published-package example lockfiles currently fail a direct
build with a registry checksum mismatch for `furnace-rs-common-macros 0.9.1`.
The benchmark setup stages the unchanged FURNACE examples in a temporary
directory, patches framework dependencies to this local checkout, and uses
the pinned local-source locks in `benchmark/targets/furnace-rs/locks/`. This
does not claim to measure the published crate archive. The Axum `Cargo.lock`
and Fiber `go.sum` are included separately. See [benchmark setup](README.md#furnace-rs-vs-native-axum-vs-go-fiber-v2)
for build and rerun commands.

## 2026-10-11 — declarative DI runtime comparison

Declarative dependency injection was introduced in FURNACE 2.0.0-beta.1.
These 2026-10-11 and 2026-10-12 runs measured earlier development commits
whose workspace metadata still reported `1.1.0-beta.1`. The tables and raw
results preserve those recorded versions; these runs do not measure the later
mandatory injection-marker release.

This rerun compares the released **FURNACE 1.0.2** source at `8c08bba` with **1.1.0-beta.1** source at `613ee02`, then places the existing Axum and Fiber targets alongside them. The full raw runs, host details, lock hashes, contract outcomes and pair order are in [the machine-readable result](results/2026-10-11-di-runtime-comparison.json).

The FURNACE versions ran in five paired rounds with order alternated. Each round first passed the benchmark contracts and warmup, then measured 12,000 Hello requests or 6,000 valid login/profile pairs (12,000 HTTP responses) at 64 clients. Axum 0.8.9 and Fiber 2.52.15 each ran five fresh stress measurements immediately afterward. All measured responses had expected status/content and zero errors. Medians below are medians of per-run throughput and p95.

| Workload | FURNACE 1.0.2 | FURNACE 1.1.0-beta.1 | Axum 0.8.9 | Fiber v2.52.15 |
| --- | ---: | ---: | ---: | ---: |
| Hello World | 8,821.6 resp/s; 17.06 ms | 9,042.4 resp/s; 16.84 ms | 8,885.4 resp/s; 17.51 ms | 8,942.7 resp/s; 17.19 ms |
| Login + protected JWT read | 6,891.9 resp/s; 20.52 ms | 6,948.6 resp/s; 20.98 ms | 6,999.9 resp/s; 20.29 ms | 6,993.9 resp/s; 20.50 ms |

In the paired runs, beta median throughput was **2.5% higher** for Hello World and **0.8% higher** for JWT than 1.0.2. Their per-run throughput ranges overlap for both workloads; p95 also overlaps. This small sample found no material runtime change from the DI release. It does not establish that every application graph has zero overhead. The DI assertions are compile-time checks; this HTTP run measures end-to-end framework and application work, not compiler cost or an isolated injector lookup.

PostgreSQL CRUD is omitted because no local server or disposable `BENCH_*_DATABASE_URL` databases were available. The pre-existing 2026-09-25 comparison table above remains historical: it measures FURNACE 0.9.1 and should not be conflated with this beta rerun. The refreshed Fiber oversized-body probe again reset the connection; this is recorded separately by the existing runner and is outside the valid-request throughput samples.

## 2026-10-12 — randomized declarative DI graph-size comparison

This rerun uses FURNACE **1.0.2** from `8c08bba` and **1.1.0-beta.1** from `53ed419`, plus the existing Axum 0.8.9 and Fiber 2.52.15 targets for Hello and Auth. The [raw result](results/2026-10-12-di-randomized-comparison.json) records seed `20261012`, all 120 randomized trial entries, per-worker request-order seeds and hashes, lockfile hashes, contract results, and all five measurements per target. The old and beta builds use lockfiles with the same third-party package versions.

Every target ran five stress measurements at 64 clients. Hello sends 12,000 requests per run. Auth performs 6,000 valid login/profile pairs (12,000 HTTP responses) per run. Each graph measurement sends 12,000 requests. The graph sizes are 1, 16, 64, and 256 registered providers in a balanced tree. Its no-op endpoint returns a constant after graph construction; its traversal endpoint visits every node and performs one observable relaxed atomic increment per node so release optimization cannot collapse the traversal. The latter measures application work over the injected graph, not repeated DI resolution. Furnace constructs the graph at startup.

The trial order and independent transactions were randomized with the recorded seed. Required request sequences stayed ordered within a transaction: login before profile, and create/read/update/delete in CRUD workloads. Startup is measured from process launch through the first successful readiness response, using a 2 ms readiness polling interval. It therefore includes process launch, readiness polling, and one loopback request; values at this scale are sensitive to host scheduling.

| Workload | Framework | Median responses/s | Median p95 | Median startup |
| --- | --- | ---: | ---: | ---: |
| Hello | FURNACE 1.0.2 | 8,840.3 | 16.785 ms | 3.1 ms |
| Hello | FURNACE 1.1.0-beta.1 | 8,944.3 | 16.841 ms | 3.1 ms |
| Hello | Axum 0.8.9 | 9,173.7 | 16.901 ms | 3.0 ms |
| Hello | Fiber v2.52.15 | 8,941.3 | 17.214 ms | 5.4 ms |
| Login + protected profile | FURNACE 1.0.2 | 6,865.1 | 20.695 ms | 3.1 ms |
| Login + protected profile | FURNACE 1.1.0-beta.1 | 7,079.4 | 20.282 ms | 3.2 ms |
| Login + protected profile | Axum 0.8.9 | 7,048.2 | 20.289 ms | 3.1 ms |
| Login + protected profile | Fiber v2.52.15 | 6,803.9 | 20.714 ms | 3.3 ms |

The beta median throughput is 1.2% higher for Hello and 3.1% higher for Auth than 1.0.2, but their five-run throughput ranges overlap. This sample does not show a material runtime regression from the DI change.

| Graph nodes | FURNACE version | No-op median responses/s | Traversal median responses/s | Startup median across both modes |
| ---: | --- | ---: | ---: | ---: |
| 1 | 1.0.2 | 8,677.5 | 8,858.7 | 3.1 ms (3.0–3.3) |
| 1 | 1.1.0-beta.1 | 8,660.9 | 8,720.2 | 3.1 ms (3.0–3.2) |
| 16 | 1.0.2 | 8,712.4 | 8,835.2 | 3.1 ms (2.9–3.3) |
| 16 | 1.1.0-beta.1 | 8,766.4 | 8,641.1 | 3.1 ms (3.0–3.2) |
| 64 | 1.0.2 | 8,760.2 | 8,675.0 | 5.2 ms (3.0–5.5) |
| 64 | 1.1.0-beta.1 | 8,789.8 | 8,628.6 | 4.2 ms (3.0–5.4) |
| 256 | 1.0.2 | 8,758.6 | 8,818.7 | 7.5 ms (5.2–7.6) |
| 256 | 1.1.0-beta.1 | 8,668.2 | 8,474.5 | 7.4 ms (7.3–7.5) |

No-op HTTP throughput and p95 remain similar across graph sizes, so the constructed graph does not add request-path DI work after startup. Traversal adds explicit application work but its run ranges overlap substantially; this benchmark does not establish a throughput trend by graph size. Startup medians rise from about 3.1 ms at 1–16 nodes to about 7.4 ms at 256 nodes. The old and beta startup ranges overlap, and the 64-node runs show more host-timing variation, so these measurements indicate scale rather than a precise version difference.

All 120 target runs passed their response contracts with zero measured request errors. The 3 MiB body-limit probe returned HTTP 413 for both FURNACE versions and Axum; Fiber again reset the connection, as in the earlier comparison. PostgreSQL CRUD was not run because no disposable benchmark databases were available.
