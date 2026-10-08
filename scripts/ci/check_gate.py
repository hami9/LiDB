#!/usr/bin/env python3
"""Require successful results from every fixed CI branch."""
import os
import sys

REQUIRED = ("DOCS", "RUST", "WORKTREES", "TUI")


def failures(results: dict[str, str]) -> list[str]:
    return [f"{name}={results.get(name, 'missing')}"
            for name in REQUIRED if results.get(name) != "success"]


if __name__ == "__main__":
    blocked = failures(dict(os.environ))
    if blocked:
        print(f"::error::CI Gate blocked: {', '.join(blocked)}", file=sys.stderr)
        sys.exit(1)
    print("CI Gate: PASS")
