#!/usr/bin/env python3
"""Validate PR naming and release intent (reads local GitHub event payload)."""
import json
import os
from pathlib import Path
import re
import sys

TITLE = re.compile(
    r"^(feat|fix|perf|refactor|docs|test|build|ci|chore|style|revert)"
    r"(?:\([a-zA-Z0-9_.-]+\))?!?: .{3,}$"
)


def validate(title: str) -> list[str]:
    messages = []
    if not TITLE.fullmatch(title.strip()):
        messages.append(
            "PR title must use a Conventional Commit prefix, e.g. "
            "'feat(gpu): add supported device detection' or 'fix: handle missing daemon'."
        )
    if len(title) > 120:
        messages.append("PR title must not exceed 120 characters.")
    return messages


def main() -> int:
    if os.environ.get("GITHUB_EVENT_NAME") != "pull_request":
        print("PR title check: not a pull_request event; skipped")
        return 0
    path = os.environ.get("GITHUB_EVENT_PATH")
    if not path:
        print("Missing GITHUB_EVENT_PATH", file=sys.stderr)
        return 2
    event = json.loads(Path(path).read_text(encoding="utf-8"))
    title = event.get("pull_request", {}).get("title", "")
    failures = validate(title)
    for failure in failures:
        print(f"::error::{failure}")
    print(f"PR title check: {'FAIL' if failures else 'PASS'}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
