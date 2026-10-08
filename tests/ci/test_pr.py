"""PR style contract tests."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("lidb_check_pr", ROOT / "scripts/ci/check_pr.py")
assert spec is not None and spec.loader is not None
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


class PRTests(unittest.TestCase):
    def test_valid(self):
        for title in (
            "ci: automate releases", "feat(gpu): add memory model",
            "feat!: redesign API", "docs: update contribution guide",
        ):
            self.assertEqual(mod.validate(title), [])

    def test_invalid(self):
        self.assertTrue(mod.validate("Unstructured feature PR"))
        self.assertTrue(mod.validate("feat: a"))
        self.assertTrue(mod.validate("fix: " + "x" * 130))


if __name__ == "__main__":
    unittest.main()
