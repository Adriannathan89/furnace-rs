# Database, JWT, and inventory security regression benchmark

Measured on 2026-10-06 at 01:31:14 UTC against baseline
`10f5958f933028a930c36076bd36ae7468223859` and the patched working tree.
The source fingerprints in the [raw results](results/2026-10-06-database-inventory-security-before-after.json)
identify the exact measured sources before the patch commit.

## Before and after

| Contract | Baseline | Patched runtime |
| --- | --- | --- |
| Inventory constructor returns a different concrete type from its descriptor | Application build succeeds with an invalid stored output; regression fails as expected | Build fails immediately with `FURNACE004`; invalid registry insertion leaves storage empty and permits a later valid insertion |
| Native zero maintenance duration | Zero idle duration is accepted; regression fails as expected | Zero idle and lifetime durations return `InvalidConfiguration` before pool creation; valid durations and native `None` settings still work |
| Configured zero maintenance duration | Zero idle duration is accepted; regression fails as expected | Idle and lifetime zero values produce `too_small` on their configuration keys |
| JWT algorithm confusion and key rotation | All 3 controls pass | All 3 controls pass, including rejecting HS256 signed using RSA public-key bytes and accepting valid RSA tokens |
| PostgreSQL recovery | All 3 controls pass | All 3 controls pass: replace a terminated idle connection, recover capacity after an acquisition timeout, and recover after a query error and cancellation |

The zero-duration baseline fixtures stop at the first failed assertion, which
uses the idle duration. The patched fixtures complete assertions for both idle
and lifetime settings. SQLx's maintenance loop uses the smaller configured
interval; a zero interval can continuously reschedule its maintenance task.
The benchmark verifies rejection of that input and does not measure CPU load.

JWT algorithm binding and native PostgreSQL recovery already worked in the
baseline. They are retained safety controls; this patch adds no JWT production
change, query replay, or connection retry layer.

## Recorded measurements

| Stage | Baseline passed / failed | Patched passed / failed | Baseline Rust duration | Patched Rust duration | Baseline including build | Patched including build |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Inventory output | 0 / 1 expected | 1 / 0 | 0.00 s | 0.00 s | 9.714 s | 6.566 s |
| Native maintenance settings | 0 / 1 expected | 6 / 0 | 0.00 s | 0.00 s | 28.036 s | 12.987 s |
| Configured maintenance settings | 0 / 1 expected | 10 / 0 | 0.00 s | 0.00 s | 1.719 s | 0.956 s |
| JWT controls | 3 / 0 | 3 / 0 | 0.04 s | 0.05 s | 22.387 s | 0.936 s |
| PostgreSQL recovery controls | 3 / 0 | 3 / 0 | 1.04 s | 1.03 s | 2.343 s | 1.232 s |
| Registry controls | Not separately measured | 3 / 0 | — | 0.00 s | — | 0.626 s |

The runner returned zero after verifying all three specific baseline failures
and **26 passing patched tests**. Both JWT and recovery control groups passed
on each version. Baseline regression Cargo commands return 101 by design;
patched commands and control commands return zero.

This is a correctness benchmark. Rust reports durations to two decimal places;
`0.00 s` means below that reporting precision. Some patched groups contain more
tests than their baseline counterparts, and compilation caches differ. These
timings do not establish a throughput improvement or per-query latency change.

## Method and reproduction

The runner archives the pinned baseline into a temporary directory and adds
the current regression fixtures without changing production behavior. Its only
baseline source insertion is the test-only configuration fixture inside the
existing test module. It runs locked, offline, all-feature Cargo tests and records
commands, full output, exit codes, timings, source fingerprints, fixture hashes,
and the runner hash in the JSON result. Source fingerprints cover the core,
common, and persistence source trees and their manifests plus the workspace
manifest and lockfile; baseline source hashing occurs before fixture insertion.

PostgreSQL 16.15 runs in a temporary cluster with a fixture-only role, trust
authentication, and a dynamically selected loopback port. The recovery tests
terminate only their own fixture backend, assert the specific acquisition
timeout variant, and observe the running query in `pg_stat_activity` before
cancelling its task. Each test checks subsequent native pool usability.
The runner stops PostgreSQL and removes temporary directories on completion.
No deployment URL or credentials are used.

Requires Python 3.12 or newer, cached Rust dependencies, local baseline Git
history, PostgreSQL binaries, and permission to bind loopback sockets:

```sh
python3 benchmark/tool/database_inventory_security.py \
  --postgres-bin /usr/lib/postgresql/16/bin \
  --output /tmp/furnace-database-inventory-security.json
```

The runner rejects missing or ambiguous Rust summaries, incorrect named test
outcomes, unexpected baseline diagnostics, ignored required tests, and nonzero
patched or control exits. Its default baseline is `10f5958`; `--before` can
select another compatible pre-patch revision. Keep the checkout unchanged
during measurement. Recovery coverage is bounded to the tested PostgreSQL
failures; server restarts, prolonged network partitions, and sustained-load
capacity are not measured.

Validation alongside this benchmark: the workspace all-feature suite passed
785 tests with 7 ignored; the three recovery tests were executed separately
against real PostgreSQL. Clippy with warnings denied and formatting checks
passed. The benchmark-tool suite ran 42 tests successfully with 3 skips.
