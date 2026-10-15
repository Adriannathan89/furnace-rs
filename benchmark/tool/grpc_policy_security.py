#!/usr/bin/env python3
"""Compare the F01 defensive admission regression on baseline and patched source."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = Path("crates/furnace-rs-grpc/tests/grpc_conditional_policy.rs")
MARKER = "FURNACE_GRPC_POLICY_RESULT "
BASELINE = "bd8ade6aa4759ad5072dd82a63b7c2006c31affc"
RUNTIME_CRATES = (
    "furnace-rs-core", "furnace-rs-core-macros", "furnace-rs-common",
    "furnace-rs-common-macros", "furnace-rs-grpc", "furnace-rs-grpc-macros",
)


def fingerprint(root: Path) -> str:
    """Identify the fixture's production crates, workspace manifests and lock."""
    paths = [Path("Cargo.toml"), Path("Cargo.lock"), FIXTURE]
    paths.extend(path.relative_to(root) for path in (root / "crates").glob("*/Cargo.toml"))
    for crate in RUNTIME_CRATES:
        paths.extend(path.relative_to(root) for path in (root / "crates" / crate / "src").rglob("*.rs"))
    paths.extend(path.relative_to(root) for path in (root / "tests/grpc-support").rglob("*.*"))
    digest = hashlib.sha256()
    for path in sorted(set(paths)):
        if "target" in path.parts or not (root / path).is_file():
            continue
        digest.update(path.as_posix().encode() + b"\0")
        digest.update((root / path).read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def parse_result(output: str, exit_code: int, rounds: int, before: bool) -> dict:
    """Require actual executed checks; build errors or missing markers never pass."""
    markers = [line[len(MARKER):] for line in output.splitlines() if line.startswith(MARKER)]
    if len(markers) != 1:
        raise ValueError("expected exactly one completed admission result")
    result = json.loads(markers[0])
    summaries = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", output)
    expected_summary = [("FAILED", "0", "1")] if before else [("ok", "1", "0")]
    expected = {
        "rounds": rounds,
        "guard_retained": not before,
        "anonymous_rejected": 0 if before else rounds,
        "authenticated_accepted": rounds,
        "public_accepted": rounds,
        "guarded_handler_calls": rounds * (2 if before else 1),
    }
    verified = (
        result == expected
        and summaries == expected_summary
        and exit_code == (101 if before else 0)
    )
    return {"observed": result, "expected_outcome_verified": verified,
            "security_contract_passed": not before and verified}


def run(root: Path, rounds: int, before: bool, target: Path) -> dict:
    command = ["cargo", "test", "--locked", "--offline", "-p", "furnace-rs-grpc",
               "--features", "jwt", "--test", "grpc_conditional_policy", "policy_comparison",
               "--", "--exact", "--nocapture"]
    started = time.monotonic()
    process = subprocess.run(
        command, cwd=root, capture_output=True, text=True, timeout=600,
        env={**os.environ, "FURNACE_GRPC_POLICY_ROUNDS": str(rounds),
             "CARGO_TARGET_DIR": str(target), "CARGO_NET_OFFLINE": "true"},
    )
    output = process.stdout + process.stderr
    result = parse_result(output, process.returncode, rounds, before)
    return {"command": command, "cargo_exit_code": process.returncode,
            "elapsed_seconds_including_build": round(time.monotonic() - started, 3),
            "source_sha256": fingerprint(root), **result, "output": output}


def compare(baseline: str, rounds: int) -> dict:
    commit = subprocess.check_output(
        ["git", "rev-parse", "--verify", f"{baseline}^{{commit}}"], cwd=ROOT, text=True,
    ).strip()
    report = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), "baseline_commit": commit,
              "current_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "current_source": "working tree, including uncommitted patches",
              "rounds": rounds, "network_required": False, "database_required": False,
              "method": "same socket-free regression fixture; synthetic JWT and local in-process RPCs",
              "source_scope": {"production_crates": RUNTIME_CRATES,
                               "other_inputs": ["workspace manifests and lock", "grpc-support", str(FIXTURE)]},
              "fixture_sha256": hashlib.sha256((ROOT / FIXTURE).read_bytes()).hexdigest(),
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    with tempfile.TemporaryDirectory(prefix="furnace-grpc-policy-") as temporary:
        directory = Path(temporary)
        archive = subprocess.check_output(
            ["git", "archive", commit, "Cargo.toml", "Cargo.lock", "crates", "tests/grpc-support"], cwd=ROOT,
        )
        with tarfile.open(fileobj=io.BytesIO(archive)) as files:
            files.extractall(directory, filter="data")
        (directory / FIXTURE).write_bytes((ROOT / FIXTURE).read_bytes())
        report["before"] = run(directory, rounds, True, ROOT / "target/grpc-policy-before")
    report["after"] = run(ROOT, rounds, False, ROOT / "target")
    report["passed"] = all(report[key]["expected_outcome_verified"] for key in ("before", "after"))
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default=BASELINE)
    parser.add_argument("--rounds", type=int, default=20)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if not 1 <= args.rounds <= 10_000:
        parser.error("rounds must be between 1 and 10000")
    try:
        report = compare(args.baseline, args.rounds)
    except (OSError, subprocess.SubprocessError, ValueError, TypeError, KeyError) as error:
        report = {"passed": False, "failure": f"comparison did not complete ({type(error).__name__}): {error}"}
    output = json.dumps(report, indent=2) + "\n"
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
