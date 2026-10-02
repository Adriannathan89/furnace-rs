"""A failed or incomplete native permission test must fail the benchmark."""

import json
import unittest

from cli_security import MARKER, parse_result


class CliSecurityTests(unittest.TestCase):
    def output(self, **changes):
        result = {"rounds": 2, "checks": 8, "failures": 0, "passed": True}
        return MARKER + json.dumps({**result, **changes}) + "\n"

    def test_full_successful_run_passes(self):
        self.assertTrue(parse_result(self.output(), 0, 2)["passed"])

    def test_nonzero_exit_overrides_success_claim(self):
        self.assertFalse(parse_result(self.output(), 101, 2)["passed"])

    def test_failed_and_incomplete_checks_fail(self):
        for changes in ({"failures": 1}, {"checks": 7}, {"passed": False}):
            with self.subTest(changes=changes):
                self.assertFalse(parse_result(self.output(**changes), 0, 2)["passed"])

    def test_missing_ambiguous_or_wrong_rounds_fail(self):
        for output in ("", self.output() * 2, self.output(rounds=1), MARKER + "[]"):
            with self.subTest(output=output):
                with self.assertRaises(ValueError):
                    parse_result(output, 0, 2)


if __name__ == "__main__":
    unittest.main()
