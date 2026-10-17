#!/usr/bin/env python3
"""Reference-only replay of the historical Python client for Rust-client comparisons.

The primary benchmark is benchmark/runner. This adapter keeps the old request
engine, replays a Rust report's exact trials, uses its operation ordering, and
checks server hashes and request-order hashes. It is not the new load generator.
"""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import time

import compare
import run

MASK = (1 << 64) - 1


def operation_order(operations: int, seed: int) -> list[int]:
    """Match the Rust runner's SplitMix64 Fisher-Yates order outside timing."""
    state = seed & MASK

    def next_u64() -> int:
        nonlocal state
        state = (state + 0x9E3779B97F4A7C15) & MASK
        value = state
        value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK
        return value ^ (value >> 31)

    order = list(range(operations))
    for index in range(operations - 1, 0, -1):
        width = index + 1
        threshold = ((-width) & MASK) % width
        sample = next_u64()
        while sample < threshold:
            sample = next_u64()
        other = sample % width
        order[index], order[other] = order[other], order[index]
    return order


def cpu_seconds(pid: int) -> float | None:
    try:
        threads = list(Path(f"/proc/{pid}/task").iterdir())
        return sum(int((thread / "schedstat").read_text().split()[0]) for thread in threads) / 1e9
    except (OSError, ValueError, IndexError):
        return None


