#!/usr/bin/env python3
"""Run repeatable security contracts against local Furnace core and persistence."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

from run import ROOT, git_head


PROFILES = {"smoke": 2, "stress": 200, "extended": 1000}
MARKER = "FURNACE_SECURITY_RESULT "
CHECKS_PER_ROUND = {
    "core_source_redaction": 1,
    "database_config_timeouts": 4,
    "database_native_timeout": 1,
    "valid_config_controls": 4,
    "database_connection_trace": 1,
}


def parse_result(output: str, returncode: int, rounds: int) -> dict:
    lines = [line.removeprefix(MARKER) for line in output.splitlines() if line.startswith(MARKER)]
    if len(lines) != 1:
        raise ValueError("expected exactly one security benchmark result")
    result = json.loads(lines[0])
    if result.get("rounds") != rounds:
        raise ValueError("benchmark rounds differ from requested profile")
    complete = all(
        isinstance(result.get(case), dict)
        and result[case].get("checks") == rounds * count
        and result[case].get("failures") == 0
        for case, count in CHECKS_PER_ROUND.items()
    )
    result["passed"] = returncode == 0 and result.get("passed") is True and complete
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, default="smoke")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    rounds = PROFILES[args.profile]
    report = {"timestamp_utc": datetime.now(timezone.utc).isoformat(),
              "git_head": git_head(), "profile": args.profile,
              "build_profile": "debug", "database_required": False}
    source = hashlib.sha256()
    for path in (
        "crates/furnace-rs-core/src/diagnostic.rs",
        "crates/furnace-rs-persistence/src/sea_orm/config.rs",
        "crates/furnace-rs-persistence/src/sea_orm/connector.rs",
        "crates/furnace-rs-persistence/tests/security_benchmark.rs",
    ):
        source.update(path.encode())
        source.update((ROOT / path).read_bytes())
    report["source_sha256"] = source.hexdigest()
    try:
        process = subprocess.run(
            ["cargo", "test", "--offline", "--locked", "-p", "furnace-rs-persistence",
             "--all-features", "--test", "security_benchmark", "--", "--nocapture"],
            cwd=ROOT, env={**os.environ, "FURNACE_SECURITY_ROUNDS": str(rounds)},
            capture_output=True, text=True, timeout=300,
        )
        report["cargo_exit_code"] = process.returncode
        report["result"] = parse_result(process.stdout, process.returncode, rounds)
        report["passed"] = report["result"]["passed"]
    except (OSError, subprocess.TimeoutExpired, ValueError, AttributeError, TypeError) as error:
        report["passed"] = False
        report["failure"] = f"security benchmark could not complete ({type(error).__name__})"
    output = json.dumps(report, indent=2)
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
