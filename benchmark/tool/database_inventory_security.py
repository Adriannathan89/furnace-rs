#!/usr/bin/env python3
"""Compare database and inventory fixes with isolated regression fixtures."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shlex
import socket
import subprocess
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
CONFIG = Path("crates/furnace-rs-persistence/src/sea_orm/config.rs")
CONFIG_TEST = "sea_orm::config::tests::zero_maintenance_deadlines_are_rejected_on_their_configuration_keys"
INVENTORY = "inventory_constructor_type_mismatch_prevents_a_successful_build"
NATIVE = "zero_maintenance_deadlines_are_rejected_before_creating_a_pool"
JWT = (
    "rsa_public_key_cannot_be_reinterpreted_as_an_hmac_secret",
    "named_keys_and_algorithms_reject_confusion_before_signature_validation",
    "current_key_signs_while_previous_key_remains_verify_only",
)
RECOVERY = (
    "terminated_idle_connection_is_replaced_without_rebuilding_the_application",
    "pool_acquisition_timeout_does_not_permanently_consume_capacity",
    "query_error_and_cancelled_query_leave_the_pool_usable",
)
FIXTURES = (
    "crates/furnace-rs-core/tests/inventory_type_safety.rs",
    "crates/furnace-rs-persistence/tests/connector.rs",
    "crates/furnace-rs-persistence/tests/recovery.rs",
    "crates/furnace-rs-common/tests/jwt_rotation.rs",
)


def fingerprint(root: Path) -> str:
    paths = [Path("Cargo.toml"), Path("Cargo.lock")]
    for crate in ("core", "common", "persistence"):
        directory = Path(f"crates/furnace-rs-{crate}")
        paths.append(directory / "Cargo.toml")
        paths.extend(sorted((root / directory / "src").rglob("*.rs")))
    digest = hashlib.sha256()
    for path in paths:
        relative = path.relative_to(root) if path.is_absolute() else path
        digest.update(str(relative).encode())
        digest.update((root / relative).read_bytes())
    return digest.hexdigest()


def verify_result(output: str, code: int, required: tuple[str, ...], failure: str | None = None) -> dict:
    summaries = re.findall(
        r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s", output
    )
    if len(summaries) != 1:
        raise ValueError("expected exactly one completed Rust test result")
    status, passed, failed, ignored, elapsed = summaries[0]
    passed, failed, ignored = int(passed), int(failed), int(ignored)
    outcome = "FAILED" if failure else "ok"
    present = all(re.search(rf"^test {re.escape(name)} \.\.\. {outcome}$", output, re.MULTILINE)
                  for name in required)
    if failure:
        verified = (code == 101 and status == "FAILED" and passed == 0 and failed == 1
                    and ignored == 0 and present and failure in output)
    else:
        verified = (code == 0 and status == "ok" and failed == 0 and ignored == 0
                    and passed >= len(required) and present)
    return {"tests_passed": passed, "tests_failed": failed, "tests_ignored": ignored,
            "test_runtime_seconds": float(elapsed), "expected_outcome_verified": bool(verified)}


def run(root: Path, package: str, target: list[str], required: tuple[str, ...],
        url: str, failure: str | None = None, ignored: bool = False) -> dict:
    command = ["cargo", "test", "--offline", "--locked", "-p", package,
               "--all-features", *target, "--", "--nocapture"]
    if ignored:
        command.append("--ignored")
    started = time.monotonic()
    process = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=300,
        env={**os.environ, "CARGO_NET_OFFLINE": "true", "FURNACE_TEST_DATABASE_URL": url})
    output = process.stdout + process.stderr
    return {"command": command, "cargo_exit_code": process.returncode, "output": output,
            "elapsed_seconds_including_build": round(time.monotonic() - started, 3),
            **verify_result(output, process.returncode, required, failure)}


@contextmanager
def postgres(directory: Path, binaries: Path):
    data = directory / "postgres"
    subprocess.run([str(binaries / "initdb"), "-D", str(data), "-A", "trust", "-U", "furnace_fixture",
                    "--no-locale"], check=True, capture_output=True, text=True, timeout=60)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    control = [str(binaries / "pg_ctl"), "-D", str(data)]
    options = shlex.join(["-h", "127.0.0.1", "-p", str(port), "-k", str(data)])
    try:
        subprocess.run([*control, "-l", str(directory / "postgres.log"), "-o", options, "-w", "start"],
                       check=True, capture_output=True, text=True, timeout=60)
        yield f"postgres://furnace_fixture@127.0.0.1:{port}/postgres"
    finally:
        # Attempt cleanup even if startup failed partway through.
        stopped = subprocess.run([*control, "-m", "fast", "-w", "stop"],
                                 capture_output=True, text=True, timeout=60)
        if stopped.returncode and (data / "postmaster.pid").exists():
            raise RuntimeError("temporary PostgreSQL server could not be stopped")


def snapshot(directory: Path, revision: str) -> tuple[Path, str]:
    directory.mkdir()
    archive = subprocess.check_output(["git", "archive", revision], cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        files.extractall(directory, filter="data")
    source_hash = fingerprint(directory)
    for fixture in FIXTURES:
        (directory / fixture).write_bytes((ROOT / fixture).read_bytes())
    source = (ROOT / CONFIG).read_text()
    marker = "    #[test]\n    fn zero_maintenance_deadlines_are_rejected_on_their_configuration_keys()"
    start = source.index(marker)
    end = source.index("    #[test]", start + len(marker))
    original = (directory / CONFIG).read_text()
    closing = original.rfind("\n}")
    if closing < 0 or original[closing:].strip() != "}":
        raise ValueError("cannot locate baseline test module closing brace")
    (directory / CONFIG).write_text(original[:closing] + "\n" + source[start:end] + original[closing:])
    return directory, source_hash


def measure(root: Path, url: str, before: bool) -> dict:
    cases = {
        "inventory": run(root, "furnace-rs-core", ["--test", "inventory_type_safety"], (INVENTORY,), url,
            "an invalid inventory output must not produce a running application" if before else None),
        "native_maintenance": run(root, "furnace-rs-persistence",
            ["--test", "connector", NATIVE] if before else ["--test", "connector"], (NATIVE,), url,
            'called `Result::unwrap_err()` on an `Ok` value: DatabaseConnection' if before else None),
        "configured_maintenance": run(root, "furnace-rs-persistence",
            ["--lib", "zero_maintenance_deadlines"] if before else ["--lib"], (CONFIG_TEST,), url,
            'called `Result::unwrap_err()` on an `Ok` value: SeaOrmConfig' if before else None),
        "jwt_controls": run(root, "furnace-rs-common", ["--test", "jwt_rotation"], JWT, url),
        "recovery_controls": run(root, "furnace-rs-persistence", ["--test", "recovery"], RECOVERY, url, ignored=True),
    }
    if not before:
        cases["registry_controls"] = run(root, "furnace-rs-core", ["--test", "registry"], (
            "rejects_invalid_erased_provider_storage_without_poisoning_the_registry",
            "resolves_the_same_application_scoped_allocation",
            "contexts_resolve_shared_providers_and_expose_merged_configuration",
        ), url)
    return cases


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", default="10f5958")
    parser.add_argument("--postgres-bin", type=Path, default=Path("/usr/lib/postgresql/16/bin"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), "build_profile": "debug",
              "method": "isolated regression comparison; no CPU stress or deployed database access",
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "fixture_sha256": {p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in FIXTURES}}
    try:
        revision = subprocess.check_output(["git", "rev-parse", "--verify", f"{args.before}^{{commit}}"],
                                           cwd=ROOT, text=True).strip()
        report["baseline_revision"] = revision
        report["after_base_revision"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        report["after_source_sha256"] = fingerprint(ROOT)
        report["after_source_kind"] = "patched working tree before commit; identified by source fingerprint"
        report["postgres_version"] = subprocess.check_output([str(args.postgres_bin / "postgres"), "--version"], text=True).strip()
        with tempfile.TemporaryDirectory(prefix="furnace-database-inventory-") as temporary:
            directory = Path(temporary)
            baseline, report["baseline_source_sha256"] = snapshot(directory / "baseline", revision)
            with postgres(directory, args.postgres_bin) as url:
                report["before"] = measure(baseline, url, True)
                report["after"] = measure(ROOT, url, False)
        report["passed"] = all(case["expected_outcome_verified"]
                               for stage in ("before", "after") for case in report[stage].values())
    except (OSError, subprocess.SubprocessError, ValueError, RuntimeError) as error:
        report["passed"] = False
        report["failure"] = f"comparison could not complete ({type(error).__name__}): {error}"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": report["passed"], "output": str(args.output),
                      "failure": report.get("failure")}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
