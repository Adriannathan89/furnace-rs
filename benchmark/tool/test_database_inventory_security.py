"""Benchmark verdicts must require completed, specifically named regressions."""

import unittest

from database_inventory_security import verify_result


class VerdictTests(unittest.TestCase):
    def output(self, status="ok", passed=1, failed=0, ignored=0):
        return (f"test fixture ... {status}\n"
                f"test result: {status}. {passed} passed; {failed} failed; {ignored} ignored; "
                "0 measured; 0 filtered out; finished in 0.01s\n")

    def test_completed_named_success_is_accepted(self):
        self.assertTrue(verify_result(self.output(), 0, ("fixture",))["expected_outcome_verified"])

    def test_success_requires_zero_exit_and_named_control(self):
        for output, code in [(self.output(), 101), (self.output().replace("fixture", "other"), 0),
                             (self.output(ignored=1), 0), (self.output(passed=0), 0)]:
            with self.subTest(output=output, code=code):
                self.assertFalse(verify_result(output, code, ("fixture",))["expected_outcome_verified"])

    def test_expected_baseline_failure_requires_specific_diagnostic(self):
        output = self.output("FAILED", passed=0, failed=1) + "specific cause\n"
        self.assertTrue(verify_result(output, 101, ("fixture",), "specific cause")["expected_outcome_verified"])
        for value, code in [(output.replace("specific cause", "other panic"), 101),
                            (output.replace("fixture", "other"), 101), (output, 0),
                            (self.output() + "specific cause", 0)]:
            with self.subTest(output=value, code=code):
                self.assertFalse(verify_result(value, code, ("fixture",), "specific cause")["expected_outcome_verified"])

    def test_missing_or_ambiguous_summary_cannot_pass(self):
        for output in ("compile error", self.output() * 2):
            with self.subTest(output=output):
                with self.assertRaises(ValueError):
                    verify_result(output, 0, ("fixture",))


if __name__ == "__main__":
    unittest.main()
