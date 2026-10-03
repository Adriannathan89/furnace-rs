#!/usr/bin/env python3
"""Compare isolated HTTP runtime regression tests before and after hardening."""

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
SERVER = Path("crates/furnace-rs-common/src/server.rs")
BODY = Path("crates/furnace-rs-common/src/server/body.rs")
HEADERS = "incomplete_headers_expire_without_stopping_the_server"
UPLOAD = "stalled_request_body_expires_and_server_remains_available"
CONTROLS = (
    HEADERS,
    UPLOAD,
    "stalled_http2_body_does_not_stop_other_requests",
    "idle_connections_and_partial_protocol_prefaces_expire",
    "header_deadline_does_not_cancel_handlers_or_graceful_drain",
    "body_deadline_preserves_valid_json_and_body_size_rejections",
    "header_deadline_preserves_http2_requests",
    "progressing_frames_reset_the_idle_deadline",
    "application_pauses_do_not_expire_ready_frames",
)


def revision(value: str) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "--verify", f"{value}^{{commit}}"], cwd=ROOT, text=True
    ).strip()


def fixture(source: str, name: str) -> str:
    marker = f"    #[tokio::test]\n    async fn {name}("
    start = source.index(marker)
    end = source.index("    #[tokio::test]", start + len(marker))
    return source[start:end]


def fingerprint(root: Path) -> str:
    digest = hashlib.sha256()
    for path in (SERVER, BODY, Path("Cargo.toml"), Path("Cargo.lock"), Path("crates/furnace-rs-common/Cargo.toml")):
        if (root / path).exists():
            digest.update(str(path).encode())
            digest.update((root / path).read_bytes())
    return digest.hexdigest()


def run(root: Path, git_revision: str, test: str, expected_failure: str | None = None) -> dict:
    command = ["cargo", "test", "--offline", "--locked", "-p", "furnace-rs-common",
               "--all-features", "--lib", test, "--", "--nocapture"]
    started = time.monotonic()
    process = subprocess.run(command, cwd=root, env={**os.environ, "CARGO_NET_OFFLINE": "true"},
                             capture_output=True, text=True, timeout=300)
    output = process.stdout + process.stderr
    summaries = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;.*?finished in ([\d.]+)s", output)
    if len(summaries) != 1:
        raise ValueError("expected exactly one completed Rust test result")
    status, passed, failed, elapsed = summaries[0]
    passed, failed = int(passed), int(failed)
    if expected_failure is not None:
        verified = (process.returncode == 101 and status == "FAILED" and passed == 0 and failed == 1
                    and expected_failure in output and "Err(Elapsed(()))" in output
                    and f"test server::tests::{test} ... FAILED" in output)
    else:
        verified = (process.returncode == 0 and status == "ok" and failed == 0
                    and all(re.search(rf"^test server::(?:tests|body::tests)::{name} \.\.\. ok$", output, re.MULTILINE)
                            for name in CONTROLS))
    return {"git_revision": git_revision, "source_sha256": fingerprint(root),
            "command": command, "cargo_exit_code": process.returncode,
            "test_runtime_seconds": float(elapsed),
            "elapsed_seconds_including_build": round(time.monotonic() - started, 3),
            "tests_passed": passed, "tests_failed": failed,
            "expected_outcome_verified": bool(verified), "output": output}


def baseline(directory: Path, commit: str, test_fixture: str) -> Path:
    directory.mkdir()
    archive = subprocess.check_output(["git", "archive", commit], cwd=ROOT)
    with tarfile.open(fileobj=io.BytesIO(archive)) as files:
        files.extractall(directory, filter="data")
    source = (directory / SERVER).read_text()
    end = source.rfind("\n}")
    if end < 0 or source[end:].strip() != "}":
        raise ValueError("cannot locate the baseline test module's closing brace")
    (directory / SERVER).write_text(source[:end] + "\n" + test_fixture + source[end:])
    return directory


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before-headers", default="af49c24")
    parser.add_argument("--before-body", default="7a19295")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = {"timestamp_utc": datetime.now(timezone.utc).isoformat(),
              "build_profile": "debug", "database_required": False,
              "observation_window_seconds": 12,
              "configured_header_deadline_seconds": 10,
              "configured_body_idle_deadline_seconds": 10,
              "method": "isolated regression tests; no connection flood or resource exhaustion",
              "baseline_method": "temporary git archives with current regression fixtures added; production code unchanged",
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    try:
        source = (ROOT / SERVER).read_text()
        before_headers, before_body, after = revision(args.before_headers), revision(args.before_body), revision("HEAD")
        with tempfile.TemporaryDirectory(prefix="furnace-http-security-") as temporary:
            directory = Path(temporary)
            report["headers_before"] = run(baseline(directory / "headers", before_headers, fixture(source, HEADERS)),
                before_headers, HEADERS, "incomplete headers retained a connection past the deadline")
            report["body_before"] = run(baseline(directory / "body", before_body, fixture(source, UPLOAD)),
                before_body, UPLOAD, "stalled upload did not expire")
        report["after"] = run(ROOT, after, "server::")
        report["passed"] = all(report[name]["expected_outcome_verified"] for name in ("headers_before", "body_before", "after"))
    except (OSError, subprocess.SubprocessError, ValueError) as error:
        report["passed"] = False
        report["failure"] = f"HTTP runtime comparison could not complete ({type(error).__name__}): {error}"
    output = json.dumps(report, indent=2) + "\n"
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
