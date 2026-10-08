"""Regressions for standalone workspaces and required-job aggregation."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / "ci" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


tui = load("check_tui")
gate = load("check_gate")


class TuiDetectionTests(unittest.TestCase):
    def test_docs_only(self):
        with tempfile.TemporaryDirectory() as directory:
            self.assertFalse(tui.tui_present(Path(directory)))

    def test_standalone_without_root_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / "prototypes" / "tui"
            workspace.mkdir(parents=True)
            (workspace / "Cargo.toml").write_text("[workspace]\n[package]\nname='tui'\n")
            (workspace / "Cargo.lock").write_text("version = 3\n")
            self.assertTrue(tui.tui_present(root))
            result = subprocess.run([sys.executable, str(ROOT / "scripts/ci/check_tui.py")],
                                    cwd=root, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, "present=true\n")

    def test_partial_workspace_fails(self):
        for missing in ("Cargo.toml", "Cargo.lock"):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                workspace = root / "prototypes" / "tui"
                workspace.mkdir(parents=True)
                if missing == "Cargo.lock":
                    (workspace / "Cargo.toml").write_text("[workspace]\n")
                with self.assertRaises(ValueError):
                    tui.tui_present(root)
                result = subprocess.run([sys.executable, str(ROOT / "scripts/ci/check_tui.py")],
                                        cwd=root, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("present=false", result.stdout)


class GateTests(unittest.TestCase):
    def test_success(self):
        self.assertEqual(gate.failures(dict.fromkeys(gate.REQUIRED, "success")), [])

    def test_each_required_job_blocks(self):
        for job in gate.REQUIRED:
            for state in ("failure", "cancelled", "skipped", "", "unknown"):
                with self.subTest(job=job, state=state):
                    results = dict.fromkeys(gate.REQUIRED, "success")
                    results[job] = state
                    self.assertEqual(gate.failures(results), [f"{job}={state}"])
            results = dict.fromkeys(gate.REQUIRED, "success")
            del results[job]
            self.assertEqual(gate.failures(results), [f"{job}=missing"])

    def test_cli_exit_status(self):
        script = str(ROOT / "scripts/ci/check_gate.py")
        for state, expected in (("success", 0), ("failure", 1), ("skipped", 1)):
            results = dict.fromkeys(gate.REQUIRED, "success")
            results["TUI"] = state
            result = subprocess.run([sys.executable, script], env=results,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, expected, result.stderr)


if __name__ == "__main__":
    unittest.main()