def affinity(pid: int) -> str | None:
    try:
        return next(line.split(":", 1)[1].strip()
                    for line in Path(f"/proc/{pid}/status").read_text().splitlines()
                    if line.startswith("Cpus_allowed_list:"))
    except (OSError, StopIteration):
        return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    reference = json.loads(args.reference.read_text())
    supported = {"hello", "auth", "posts", "di-graph-noop", "di-graph-traverse"}
    if any(item["scenario"] not in supported for item in reference.get("results", [])):
        parser.error("reference replay supports comparison/DI scenarios, not HTTP fault stress cases")
    if reference.get("passed") is not True:
        parser.error("reference must be a successful Rust benchmark report")
    for key, value in reference["load_generator"]["server_environment"].items():
        if os.environ.get(key) != value:
            parser.error(f"{key} must match the reference")
    if affinity(os.getpid()) != reference["load_generator"]["client_cpu_affinity"]:
        parser.error("client CPU affinity must match the reference")
    if os.environ.get("BENCH_SERVER_CPUS") != reference["load_generator"]["requested_server_cpu_affinity"]:
        parser.error("BENCH_SERVER_CPUS must match the reference")

    # Only adapt order generation and process affinity. The historical Client,
    # request timing, body checks and ThreadPoolExecutor measurement stay intact.
    run.operation_order = operation_order
    original_target_command = compare.target_command

    def target_command(*arguments, **keywords):
        command, directory = original_target_command(*arguments, **keywords)
        cpus = os.environ.get("BENCH_SERVER_CPUS")
        if cpus:
            command = ["/usr/bin/taskset", "-c", cpus, *command]
        return command, directory

    compare.target_command = target_command
    results = {}
    expected = {(item["scenario"], item["framework"], item["dependency_count"]): item
                for item in reference["results"]}
    for trial in reference["execution_order"]:
        scenario, framework, index, count = (trial[key] for key in
                                             ("scenario", "framework", "run", "dependency_count"))
        key = scenario, framework, count
        rust_run = next(item for item in expected[key]["runs"] if item["run_index"] == index)
        binary = run.ROOT / rust_run["artifact"]["path"]
        if hashlib.sha256(binary.read_bytes()).hexdigest() != rust_run["artifact"]["sha256"]:
            raise RuntimeError("server binary changed since Rust benchmark")
        item = results.setdefault(key, {"scenario": scenario, "framework": framework,
                                       "dependency_count": count, "runs": [], "contract_errors": []})
        url = None
        if scenario == "posts":
            variable = "BENCH_FURNACE_DATABASE_URL" if framework.startswith("furnace-rs") else f"BENCH_{framework.upper()}_DATABASE_URL"
            url = os.environ.get(variable) or os.environ.get("BENCH_DATABASE_URL")
            if not url:
                raise RuntimeError(f"{variable} is required")
        print(f"Python reference: {framework}/{scenario} run {index + 1}", flush=True)
        with compare.server(framework, scenario, url, dependency_count=count, include_startup=True) as (port, log, startup):
            errors = compare.contract_checks(scenario, port, count)
            item["contract_errors"].extend(errors)
            if errors:
                raise RuntimeError(f"contract check failed; log: {log}")
            if scenario == "auth" and index == 0:
                item["body_limit"] = compare.body_limit_probe(port)
            children = Path(f"/proc/self/task/{os.getpid()}/children").read_text().split()
            if len(children) != 1:
                raise RuntimeError("expected exactly one owned server child")
            server_pid = int(children[0])
            if affinity(server_pid) != rust_run["server_cpu_affinity"]:
                raise RuntimeError("server CPU affinity differs from Rust reference")
            warmup = compare.measure(scenario, port, rust_run["warmup"]["operations"],
                                     rust_run["concurrency"], seed=trial["request_order_seed"],
                                     expected_dependency_count=count)
            if not warmup["passed"]:
                raise RuntimeError(f"warmup failed; log: {log}")
            before = time.process_time()
            server_before = cpu_seconds(server_pid)
            measured = compare.measure(scenario, port, rust_run["operations"], rust_run["concurrency"],
                                       seed=trial["request_order_seed"], expected_dependency_count=count)
            cpu = time.process_time() - before
            server_after = cpu_seconds(server_pid)
            if measured["request_order_sha256"] != rust_run["request_order_sha256"]:
                raise RuntimeError("Python and Rust operation order differs")
            responses_per_operation = 5 if scenario == "posts" else 2 if scenario == "auth" else 1
            if not measured["passed"] or measured["http_responses"] != rust_run["operations"] * responses_per_operation:
                raise RuntimeError(f"measurement failed or responses missing; log: {log}")
            server_cpu = None if server_before is None or server_after is None else server_after - server_before
            measured.update({"run_index": index, "startup_seconds": startup, "artifact": rust_run["artifact"],
                             "client_cpu_seconds": cpu,
                             "client_cpu_percent_one_core": cpu / measured["duration_seconds"] * 100,
                             "server_cpu_seconds": server_cpu,
                             "server_cpu_percent_one_core": None if server_cpu is None else server_cpu / measured["duration_seconds"] * 100,
                             "server_cpu_affinity": affinity(server_pid), "warmup": warmup})
            item["runs"].append(measured)
    for item in results.values():
        item["summary"] = compare.median_summary(item["runs"])
        item["passed"] = len(item["runs"]) == reference["measured_runs"] and item["summary"]["passed"]
    report = {"timestamp_utc": datetime.now(timezone.utc).isoformat(), "python": platform.python_version(),
              "git_head": reference["git_head"], "versions": reference["versions"], "profile": reference["profile"],
              "measured_runs": reference["measured_runs"], "execution_order": reference["execution_order"],
              "random_seed": reference["random_seed"], "rust_reference_sha256": hashlib.sha256(args.reference.read_bytes()).hexdigest(),
              "load_generator": {"language": "Python", "http_client": "stdlib http.client + ThreadPoolExecutor",
                                 "client_cpu_affinity": affinity(os.getpid()),
                                 "cpu_measurement": "client: time.process_time (includes worker creation/order bookkeeping); server: summed live-thread schedstat",
                                 "adaptations": "same fresh-process trial schedule, warmup sizes, operation seed/order and server affinity as Rust; historical request engine retained",
                                 "source_sha256": {name: hashlib.sha256((Path(__file__).parent / name).read_bytes()).hexdigest()
                                                   for name in ("run.py", "compare.py", "python_reference.py")}},
              "results": list(results.values()), "passed": all(item["passed"] for item in results.values())}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
