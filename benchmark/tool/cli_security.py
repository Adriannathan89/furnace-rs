#!/usr/bin/env python3
"""Check private CLI directory permissions against independent Unix umasks."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

from run import ROOT, git_head

PROFILES = {"smoke": 2, "stress": 200, "extended": 1000}
MARKER = "FURNACE_CLI_SECURITY_RESULT "


def parse_result(output: str, returncode: int, rounds: int) -> dict:
    lines = [line[len(MARKER):] for line in output.splitlines() if line.startswith(MARKER)]
    if len(lines) != 1:
        raise ValueError("expected exactly one CLI security result")
    result = json.loads(lines[0])
    if not isinstance(result, dict) or result.get("rounds") != rounds:
        raise ValueError("CLI security rounds differ from the requested profile")
    result["passed"] = (
        returncode == 0 and result.get("passed") is True
        and result.get("checks") == rounds * 4 and result.get("failures") == 0
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, default="smoke")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    rounds = PROFILES[args.profile]
    source = hashlib.sha256()
    for path in ("crates/furnace-rs-cli/src/private_temp_dir.rs",
                 "crates/furnace-rs-cli/src/lib.rs",
                 "crates/furnace-rs-cli/src/process.rs",
                 "crates/furnace-rs-cli/src/inspection.rs", "Cargo.lock"):
        source.update(path.encode())
        source.update((ROOT / path).read_bytes())
    report = {"timestamp_utc": datetime.now(timezone.utc).isoformat(),
              "git_head": git_head(), "source_sha256": source.hexdigest(),
              "profile": args.profile, "umasks": ["000", "002", "022", "077"],
              "build_profile": "debug", "database_required": False}
    try:
        if os.name != "posix":
            raise ValueError("the CLI permission benchmark requires Unix")
        started = datetime.now(timezone.utc)
        process = subprocess.run(
            ["cargo", "test", "--offline", "--locked", "-p", "furnace-rs-cli", "--lib",
             "private_directories_exclude_other_users_under_each_umask", "--", "--nocapture"],
            cwd=ROOT, env={**os.environ, "FURNACE_CLI_SECURITY_ROUNDS": str(rounds)},
            capture_output=True, text=True, timeout=300,
        )
        report["elapsed_seconds_including_build"] = (datetime.now(timezone.utc) - started).total_seconds()
        report["cargo_exit_code"] = process.returncode
        report["result"] = parse_result(process.stdout, process.returncode, rounds)
        report["passed"] = report["result"]["passed"]
    except (OSError, subprocess.TimeoutExpired, ValueError, TypeError) as error:
        report["passed"] = False
        report["failure"] = f"CLI security benchmark could not complete ({type(error).__name__})"
    output = json.dumps(report, indent=2)
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
