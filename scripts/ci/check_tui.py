#!/usr/bin/env python3
"""Detect the standalone TUI workspace without silently skipping partial code."""
from pathlib import Path
import sys


def tui_present(root: Path) -> bool:
    """Return True if standalone TUI exists with manifest and lockfile, False if absent.

    Raises:
        ValueError: If prototypes/tui directory exists but lacks Cargo.toml or Cargo.lock.
    """
    workspace = root / "prototypes" / "tui"
    if not workspace.exists():
        return False
    if not (workspace / "Cargo.toml").is_file():
        raise ValueError("prototypes/tui exists without Cargo.toml")
    if not (workspace / "Cargo.lock").is_file():
        raise ValueError("prototypes/tui requires a committed Cargo.lock for locked CI")
    return True


if __name__ == "__main__":
    try:
        print(f"present={str(tui_present(Path.cwd())).lower()}")
    except ValueError as error:
        print(f"::error::{error}", file=sys.stderr)
        sys.exit(1)
