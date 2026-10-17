"""Checks for the historical-client replay's comparability guards."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from python_reference import operation_order


class PythonReferenceTests(unittest.TestCase):
    def test_order_matches_independently_recorded_rust_trial(self):
        # Recorded by furnace-bench's native Rust engine, not derived by this helper.
        seed = 15815171069621752182
        orders = [operation_order(63 if worker < 16 else 62, seed ^ (worker * 0x9E3779B1))
                  for worker in range(32)]
        serialized = ";".join(f"{worker}:{index}"
                              for worker, order in enumerate(orders) for index in order)
        self.assertEqual(hashlib.sha256(serialized.encode()).hexdigest(),
                         "73190c994d49a9e58ee965ed4af8f02421748abdf5122a441feb41f2af0aafb3")

    def test_invalid_reference_never_becomes_a_successful_baseline(self):
        for reference in [{"passed": False},
                          {"passed": True, "results": [{"scenario": "oversized"}]}]:
            with self.subTest(reference=reference), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "reference.json"
                source.write_text(json.dumps(reference))
                destination = root / "baseline.json"
                output = subprocess.run(
                    [sys.executable, str(Path(__file__).with_name("python_reference.py")),
                     "--reference", str(source), "--output", str(destination)],
                    capture_output=True, text=True,
                )
                self.assertEqual(output.returncode, 2)
                self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
