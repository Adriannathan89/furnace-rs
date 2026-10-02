"""Reject incomplete or failed native security runs even if their JSON says passed."""

import json
import unittest

from infrastructure_security import CHECKS_PER_ROUND, MARKER, parse_result


class InfrastructureSecurityTests(unittest.TestCase):
    def fixture(self):
        return {"rounds": 2, "passed": True,
                **{case: {"checks": 2 * count, "failures": 0}
                   for case, count in CHECKS_PER_ROUND.items()}}

    def output(self, result):
        return "cargo test preamble\n" + MARKER + json.dumps(result) + "\n"

    def test_nonzero_cargo_exit_cannot_be_reported_as_passed(self):
        self.assertFalse(parse_result(self.output(self.fixture()), 101, 2)["passed"])

    def test_missing_probes_cannot_be_reported_as_passed(self):
        result = self.fixture()
        del result["database_connection_trace"]
        self.assertFalse(parse_result(self.output(result), 0, 2)["passed"])

    def test_probe_failures_cannot_be_reported_as_passed(self):
        for case in CHECKS_PER_ROUND:
            with self.subTest(case=case):
                result = self.fixture()
                result[case]["failures"] = 1
                self.assertFalse(parse_result(self.output(result), 0, 2)["passed"])

    def test_a_full_successful_run_is_reported_as_passed(self):
        self.assertTrue(parse_result(self.output(self.fixture()), 0, 2)["passed"])

    def test_wrong_rounds_and_ambiguous_results_fail(self):
        output = self.output(self.fixture())
        for value, rounds in [(output, 200), (output + output, 2), ("", 2)]:
            with self.assertRaises(ValueError):
                parse_result(value, 0, rounds)


if __name__ == "__main__":
    unittest.main()
