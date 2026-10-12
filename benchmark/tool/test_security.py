"""Check that the security benchmark detects acceptance and unsafe rejections."""

import base64
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
import unittest

import security


class SecurityBenchmarkTests(unittest.TestCase):
    def run_case(self, mode):
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                values = self.headers.get_all("Authorization") or []
                valid = len(values) == 1 and values[0] in (
                    "Bearer test-token", "bearer test-token", "bEaReR   test-token",
                )
                if mode == "accept-tab" and values == ["Bearer\ttest-token"]:
                    valid = True
                status = 200 if valid else 401
                body = (b'{"id":1,"username":"demo"}' if valid else
                        b'{"error":{"code":"unauthorized","message":"authentication was rejected"}}')
                if mode == "leak" and not valid:
                    body = b'{"error":{"code":"unauthorized","message":"private details"}}'
                self.send_response(status)
                if not valid and mode != "no-challenge":
                    self.send_header("WWW-Authenticate", "Bearer")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
            thread = Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                return security.security_case("smoke", server.server_port, "test-token")
            finally:
                server.shutdown()
                thread.join(timeout=2)

    def test_all_probes_and_positive_controls_are_measured(self):
        result = self.run_case("safe")
        self.assertTrue(result["passed"], result)
        self.assertEqual(result["http_responses"], result["operations"] * 2)
        self.assertEqual(sum(result["probe_counts"].values()), result["operations"])
        self.assertTrue(all(count == 2 for count in result["probe_counts"].values()))
        self.assertEqual(result["status_counts"]["401"], result["expected_rejections"])

    def test_accepting_tab_separated_valid_token_fails_benchmark(self):
        result = self.run_case("accept-tab")
        self.assertFalse(result["passed"])
        self.assertTrue(any("tab separator" in error for error in result["error_examples"]))

    def test_leaking_rejection_details_fails_benchmark(self):
        self.assertFalse(self.run_case("leak")["passed"])

    def test_missing_bearer_challenge_fails_benchmark(self):
        self.assertFalse(self.run_case("no-challenge")["passed"])

    def test_oversized_fixture_has_a_valid_signature_and_existing_user(self):
        import json
        import time

        _label, headers, status = next(probe for probe in security.probes("test-token")
                                      if probe[0] == "oversized token")
        token = headers[0][1].removeprefix("Bearer ")
        self.assertGreater(len(token), 8192)
        message, signature = token.rsplit(".", 1)
        self.assertEqual(signature, security.encoded(hmac.new(
            security.DEMO_SECRET.encode(), message.encode(), hashlib.sha256).digest()))
        payload = message.split(".")[1]
        claims = json.loads(base64.urlsafe_b64decode(payload + "=" * (-len(payload) % 4)))
        self.assertEqual(claims["user_id"], 1)
        self.assertGreater(claims["exp"], int(time.time()))
        self.assertEqual(status, 401)


if __name__ == "__main__":
    unittest.main()
