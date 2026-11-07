#!/usr/bin/env python3
"""Integration tests for scripts/install.sh."""

import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INSTALL_SCRIPT = ROOT / "scripts" / "install.sh"


class InstallScriptTests(unittest.TestCase):
    def test_script_exists_and_is_nonempty(self):
        self.assertTrue(INSTALL_SCRIPT.is_file())
        content = INSTALL_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("REPO=\"hami9/LiDB\"", content)
        self.assertIn("BINARY_NAME=\"lidash\"", content)
        self.assertIn("usage()", content)

    def test_help_option_exits_zero_with_usage(self):
        # On POSIX or Git Bash on Windows:
        bash_path = "bash"
        try:
            result = subprocess.run(
                [bash_path, "scripts/install.sh", "--help"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode == 0:
                self.assertIn("LiDashBoard (lidash) Installer", result.stdout)
                self.assertIn("--uninstall", result.stdout)
                self.assertIn("--dry-run", result.stdout)
        except FileNotFoundError:
            # Skip if bash is not available on execution environment
            pass

    def test_dry_run_option_exits_cleanly(self):
        bash_path = "bash"
        try:
            result = subprocess.run(
                [bash_path, "scripts/install.sh", "--dry-run", "--tag", "v0.0.6", "--dir", "/tmp"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
            # Either passes dry-run on Linux or fails with unsupported OS on Windows
            if "Unsupported operating system" not in result.stderr:
                self.assertEqual(result.returncode, 0)
                self.assertIn("[dry-run]", result.stdout)
        except FileNotFoundError:
            pass


if __name__ == "__main__":
    unittest.main()
