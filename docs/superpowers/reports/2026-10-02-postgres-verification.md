# Local PostgreSQL verification

User authorized minimal local PostgreSQL or Docker to complete the previously unavailable database checks. Executed on `refactor/mads-declaration-module`, implementation commit `61feee3`, on 2026-10-02.

## Environment

Used the installed PostgreSQL 16 binaries with a fresh cluster under `/tmp`, bound only to localhost on an allocated port. Separate databases were created for persistence tests and benchmark recovery tests. TCP authentication used SCRAM/password, ensuring invalid-credential tests exercise actual authentication rejection. The private local socket used trust authentication. Test-only settings disabled fsync, with 16 MB shared buffers and a 30-connection limit; durability under a database crash was not evaluated.

No installed database, existing container, user data or system service was changed. Docker was unnecessary. Cargo used offline/locked workspace commands and the established no-incremental/no-debug settings. Benchmark example builds used their standalone offline manifest.

## Results

| Gate | Result |
| --- | --- |
| Stable PostgreSQL acceptance tests | 4 passed, 0 failed, 0 ignored |
| Rust 1.94 PostgreSQL acceptance tests | 4 passed, 0 failed, 0 ignored |
| Benchmark Python suite with database/application environment | 24 passed, 0 failed, 0 skipped |
| Cluster shutdown and deletion | Completed successfully |

The persistence commands were:

```sh
cargo test --locked --offline -p furnace-rs-persistence \
  --features sea-orm-postgres --test postgres -- --ignored --test-threads=1
cargo +1.94.0 test --locked --offline -p furnace-rs-persistence \
  --features sea-orm-postgres --test postgres -- --ignored --test-threads=1
```

Both received `FURNACE_TEST_DATABASE_URL` pointing to the isolated cluster. Coverage includes native CRUD, committed/rolled-back transactions, invalid credential error/source redaction, pool closure following failing shutdown hooks, connection checks before HTTP bind and conventional SIGTERM shutdown.

Applied the posts migration to the separate benchmark database and built `example/posts-crud` in the debug profile. The benchmark suite received `FURNACE_TEST_BENCH_DATABASE_URL` and `FURNACE_TEST_BENCH_APP_ROOT` and ran:

```sh
python3 -m unittest discover -s benchmark/tool -p 'test_*.py'
```

The three previously skipped cases now passed: PostgreSQL table-lock visibility/release, HTTP recovery following query timeout, and HTTP recovery following a stalled database TCP response. This was a regression suite, not a throughput benchmark run.

## Updated status

The four persistence tests and three benchmark database cases previously listed as requiring external verification are now verified locally. The earlier stable/MSRV full workspace and feature gates remain as recorded in the [optional seals report](2026-10-02-optional-controller-seals-verification.md). Normal CI PostgreSQL jobs remain configured; no release or remote action was performed.
