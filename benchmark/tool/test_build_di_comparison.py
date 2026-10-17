import unittest

from build_di_comparison import baseline_lock_path, third_party_packages
from run import ROOT


class ComparisonBuildTests(unittest.TestCase):
    def test_baseline_lock_matches_beta_third_party_package_versions(self):
        for example in ("hello-world", "protected-route"):
            with self.subTest(example=example):
                baseline = baseline_lock_path(example)
                current = ROOT / "benchmark" / "targets" / "furnace-rs" / "locks" / f"{example}.lock"
                self.assertEqual(third_party_packages(baseline), third_party_packages(current))

    def test_unknown_baseline_example_is_rejected(self):
        with self.assertRaises(ValueError):
            baseline_lock_path("posts-crud")


if __name__ == "__main__":
    unittest.main()
