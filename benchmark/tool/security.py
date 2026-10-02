#!/usr/bin/env python3
"""Security regression workload against the locally built FURNACE auth example."""

from __future__ import annotations

import argparse
import base64
from datetime import datetime, timezone
import hashlib
import hmac
import http.client
import json
from pathlib import Path
import platform
import time

from compare import server, target_command
from run import (
    Client, DEMO_PASSWORD, DEMO_SECRET, DEMO_USERNAME, Measurements, Response,
    git_head, json_body, run_workers, work_chunks,
)


PROFILES = {"smoke": (2, 4), "stress": (200, 16), "extended": (1000, 32)}
REJECTION = {"error": {"code": "unauthorized", "message": "authentication was rejected"}}
PROFILE = {"id": 1, "username": DEMO_USERNAME}


def encoded(value: bytes) -> str:
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def signed_token(header: dict, claims: dict, *, raw_claims: str | None = None) -> str:
    """Sign adversarial fixtures with the benchmark-only key, never a deployment key."""
    payload = raw_claims if raw_claims is not None else json.dumps(claims)
    message = encoded(json.dumps(header).encode()) + "." + encoded(payload.encode())
    digest = hashlib.sha512 if header.get("alg") == "HS512" else hashlib.sha256
    signature = hmac.new(DEMO_SECRET.encode(), message.encode(), digest).digest()
    return message + "." + encoded(signature)


def probes(token: str) -> list[tuple[str, list[tuple[str, str]], int]]:
    def bearer(value: str) -> list[tuple[str, str]]:
        return [("Authorization", value)]

    header = {"alg": "HS256", "typ": "furnace_rs-access+jwt"}
    now = int(time.time())
    claims = {"user_id": 1, "sub": "1", "iat": now, "exp": now + 300, "token_use": "access"}
    expired = signed_token(header, {**claims, "iat": now - 600, "exp": now - 300})
    future = signed_token(header, {**claims, "nbf": now + 3600})
    refresh = signed_token({**header, "typ": "furnace_rs-refresh+jwt"}, {**claims, "token_use": "refresh"})
    wrong_algorithm = signed_token({**header, "alg": "HS512"}, claims)
    missing_expiration = signed_token(header, {key: value for key, value in claims.items() if key != "exp"})
    oversized = signed_token(header, {**claims, "padding": "a" * 8193})
    duplicate_expiration = signed_token(header, claims, raw_claims=
        '{"exp":0,' + json.dumps(claims)[1:])
    unsigned = encoded(json.dumps({**header, "alg": "none"}).encode()) + "." + encoded(json.dumps(claims).encode()) + "."
    forged = signed_token(header, claims)
    message, signature = forged.rsplit(".", 1)
    forged = message + "." + ("A" if signature[0] != "A" else "B") + signature[1:]
    return [
        ("valid Bearer", bearer(f"Bearer {token}"), 200),
        ("lowercase scheme", bearer(f"bearer {token}"), 200),
        ("mixed case and multiple spaces", bearer(f"bEaReR   {token}"), 200),
        ("tab separator", bearer(f"Bearer\t{token}"), 401),
        ("space then tab separator", bearer(f"Bearer \t{token}"), 401),
        ("tab then space separator", bearer(f"Bearer\t {token}"), 401),
        ("duplicate valid then invalid", bearer(f"Bearer {token}") + bearer("Bearer bad.token.value"), 401),
        ("duplicate invalid then valid", bearer("Bearer bad.token.value") + bearer(f"Bearer {token}"), 401),
        ("duplicate valid credentials", bearer(f"Bearer {token}") * 2, 401),
        ("comma combined credentials", bearer(f"Bearer {token}, Bearer {token}"), 401),
        ("extra credential", bearer(f"Bearer {token} extra"), 401),
        ("wrong scheme", bearer(f"Basic {token}"), 401),
        ("missing credentials", [], 401),
        ("empty credential", bearer("Bearer "), 401),
        ("forged signature", bearer(f"Bearer {forged}"), 401),
        ("unsigned token", bearer(f"Bearer {unsigned}"), 401),
        ("algorithm mismatch", bearer(f"Bearer {wrong_algorithm}"), 401),
        ("expired token", bearer(f"Bearer {expired}"), 401),
        ("future not before", bearer(f"Bearer {future}"), 401),
        ("refresh token on access route", bearer(f"Bearer {refresh}"), 401),
        ("missing expiration", bearer(f"Bearer {missing_expiration}"), 401),
        ("duplicate expiration", bearer(f"Bearer {duplicate_expiration}"), 401),
        ("oversized token", bearer(f"Bearer {oversized}"), 401),
    ]


