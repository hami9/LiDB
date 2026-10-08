"""Safety and life-cycle tests for local worktrees using temporary Git repositories."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "worktrees.py"


def run(*args: str, cwd: Path, ok: bool = True):
    result = subprocess.run(args, cwd=cwd, text=True, encoding="utf-8", capture_output=True)
    if ok and result.returncode != 0:
        raise AssertionError(f"{args} returned {result.returncode}: {result.stderr}")
    return result


class WorktreeIntegrationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        root = Path(self.tmp.name)
        seed = root / "seed"
        seed.mkdir()
        run("git", "init", "-b", "main", cwd=seed)
        run("git", "config", "user.name", "LiDB CI", cwd=seed)
        run("git", "config", "user.email", "lidb-ci@example.invalid", cwd=seed)
        (seed / "README.md").write_text("# Test repo\n", encoding="utf-8")
        run("git", "add", "README.md", cwd=seed)
        run("git", "commit", "-m", "test: seed fixture", cwd=seed)
        bare = root / "origin.git"
        run("git", "clone", "--bare", str(seed), str(bare), cwd=root)
        self.repo = root / "repo"
        run("git", "clone", str(bare), str(self.repo), cwd=root)

    def invoke(self, *args):
        return run(sys.executable, str(SCRIPT), *args, cwd=self.repo, ok=False)

    def test_create_list_remove_and_branch_retention(self):
        result = self.invoke("create", "antigravity", "p0-core", "--offline")
        self.assertEqual(result.returncode, 0, result.stderr)
        path = self.repo / ".worktrees/antigravity/p0-core"
        self.assertTrue(path.exists())
        branch = run("git", "branch", "--show-current", cwd=path).stdout.strip()
        self.assertEqual(branch, "agent/antigravity/p0-core")
        self.assertIn("agent/antigravity/p0-core", self.invoke("list").stdout)
        removal = self.invoke("remove", "antigravity", "p0-core")
        self.assertEqual(removal.returncode, 0, removal.stderr)
        self.assertFalse(path.exists())
        branches = run("git", "branch", "--list", "agent/antigravity/p0-core", cwd=self.repo).stdout
        self.assertIn("agent/antigravity/p0-core", branches)

    def test_online_creation_uses_remote_base(self):
        result = self.invoke("create", "claude", "p0-telemetry")
        self.assertEqual(result.returncode, 0, result.stderr)
        path = self.repo / ".worktrees/claude/p0-telemetry"
        self.assertTrue(path.exists())

    def test_duplicate_and_invalid_names_rejected(self):
        self.assertEqual(self.invoke("create", "claude", "p0-telemetry", "--offline").returncode, 0)
        self.assertNotEqual(self.invoke("create", "claude", "p0-telemetry", "--offline").returncode, 0)
        for args in (("create", "../escape", "p0", "--offline"), ("create", "claude", "../escape", "--offline"),
                     ("create", "claude", "UPPER", "--offline"), ("create", "", "p0", "--offline")):
            self.assertNotEqual(self.invoke(*args).returncode, 0)

    def test_dirty_worktree_cannot_be_removed(self):
        self.assertEqual(self.invoke("create", "chatgpt", "p0-ipc", "--offline").returncode, 0)
        path = self.repo / ".worktrees/chatgpt/p0-ipc"
        (path / "scratch.txt").write_text("uncommitted\n", encoding="utf-8")
        result = self.invoke("remove", "chatgpt", "p0-ipc")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(path.exists())

    def test_unrelated_directory_cannot_be_removed(self):
        path = self.repo / ".worktrees/claude/fake"
        path.mkdir(parents=True)
        (path / "important.txt").write_text("do not delete", encoding="utf-8")
        self.assertNotEqual(self.invoke("remove", "claude", "fake").returncode, 0)
        self.assertTrue((path / "important.txt").exists())

    def test_doctor_does_not_modify_repo(self):
        self.assertIn("Primary worktree:", self.invoke("doctor").stdout)
        self.assertEqual(self.invoke("list").returncode, 0)


if __name__ == "__main__":
    unittest.main()
