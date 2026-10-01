import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from build_furnace import EXAMPLES, ROOT, stage_example


class StageExampleTests(unittest.TestCase):
    def test_staged_examples_resolve_checkout_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in EXAMPLES:
                with self.subTest(example=name):
                    source = ROOT / "example" / name
                    original = (source / "Cargo.toml").read_text()
                    staged = stage_example(source, Path(directory) / name)
                    result = subprocess.run(
                        ["cargo", "check", "--offline", "--manifest-path", str(staged / "Cargo.toml"),
                         "--target-dir", str(ROOT / "target")],
                        env=dict(os.environ, CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0"),
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                    )
                    self.assertEqual(result.returncode, 0, result.stdout)
                    self.assertEqual((source / "Cargo.toml").read_text(), original)

    def test_stage_excludes_old_lockfile_and_target(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            (source / "Cargo.toml").write_text("[package]\nname='demo'\nversion='0.1.0'\n")
            (source / "Cargo.lock").write_text("stale")
            (source / ".env").write_text("secret=do-not-copy")
            (source / "target").mkdir()
            (source / "target" / "artifact").write_text("old")
            destination = stage_example(source, root / "staged")
            self.assertTrue((destination / "Cargo.toml").is_file())
            self.assertFalse((destination / "Cargo.lock").exists())
            self.assertFalse((destination / ".env").exists())
            self.assertFalse((destination / "target").exists())

    def test_stage_uses_benchmark_lock_instead_of_example_lock(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            (source / "Cargo.toml").write_text("[package]\nname='demo'\nversion='0.1.0'\n")
            (source / "Cargo.lock").write_text("stale")
            lock = root / "benchmark.lock"
            lock.write_text("pinned")
            destination = stage_example(source, root / "staged", lock)
            self.assertEqual((destination / "Cargo.lock").read_text(), "pinned")


if __name__ == "__main__":
    unittest.main()
