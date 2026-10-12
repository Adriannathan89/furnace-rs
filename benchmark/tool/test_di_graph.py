import unittest

from di_graph import dependency_layout, parse_dependency_counts, render_source


class DependencyGraphTests(unittest.TestCase):
    def test_layout_contains_requested_reachable_dependency_count(self):
        layout = dependency_layout(11)

        self.assertEqual(len(layout), 11)
        self.assertEqual(layout[0], (1, 2))
        self.assertEqual(layout[4], (9, 10))
        self.assertEqual(layout[-1], ())
        self.assertTrue(all(child > index for index, children in enumerate(layout) for child in children))

    def test_source_registers_every_node_in_both_di_apis(self):
        current = render_source(7, declarative=True)
        legacy = render_source(7, declarative=False)

        for node in range(7):
            name = f"Dependency{node:04}"
            self.assertIn(name, current)
            self.assertIn(f".provide::<{name}>()", legacy)
        self.assertIn("#[cauldron(root", current)
        self.assertIn("impl furnace_rs::core::Cauldron for AppCauldron", legacy)
        self.assertIn("dependency_count", current)
        self.assertIn("dependency_count", legacy)
        self.assertIn('#[get("/noop")]', current)
        self.assertIn('#[get("/traverse")]', current)
        self.assertIn('#[get("/noop")]', legacy)
        self.assertIn('#[get("/traverse")]', legacy)

    def test_traversal_uses_a_runtime_side_effect_for_each_node(self):
        source = render_source(5, declarative=True)

        self.assertIn("AtomicUsize::new(0)", source)
        self.assertIn("fetch_add(1, Ordering::Relaxed)", source)
        self.assertEqual(source.count("touch_graph_node();"), 5)

    def test_current_graph_marks_dependencies_without_changing_the_baseline_api(self):
        current = render_source(7, declarative=True)
        baseline = render_source(7, declarative=False)
        # Six tree edges plus the controller's root dependency.
        self.assertEqual(current.count("#[inject]"), 7)
        self.assertNotIn("#[inject]", baseline)

    def test_non_positive_dependency_counts_are_rejected(self):
        for count in (0, -1):
            with self.subTest(count=count), self.assertRaises(ValueError):
                dependency_layout(count)

    def test_dependency_count_option_parses_a_unique_positive_list(self):
        self.assertEqual(parse_dependency_counts("1,8,128"), (1, 8, 128))

    def test_dependency_count_option_rejects_empty_duplicate_and_non_integer_values(self):
        for raw in ("", "1,,8", "2,2", "1,nope", "0,4"):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                parse_dependency_counts(raw)


if __name__ == "__main__":
    unittest.main()
