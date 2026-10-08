"""Ensure proposed branch and tag protections remain safe and unambiguous."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class RulesetsTests(unittest.TestCase):
    def read(self, name):
        return json.loads((ROOT / ".github/rulesets" / name).read_text(encoding="utf-8"))

    def test_main_has_gate_and_pr(self):
        rules = self.read("main.json")
        self.assertEqual(rules["target"], "branch")
        self.assertEqual(rules["enforcement"], "active")
        self.assertEqual(rules["bypass_actors"], [])
        self.assertEqual(rules["conditions"]["ref_name"]["include"], ["refs/heads/main"])
        kinds = {entry["type"]: entry for entry in rules["rules"]}
        for required in ("pull_request", "required_status_checks", "non_fast_forward", "deletion", "required_linear_history"):
            self.assertIn(required, kinds)
        statuses = kinds["required_status_checks"]["parameters"]
        self.assertEqual(statuses["required_status_checks"], [{"context": "CI Gate"}])
        self.assertTrue(statuses["strict_required_status_checks_policy"])
        self.assertEqual(kinds["pull_request"]["parameters"]["required_approving_review_count"], 0)
        self.assertEqual(kinds["pull_request"]["parameters"]["allowed_merge_methods"], ["squash", "rebase"])

    def test_tags_cannot_be_deleted_or_moved(self):
        rules = self.read("release-tags.json")
        self.assertEqual(rules["target"], "tag")
        self.assertEqual(rules["conditions"]["ref_name"]["include"], ["refs/tags/v*"])
        self.assertEqual({entry["type"] for entry in rules["rules"]}, {"non_fast_forward", "deletion"})
        self.assertEqual(rules["bypass_actors"], [])


if __name__ == "__main__":
    unittest.main()
