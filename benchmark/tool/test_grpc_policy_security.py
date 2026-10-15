"""Evidence parsing must distinguish a reproduced regression from harness failure."""
import json
import unittest

import grpc_policy_security as runner


class PolicyEvidenceTests(unittest.TestCase):
    def output(self, before=False, **changes):
        result = {"rounds": 2, "guard_retained": not before,
                  "anonymous_rejected": 0 if before else 2,
                  "authenticated_accepted": 2, "public_accepted": 2,
                  "guarded_handler_calls": 4 if before else 2}
        result.update(changes)
        summary = "FAILED. 0 passed; 1 failed;" if before else "ok. 1 passed; 0 failed;"
        return runner.MARKER + json.dumps(result) + "\ntest result: " + summary + "\n"

    def test_baseline_requires_observed_admission_mismatch(self):
        result = runner.parse_result(self.output(True), 101, 2, True)
        self.assertTrue(result["expected_outcome_verified"])
        self.assertFalse(result["security_contract_passed"])

    def test_patched_requires_all_controls_and_handler_entry_count(self):
        self.assertTrue(runner.parse_result(self.output(), 0, 2, False)["security_contract_passed"])
        for field in ("anonymous_rejected", "authenticated_accepted", "public_accepted", "guarded_handler_calls"):
            self.assertFalse(runner.parse_result(self.output(**{field: 0}), 0, 2, False)["expected_outcome_verified"])

    def test_incomplete_or_compilation_failure_cannot_be_evidence(self):
        with self.assertRaises(ValueError):
            runner.parse_result("error: compilation failed", 101, 2, True)
        with self.assertRaises(ValueError):
            runner.parse_result(self.output() + self.output(), 0, 2, False)
        self.assertFalse(runner.parse_result(self.output(), 101, 2, False)["expected_outcome_verified"])
        self.assertFalse(runner.parse_result(self.output(), 0, 3, False)["expected_outcome_verified"])


if __name__ == "__main__":
    unittest.main()
