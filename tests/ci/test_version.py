"""Regression tests for release planning; no GitHub token required."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("lidb_release_version", ROOT / "scripts/release/version.py")
assert spec is not None and spec.loader is not None
version = importlib.util.module_from_spec(spec)
spec.loader.exec_module(version)


class VersionTests(unittest.TestCase):
    def test_patch_default_and_fix(self):
        self.assertEqual(version.commit_bump("docs: update README"), "patch")
        self.assertEqual(version.commit_bump("fix(net): repair parser"), "patch")
        self.assertEqual(version.commit_bump("Merge pull request #7"), "patch")

    def test_minor_feature(self):
        self.assertEqual(version.commit_bump("feat(ai): add GPU panel"), "minor")
        self.assertEqual(version.increment((0, 0, 9), "minor"), (0, 1, 0))

    def test_major_breaking(self):
        self.assertEqual(version.commit_bump("feat(core)!: redesign socket API"), "major")
        self.assertEqual(version.commit_bump("refactor: schemas\n\nBREAKING CHANGE: incompatible schema"), "major")
        self.assertEqual(version.increment((0, 9, 2), "major"), (1, 0, 0))

    def test_explicit_overrides(self):
        self.assertEqual(version.commit_bump("chore: migration\n\nRelease-Bump: major"), "major")
        self.assertEqual(version.commit_bump("feat: small change\n\nRelease-Bump: patch"), "patch")
        self.assertEqual(version.commit_bump("feat: change\n\nRelease-Bump: minor"), "minor")

    def test_highest_wins(self):
        self.assertEqual(version.highest_bump(["fix: bug", "feat: new", "chore!: break"]), "major")
        self.assertEqual(version.highest_bump(["docs: a", "feat: b"]), "minor")

    def test_version_zero_and_empty(self):
        self.assertEqual(version.build_plan([], None)["publish"], False)
        self.assertEqual(version.build_plan(["fix: bug"], None)["tag"], "v0.0.1")
        self.assertEqual(version.build_plan(["feat: new"], "v0.3.8")["tag"], "v0.4.0")
        self.assertEqual(version.build_plan(["feat!: breaking"], "v0.4.5")["tag"], "v1.0.0")

    def test_tag_validation(self):
        self.assertEqual(version.parse_version("v1.2.3"), (1, 2, 3))
        for bad in ("v01.2.3", "1.2.3", "v1.2.3+1", "v1.2.3-beta"):
            with self.assertRaises(ValueError):
                version.parse_version(bad)

    def test_release_notes(self):
        plan = version.build_plan(["feat: hello", "fix: bug"], None)
        notes = version.render_notes(["feat: hello", "fix: bug"], plan)
        self.assertIn("Features", notes)
        self.assertIn("Fixes", notes)
        self.assertIn("v0.1.0", notes)


if __name__ == "__main__":
    unittest.main()
