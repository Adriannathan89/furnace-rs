#!/usr/bin/env python3
"""Run RustSec checks for every committed Cargo lockfile."""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    paths = subprocess.check_output(["git", "ls-files", "-z", "*Cargo.lock"], cwd=ROOT).split(b"\0")
    locks = [Path(path.decode()) for path in paths if path]
    if not locks:
        raise RuntimeError("no committed Cargo lockfiles found")
    failed = False
    for lock in locks:
        print(f"Auditing {lock}", flush=True)
        result = subprocess.run(["cargo", "audit", "--no-fetch", "--no-yanked", "--file", str(lock)], cwd=ROOT)
        failed |= result.returncode != 0
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
