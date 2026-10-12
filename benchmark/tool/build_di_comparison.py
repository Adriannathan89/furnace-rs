#!/usr/bin/env python3
"""Build old/current FURNACE binaries and synthetic DI graph workloads."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

from build_furnace import ROOT, stage_example
from di_graph import parse_dependency_counts, render_source


BASELINE_REVISION = "8c08bba"
TARGET_ROOT = ROOT / "benchmark" / "targets" / "furnace-rs" / "target"
EXAMPLES = {
    "hello-world": "furnace-rs-example-hello-world",
    "protected-route": "furnace-rs-example-protected-route",
}


def baseline_lock_path(example: str) -> Path:
    if example not in EXAMPLES:
        raise ValueError(f"unsupported baseline example: {example}")
    return ROOT / "benchmark" / "results" / "2026-10-11-di-runtime-locks" / f"1.0.2-{example}.lock"


def third_party_packages(lockfile: Path) -> frozenset[tuple[str, str]]:
    packages = tomllib.loads(lockfile.read_text())["package"]
    return frozenset(
        (package["name"], package["version"])
        for package in packages
        if not package["name"].startswith("furnace-rs")
    )


def archive_revision(revision: str, destination: Path) -> Path:
    archive = subprocess.run(
        ["git", "archive", revision], cwd=ROOT, check=True, capture_output=True,
    ).stdout
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as tar:
        tar.extractall(destination, filter="data")
    return destination


def binary_path(version: str, node_count: int) -> Path:
    return (
        TARGET_ROOT / "di-comparison" / "graph" / version / f"nodes-{node_count}"
        / EXAMPLES["hello-world"]
    )


def cargo_build(stage: Path, target: Path, offline: bool, patch_root: Path | None = None) -> Path:
    command = ["cargo"]
    if patch_root is not None:
        command.extend([
            "--config", f'patch.crates-io.furnace-rs.path="{patch_root / "crates" / "furnace-rs"}"',
        ])
    command.extend([
        "build", "--release", "--locked", "--manifest-path", str(stage / "Cargo.toml"),
        "--target-dir", str(target),
    ])
    if offline:
        command.append("--offline")
    subprocess.run(command, cwd=ROOT, check=True)
    return target / "release" / EXAMPLES["hello-world"]


def build_baseline_examples(tree: Path, output: Path, offline: bool) -> None:
    target = TARGET_ROOT / "di-comparison" / "baseline-build"
    for name, binary in EXAMPLES.items():
        source = tree / "example" / name
        with tempfile.TemporaryDirectory(prefix=f"furnace-baseline-{name}-") as temporary:
            stage = stage_example(source, Path(temporary) / name, baseline_lock_path(name))
            command = [
                "cargo", "build", "--release", "--locked", "--manifest-path",
                str(stage / "Cargo.toml"), "--target-dir", str(target),
            ]
            if offline:
                command.append("--offline")
            subprocess.run(command, cwd=ROOT, check=True)
            destination = output / binary
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(target / "release" / binary, destination)


def build_graphs(
    tree: Path, version: str, node_counts: tuple[int, ...], offline: bool, current: bool,
) -> None:
    source = tree / "example" / "hello-world"
    target = TARGET_ROOT / "di-comparison" / f"{version}-graph-build"
    lockfile = (
        ROOT / "benchmark" / "targets" / "furnace-rs" / "locks" / "hello-world.lock"
        if current else baseline_lock_path("hello-world")
    )
    with tempfile.TemporaryDirectory(prefix=f"furnace-{version}-di-graph-") as temporary:
        stage = stage_example(source, Path(temporary) / "hello-world", lockfile)
        for node_count in node_counts:
            (stage / "src" / "main.rs").write_text(
                render_source(node_count, declarative=current)
            )
            binary = cargo_build(stage, target, offline, patch_root=tree if current else None)
            destination = binary_path(version, node_count)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(binary, destination)


def build_reference_targets(offline: bool) -> None:
    axum_target = ROOT / "benchmark" / "targets" / "axum" / "target"
    axum_command = [
        "cargo", "build", "--release", "--locked", "--manifest-path",
        str(ROOT / "benchmark" / "targets" / "axum" / "Cargo.toml"),
        "--target-dir", str(axum_target),
    ]
    if offline:
        axum_command.append("--offline")
    subprocess.run(axum_command, cwd=ROOT, check=True)

    fiber = ROOT / "benchmark" / "targets" / "fiber"
    (fiber / "bin").mkdir(parents=True, exist_ok=True)
    go_command = ["go", "build", "-o", str(fiber / "bin" / "furnace-rs-bench-fiber"), "."]
    if offline:
        go_environment = {**os.environ, "GOPROXY": "off"}
    else:
        go_environment = None
    subprocess.run(go_command, cwd=fiber, env=go_environment, check=True)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dependency-counts", default="1,16,64,256",
        help="comma-separated unique positive graph sizes (default: 1,16,64,256)",
    )
    parser.add_argument("--baseline-revision", default=BASELINE_REVISION)
    parser.add_argument("--online", action="store_true", help="allow Cargo to access the registry")
    args = parser.parse_args()
    try:
        node_counts = parse_dependency_counts(args.dependency_counts)
    except ValueError as error:
        parser.error(str(error))

    offline = not args.online
    subprocess.run([
        sys.executable, str(ROOT / "benchmark" / "tool" / "build_furnace.py"),
        "--example", "hello-world", "--example", "protected-route",
        *( ["--offline"] if offline else [] ),
    ], cwd=ROOT, check=True)

    baseline_output = TARGET_ROOT / "di-comparison" / "baseline-1.0.2"
    baseline_lock_hashes = {
        name: sha256_file(baseline_lock_path(name)) for name in EXAMPLES
    }
    with tempfile.TemporaryDirectory(prefix="furnace-di-baseline-") as temporary:
        tree = archive_revision(args.baseline_revision, Path(temporary) / "source")
        build_baseline_examples(tree, baseline_output, offline)
        build_graphs(tree, "1.0.2", node_counts, offline, current=False)

    build_graphs(ROOT, "beta", node_counts, offline, current=True)
    build_reference_targets(offline)
    current_lock_dir = ROOT / "benchmark" / "targets" / "furnace-rs" / "locks"
    manifest = {
        "built_at_utc": datetime.now(timezone.utc).isoformat(),
        "baseline_revision": subprocess.run(
            ["git", "rev-parse", args.baseline_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip(),
        "current_revision": subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip(),
        "dependency_counts": node_counts,
        "third_party_lock_versions_match": all(
            third_party_packages(baseline_lock_path(name))
            == third_party_packages(current_lock_dir / f"{name}.lock")
            for name in EXAMPLES
        ),
        "source_lock_sha256": {
            "baseline_1.0.2": baseline_lock_hashes,
            "current_beta": {
                name: sha256_file(current_lock_dir / f"{name}.lock")
                for name in EXAMPLES
            },
        },
    }
    manifest_path = TARGET_ROOT / "di-comparison" / "build-manifest.json"
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