def request_headers(client: Client, stats: Measurements, label: str,
                    headers: list[tuple[str, str]]) -> Response | None:
    """Preserve duplicate fields on the wire instead of collapsing them into a dict."""
    start = time.perf_counter_ns()
    try:
        client.connection.putrequest("GET", "/auth/me")
        for name, value in headers:
            client.connection.putheader(name, value)
        client.connection.endheaders()
        reply = client.connection.getresponse()
        body = reply.read()
        return Response(reply.status, body,
                        {name.lower(): value for name, value in reply.getheaders()},
                        (time.perf_counter_ns() - start) / 1_000_000)
    except (OSError, http.client.HTTPException):
        stats.error(f"{label}: transport failure")
        client.reset()
        return None


def security_case(profile: str, port: int, token: str) -> dict[str, object]:
    variants = probes(token)
    rounds, concurrency = PROFILES[profile]
    operations = len(variants) * rounds
    chunks = work_chunks(operations, concurrency)
    offsets = [sum(chunks[:worker]) for worker in range(len(chunks))]

    def check(client: Client, stats: Measurements, label: str,
              headers: list[tuple[str, str]], status: int) -> None:
        response = request_headers(client, stats, label, headers)
        expected = PROFILE if status == 200 else REJECTION
        if stats.check(label, response, status, lambda body: json_body(body) == expected):
            if status == 401 and response.headers.get("www-authenticate") != "Bearer":
                stats.error(f"{label}: missing Bearer challenge")

    def operation(client: Client, stats: Measurements, worker: int, index: int) -> None:
        label, headers, status = variants[(offsets[worker] + index) % len(variants)]
        check(client, stats, label, headers, status)
        check(client, stats, "valid follow-up", [("Authorization", f"Bearer {token}")], 200)

    result = run_workers("security-auth", operations, concurrency, port, operation)
    result["probe_counts"] = {label: rounds for label, _headers, _status in variants}
    result["expected_rejections"] = sum(status == 401 for _label, _headers, status in variants) * rounds
    return result


def login(port: int) -> str:
    client = Client(port)
    try:
        stats = Measurements()
        response = client.request(stats, "security login", "POST", "/auth/login",
            json.dumps({"username": DEMO_USERNAME, "password": DEMO_PASSWORD}).encode(),
            {"Content-Type": "application/json"})
        if response is None or response.status != 200:
            raise RuntimeError("could not obtain benchmark JWT")
        token = json_body(response.body).get("access_token")
        if not isinstance(token, str) or not token:
            raise RuntimeError("login did not return a token")
        return token
    finally:
        client.close()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, default="smoke")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "git_head": git_head(), "profile": args.profile, "platform": platform.platform(),
        "framework": "furnace-rs", "binary_profile": "release",
        "binary": target_command("furnace-rs", "auth")[0][0],
        "build_command": "python3 benchmark/tool/build_furnace.py --offline --example protected-route",
    }
    try:
        with Path(report["binary"]).open("rb") as binary:
            report["binary_sha256"] = hashlib.file_digest(binary, "sha256").hexdigest()
        with server("furnace-rs", "auth", None) as (port, log_path):
            result = security_case(args.profile, port, login(port))
            report["cases"] = [result]
            report["passed"] = result["passed"]
            if not result["passed"]:
                report["server_log"] = str(log_path)
                raise RuntimeError("security regression contract failed")
    except (OSError, RuntimeError, ValueError, KeyError) as error:
        report["passed"] = False
        report["failure"] = str(error)
    output = json.dumps(report, indent=2)
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
