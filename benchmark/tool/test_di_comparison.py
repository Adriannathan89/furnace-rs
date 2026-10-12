import unittest

from di_comparison import dependency_counts_are_built, randomized_schedule, summarize, trial_seed


class DependencyComparisonTests(unittest.TestCase):
    def test_requested_graph_sizes_may_be_a_subset_of_built_sizes(self):
        self.assertTrue(dependency_counts_are_built((1, 256), (1, 16, 64, 256)))
        self.assertFalse(dependency_counts_are_built((512,), (1, 16, 64, 256)))

    def test_randomized_schedule_is_complete_repeatable_and_limits_graph_to_furnace(self):
        first = randomized_schedule(("hello",), ("axum", "furnace-rs-beta"), 2, (1, 8), 71)
        second = randomized_schedule(("hello",), ("axum", "furnace-rs-beta"), 2, (1, 8), 71)

        self.assertEqual(first, second)
        self.assertEqual(len(first), 20)
        graph_trials = [entry for entry in first if entry[0].startswith("di-graph-")]
        self.assertEqual(len(graph_trials), 16)
        self.assertEqual({entry[0] for entry in graph_trials}, {"di-graph-noop", "di-graph-traverse"})
        self.assertEqual({entry[1] for entry in graph_trials}, {"furnace-rs-1.0.2", "furnace-rs-beta"})

    def test_trial_seed_is_stable_and_changes_between_independent_samples(self):
        self.assertEqual(trial_seed(5, "hello", "axum", 1), trial_seed(5, "hello", "axum", 1))
        self.assertNotEqual(trial_seed(5, "hello", "axum", 1), trial_seed(5, "hello", "axum", 2))

    def test_summary_reports_median_startup_with_http_metrics(self):
        runs = [
            {"passed": True, "responses_per_second": 100.0, "p50_ms": 1.0, "p95_ms": 2.0, "p99_ms": 3.0, "startup_seconds": 0.2},
            {"passed": True, "responses_per_second": 200.0, "p50_ms": 1.0, "p95_ms": 2.0, "p99_ms": 3.0, "startup_seconds": 0.4},
            {"passed": True, "responses_per_second": 150.0, "p50_ms": 1.0, "p95_ms": 2.0, "p99_ms": 3.0, "startup_seconds": 0.3},
        ]

        self.assertEqual(summarize(runs)["median_startup_seconds"], 0.3)


if __name__ == "__main__":
    unittest.main()
