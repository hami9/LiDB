#!/usr/bin/env python3
"""Deterministic Semantic Versioning for merged LiDB commits.

Does not change repository state; does not publish or tag. Stdlib only.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys

VERSION_RE = re.compile(r"^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
HEADER_RE = re.compile(
    r"^(?P<type>feat|fix|perf|refactor|docs|test|build|ci|chore|style|revert)"
    r"(?:\([\w.\-/]+\))?(?P<breaking>!)?:\s+.+",
    re.IGNORECASE,
)
TRAILER_RE = re.compile(r"(?im)^Release-Bump:\s*(major|minor|patch)\s*$")
BREAKING_RE = re.compile(r"(?im)^BREAKING(?:[ -]CHANGE):\s*\S")
ORDER = {"none": 0, "patch": 1, "minor": 2, "major": 3}


def git(*arguments: str) -> str:
    result = subprocess.run(
        ["git", *arguments], check=True, capture_output=True, text=True, encoding="utf-8"
    )
    return result.stdout


def parse_version(value: str) -> tuple[int, int, int]:
    match = VERSION_RE.fullmatch(value)
    if not match:
        raise ValueError(f"Invalid release tag: {value!r}")
    return tuple(int(part) for part in match.groups())


def commit_bump(message: str) -> str:
    message = message.strip()
    if not message:
        return "none"
    declared = TRAILER_RE.findall(message)
    if declared:
        return max((item.lower() for item in declared), key=ORDER.get)
    header = message.splitlines()[0]
    match = HEADER_RE.match(header)
    if BREAKING_RE.search(message) or (match and match.group("breaking")):
        return "major"
    if match and match.group("type").lower() == "feat":
        return "minor"
    return "patch"


def highest_bump(messages: list[str]) -> str:
    return max((commit_bump(message) for message in messages), key=ORDER.get, default="none")


def increment(previous: tuple[int, int, int], bump: str) -> tuple[int, int, int]:
    major, minor, patch = previous
    if bump == "major":
        return major + 1, 0, 0
    if bump == "minor":
        return major, minor + 1, 0
    if bump == "patch":
        return major, minor, patch + 1
    if bump == "none":
        return previous
    raise ValueError(f"Unknown bump {bump!r}")


def latest_ancestor_tag() -> str | None:
    tags = git("tag", "--list", "v*").splitlines()
    candidates: list[tuple[tuple[int, int, int], str]] = []
    for tag in tags:
        if not VERSION_RE.fullmatch(tag):
            continue
        result = subprocess.run(
            ["git", "merge-base", "--is-ancestor", tag, "HEAD"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            candidates.append((parse_version(tag), tag))
    return max(candidates)[1] if candidates else None


def get_commits(since: str | None) -> list[str]:
    revision = f"{since}..HEAD" if since else "HEAD"
    raw = git("log", "-z", "--no-merges", "--format=%B", revision)
    return [entry.strip() for entry in raw.split("\0") if entry.strip()]


def build_plan(messages: list[str], previous_tag: str | None) -> dict:
    previous = parse_version(previous_tag) if previous_tag else (0, 0, 0)
    bump = highest_bump(messages)
    next_version = increment(previous, bump)
    version = ".".join(map(str, next_version))
    return {
        "previous_tag": previous_tag,
        "version": version,
        "tag": f"v{version}",
        "bump": bump,
        "commit_count": len(messages),
        "publish": bump != "none",
    }


def render_notes(messages: list[str], plan: dict) -> str:
    groups: dict[str, list[str]] = {"major": [], "minor": [], "patch": []}
    for message in messages:
        title = message.splitlines()[0].strip()
        classification = commit_bump(message)
        if classification in groups:
            groups[classification].append(title)
    headings = {"major": "Breaking changes", "minor": "Features", "patch": "Fixes, maintenance and documentation"}
    chunks = [
        f"# LiDB {plan['tag']}",
        "",
        f"Release type: **{plan['bump'].upper()}**. Compared with {plan['previous_tag'] or 'initial repository state'}.",
        "",
    ]
    for kind in ("major", "minor", "patch"):
        if groups[kind]:
            chunks.extend([f"## {headings[kind]}", "", *[f"- {title}" for title in groups[kind]], ""])
    chunks.extend([
        "## Compatibility",
        "",
        "Check the changelog and documentation for actual supported features and hardware.",
        "The presence of a release does not imply DGX Spark, NCCL or multi-node hardware validation.",
        "",
    ])
    return "\n".join(chunks)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("plan", "notes"))
    parser.add_argument("--output", help="Path for Markdown notes (notes command)")
    args = parser.parse_args()
    try:
        previous = latest_ancestor_tag()
        commits = get_commits(previous)
        plan = build_plan(commits, previous)
    except (OSError, subprocess.CalledProcessError, ValueError) as exc:
        print(f"release plan failed: {exc}", file=sys.stderr)
        return 2
    if args.command == "plan":
        print(json.dumps(plan, sort_keys=True))
    else:
        result = render_notes(commits, plan)
        if args.output:
            from pathlib import Path
            Path(args.output).write_text(result, encoding="utf-8")
        else:
            print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
