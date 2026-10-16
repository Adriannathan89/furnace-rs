#!/usr/bin/env python3
"""Compare randomized HTTP workloads and startup across FURNACE DI versions.

Legacy HTTP performance client. Use benchmark/runner (Rust + reqwest) for
new performance measurements. Retained for historical and correctness tools.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import secrets
import statistics
import subprocess
import tomllib

from compare import (
    build_run_schedule,
    body_limit_probe,
    contract_checks,
    measure,
    median_summary,
    server,
    warmup_size,
)
from di_graph import parse_dependency_counts
from run import PROFILES, ROOT


HTTP_FRAMEWORKS = ("furnace-rs-1.0.2", "furnace-rs-beta", "axum", "fiber")
FURNACE_FRAMEWORKS = ("furnace-rs-1.0.2", "furnace-rs-beta")
HTTP_SCENARIOS = ("hello", "auth")
BUILD_MANIFEST = ROOT / "benchmark" / "targets" / "furnace-rs" / "target" / "di-comparison" / "build-manifest.json"


def trial_seed(seed: int, scenario: str, framework: str, run_index: int,
               dependency_count: int | None = None) -> int:
    material = f"{seed}:{scenario}:{framework}:{run_index}:{dependency_count}".encode()
    return int.from_bytes(hashlib.sha256(material).digest()[:8], "big")


def dependency_counts_are_built(requested: tuple[int, ...], built: tuple[int, ...]) -> bool:
    return set(requested).issubset(built)


def randomized_schedule(
    scenarios: tuple[str, ...], frameworks: tuple[str, ...], runs: int,
    dependency_counts: tuple[int, ...], seed: int,
) -> list[tuple[str, str, int, int | None]]:
    schedule = [(*trial, None) for trial in build_run_schedule(scenarios, frameworks, runs, seed)]
    schedule.extend(
        (scenario, framework, run_index, node_count)
        for scenario in ("di-graph-noop", "di-graph-traverse")
        for node_count in dependency_counts
        for framework in FURNACE_FRAMEWORKS
        for run_index in range(runs)
    )
    random.Random(seed ^ 0xD1C0FFEE).shuffle(schedule)
    return schedule


def result_key(scenario: str, framework: str, dependency_count: int | None) -> tuple[str, str, int | None]:
    return scenario, framework, dependency_count


def summarize(runs: list[dict[str, object]]) -> dict[str, object]:
    summary = median_summary(runs)
    startup_samples = [float(run["startup_seconds"]) for run in runs]
    summary["median_startup_seconds"] = round(statistics.median(startup_samples), 4)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=tuple(PROFILES), default="stress")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--seed", type=int, help="reproducible randomization seed")
    parser.add_argument("--dependency-counts", default="1,16,64,256")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--framework", action="append", choices=HTTP_FRAMEWORKS,
        help="HTTP targets to compare (repeatable); dependency graph always compares both FURNACE versions",
    )
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    try:
        dependency_counts = parse_dependency_counts(args.dependency_counts)
    except ValueError as error:
        parser.error(str(error))
    if not BUILD_MANIFEST.is_file():
        parser.error("build targets first with benchmark/tool/build_di_comparison.py")
    build_manifest = json.loads(BUILD_MANIFEST.read_text())
    built_counts = tuple(build_manifest.get("dependency_counts", ()))
    if not dependency_counts_are_built(dependency_counts, built_counts):
        parser.error("requested dependency counts were not built; rebuild targets for these sizes")

    seed = args.seed if args.seed is not None else secrets.randbits(64)
    frameworks = tuple(args.framework or HTTP_FRAMEWORKS)
    schedule = randomized_schedule(HTTP_SCENARIOS, frameworks, args.runs, dependency_counts, seed)
    items: dict[tuple[str, str, int | None], dict[str, object]] = {}
    execution_order: list[dict[str, object]] = []

    for scenario, framework, run_index, dependency_count in schedule:
        key = result_key(scenario, framework, dependency_count)
        item = items.setdefault(key, {
            "scenario": scenario,
            "framework": framework,
            "dependency_count": dependency_count,
            "runs": [],
            "contract_errors": [],
        })
        order_seed = trial_seed(seed, scenario, framework, run_index, dependency_count)
        execution_order.append({
            "scenario": scenario,
            "framework": framework,
            "run": run_index,
            "dependency_count": dependency_count,
            "request_order_seed": order_seed,
        })
        try:
            with server(
                framework, scenario, None, dependency_count=dependency_count,
                include_startup=True,
            ) as (port, server_log, startup_seconds):
                errors = contract_checks(scenario, port, dependency_count)
                item["contract_errors"].extend(errors)
                if errors:
                    raise RuntimeError(f"contract checks failed; server log: {server_log}")
                if scenario == "auth" and run_index == 0:
                    item["body_limit"] = body_limit_probe(port)

                profile_key = "jwt" if scenario == "auth" else "hello"
                operations, concurrency = PROFILES[args.profile][profile_key]
                warmup_operations, warmup_concurrency = warmup_size(operations, concurrency)
                warmup = measure(
                    scenario, port, warmup_operations, warmup_concurrency,
                    expected_dependency_count=dependency_count,
                )
                if not warmup["passed"]:
                    raise RuntimeError(f"warmup failed: {warmup['error_examples']}; server log: {server_log}")
                measured = measure(
                    scenario, port, operations, concurrency, seed=order_seed,
                    expected_dependency_count=dependency_count,
                )
                measured["run_index"] = run_index
                measured["startup_seconds"] = round(startup_seconds, 4)
                item["runs"].append(measured)
                if not measured["passed"]:
                    raise RuntimeError(f"measured run failed: {measured['error_examples']}; server log: {server_log}")
        except Exception as error:
            item.setdefault("errors", []).append(f"{type(error).__name__}: {error}")

    results = []
    for item in items.values():
        runs = item["runs"]
        if runs:
            item["summary"] = summarize(runs)
        results.append(item)

    report = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "logical_cpus": os.cpu_count(),
        "python": platform.python_version(),
        "rustc": subprocess.run(["rustc", "--version"], check=True, capture_output=True, text=True).stdout.strip(),
        "git_head": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip(),
        "versions": {
            "furnace-rs-1.0.2": "1.0.2",
            "furnace-rs-beta": tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"],
            "axum": "0.8.9",
            "fiber": "2.52.15",
        },
        "build_provenance": build_manifest,
        "profile": args.profile,
        "measured_runs": args.runs,
        "random_seed": seed,
        "request_randomization": {
            "target_order": "one randomized process/workload trial per schedule entry",
            "operation_order": "independent transactions shuffled per worker; request sequences within a transaction remain ordered",
            "seed_derivation": "SHA-256 of master seed, workload, framework, run index, and dependency count",
        },
        "dependency_graph": {
            "counts": dependency_counts,
            "shape": "balanced binary tree; every node is registered and reachable from the root controller",
            "request_modes": {
                "di-graph-noop": "returns a constant without traversing the graph, isolating request-path impact of a larger constructed graph",
                "di-graph-traverse": "recursively visits the complete injected graph and returns its node count; each node performs an observable atomic visit so release optimization cannot fold the traversal into a constant",
            },
        },
        "execution_order": execution_order,
        "results": results,
    }
    report["passed"] = all(
        len(item["runs"]) == args.runs
        and "errors" not in item
        and all(bool(run["passed"]) for run in item["runs"])
        for item in results
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
