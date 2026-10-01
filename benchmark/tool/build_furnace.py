#!/usr/bin/env python3
"""Build staged FURNACE examples against this checkout without editing source locks."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

from run import ROOT


EXAMPLES = ("hello-world", "posts-crud", "protected-route")


def stage_example(source: Path, destination: Path, lockfile: Path | None = None) -> Path:
    shutil.copytree(source, destination, ignore=shutil.ignore_patterns("Cargo.lock", "target", ".env", ".env.*"))
    manifest = destination / "Cargo.toml"
    text = manifest.read_text()
    data = tomllib.loads(text)
    tables = [data, *data.get("target", {}).values(), data.get("workspace", {})]
    for table in tables:
        for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
            for dependency in table.get(kind, {}).values():
                if isinstance(dependency, dict) and "path" in dependency:
                    relative = dependency["path"]
                    absolute = str((source / relative).resolve())
                    pattern = r"\bpath\s*=\s*([\"'])" + re.escape(relative) + r"\1"
                    text = re.sub(pattern, lambda _: "path = " + json.dumps(absolute), text)
    manifest.write_text(text)
    if lockfile is not None:
        shutil.copy2(lockfile, destination / "Cargo.lock")
    return destination


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--example", action="append", choices=EXAMPLES)
    parser.add_argument("--offline", action="store_true", help="use cached Cargo dependencies only")
    args = parser.parse_args()
    target = ROOT / "benchmark" / "targets" / "furnace-rs" / "target"
    locks = ROOT / "benchmark" / "targets" / "furnace-rs" / "locks"
    locks.mkdir(parents=True, exist_ok=True)
    examples = args.example or EXAMPLES
    with tempfile.TemporaryDirectory(prefix="furnace-rs-bench-build-") as directory:
        for name in examples:
            lockfile = locks / f"{name}.lock"
            staged = stage_example(
                ROOT / "example" / name, Path(directory) / name,
                lockfile if lockfile.is_file() else None,
            )
            command = [
                "cargo",
                "--config", f'patch.crates-io.furnace-rs.path="{ROOT / "crates" / "furnace-rs"}"',
            ]
            if name == "posts-crud":
                command.extend([
                    "--config", f'patch.crates-io.furnace-rs-persistence.path="{ROOT / "crates" / "furnace-rs-persistence"}"',
                ])
            command.extend([
                "build", "--release", "--manifest-path", str(staged / "Cargo.toml"),
                "--target-dir", str(target),
            ])
            if args.offline:
                command.append("--offline")
            if lockfile.is_file():
                command.append("--locked")
            subprocess.run(command, check=True)
            if not lockfile.is_file():
                shutil.copy2(staged / "Cargo.lock", lockfile)


if __name__ == "__main__":
    main()
