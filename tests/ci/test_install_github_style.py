"""Verify safe, repeatable user-scope skill installation in temporary directories."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('install_github_style', ROOT / 'scripts/install_github_style.py')
INSTALLER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALLER)


class InstallGithubStyleTests(unittest.TestCase):
    def test_repeatable_copy_preserves_supplied_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / 'skills/github-style'
            self.assertTrue(INSTALLER.install(INSTALLER.SOURCE, directory).startswith('Installed:'))
            self.assertEqual((directory / 'SKILL.md').read_bytes(), INSTALLER.SOURCE.read_bytes())
            self.assertTrue(INSTALLER.install(INSTALLER.SOURCE, directory).startswith('Already installed:'))

    def test_existing_different_skill_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            target = directory / 'SKILL.md'
            target.write_text('Existing user instructions')
            with self.assertRaises(ValueError):
                INSTALLER.install(INSTALLER.SOURCE, directory)
            self.assertEqual(target.read_text(), 'Existing user instructions')

    def test_symlink_is_not_followed(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            other = directory / 'other.md'
            other.write_text('Preserve this file')
            try:
                (directory / 'SKILL.md').symlink_to(other)
            except OSError as error:
                self.skipTest(f'Symlink creation unavailable: {error}')
            with self.assertRaises(ValueError):
                INSTALLER.install(INSTALLER.SOURCE, directory)
            self.assertEqual(other.read_text(), 'Preserve this file')


if __name__ == '__main__':
    unittest.main()
