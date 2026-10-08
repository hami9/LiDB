#!/usr/bin/env python3
"""Create and manage isolated local Git worktrees for LiDB coding agents.

Python standard library only. Supports Windows and Linux. This script never
pushes, merges, rebases, deletes branches, or operates on production hosts.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import re
import subprocess
import sys

SLUG = re.compile(r"^[a-z][a-z0-9-]{0,39}$")
PRIMARY_BRANCH = "main"


class WorktreeError(RuntimeError):
    """Action refused because preconditions or safety checks failed."""


def git(*args: str, cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            ["git", *(["-C", str(cwd)] if cwd else []), *args],
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=check,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        detail = getattr(exc, "stderr", None) or str(exc)
        raise WorktreeError(f"git {' '.join(args)}: {detail.strip()}") from exc


def main_worktree() -> Path:
    # git worktree list reports the primary working tree before linked worktrees.
    info = git("worktree", "list", "--porcelain")
    first = next((line for line in info.stdout.splitlines() if line.startswith("worktree ")), None)
    if not first:
        raise WorktreeError("A non-bare Git working tree is required.")
    return Path(first[len("worktree "):]).resolve()


def check_slug(value: str, label: str) -> str:
    if not SLUG.fullmatch(value):
        raise WorktreeError(f"Invalid {label} {value!r}: use lowercase ASCII letters, digits, hyphens; begin with a letter (max 40 characters).")
    return value


def identity(root: Path, agent: str, task: str) -> tuple[str, Path]:
    agent, task = check_slug(agent, "agent"), check_slug(task, "task")
    workspace = root / ".worktrees"
    if workspace.exists() and workspace.is_symlink():
        raise WorktreeError("Refusing a symlinked .worktrees directory.")
    parent = workspace / agent
    if parent.exists() and parent.is_symlink():
        raise WorktreeError("Refusing a symlinked agent directory.")
    location = parent / task
    if not location.resolve().is_relative_to(workspace.resolve()):
        raise WorktreeError("Worktree location escapes .worktrees.")
    return f"agent/{agent}/{task}", location


def exists_ref(root: Path, ref: str) -> bool:
    result = git("show-ref", "--verify", "--quiet", ref, cwd=root, check=False)
    return result.returncode == 0


def create(root: Path, agent: str, task: str, offline: bool = False) -> None:
    branch, location = identity(root, agent, task)
    if location.exists() or location.is_symlink():
        raise WorktreeError(f"Location already exists: {location}")
    if exists_ref(root, f"refs/heads/{branch}"):
        raise WorktreeError(f"Local branch already exists: {branch}. Choose a new task ID; never reuse another agent's branch.")
    if offline:
        if not exists_ref(root, f"refs/heads/{PRIMARY_BRANCH}"):
            raise WorktreeError("Offline mode requires an existing local main branch.")
        start = PRIMARY_BRANCH
        print("Using local main without fetching; it may be stale.", flush=True)
    else:
        git("fetch", "--no-tags", "origin", PRIMARY_BRANCH, cwd=root)
        start = f"refs/remotes/origin/{PRIMARY_BRANCH}"
        if not exists_ref(root, start):
            raise WorktreeError(f"Missing remote base: {start}")
        remote = git("ls-remote", "--heads", "origin", f"refs/heads/{branch}", cwd=root)
        if remote.stdout.strip():
            raise WorktreeError(f"Remote branch already exists: {branch}. Coordinate ownership through GitHub Issues/PRs.")
    location.parent.mkdir(parents=True, exist_ok=True)
    git("worktree", "add", "-b", branch, str(location), start, cwd=root)
    print(f"Created worktree: {location}")
    print(f"Agent branch: {branch}")
    print("Open this directory as its own workspace. Push only this branch and create a PR.")


def remove(root: Path, agent: str, task: str) -> None:
    branch, location = identity(root, agent, task)
    # Safety: a local folder with this name is not necessarily a Git worktree.
    listed = git("worktree", "list", "--porcelain", cwd=root).stdout
    registered = {Path(line[9:]).resolve() for line in listed.splitlines() if line.startswith("worktree ")}
    if location.resolve() not in registered:
        raise WorktreeError(f"Not a registered worktree: {location}")
    actual = git("symbolic-ref", "--quiet", "--short", "HEAD", cwd=location, check=False)
    if actual.returncode != 0 or actual.stdout.strip() != branch:
        raise WorktreeError(f"Branch mismatch: expected {branch}; refusing removal.")
    if git("status", "--porcelain", "--untracked-files=all", cwd=location).stdout.strip():
        raise WorktreeError(f"Worktree is dirty: {location}. Commit/review all files first.")
    git("worktree", "remove", str(location), cwd=root)
    print(f"Removed clean worktree: {location}")
    print(f"Branch retained: {branch}. Delete it separately only after confirming merged PR and backups.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_subparsers(dest="command", required=True)
    creator = actions.add_parser("create", help="Create an isolated agent worktree and fresh branch.")
    creator.add_argument("agent", help="e.g. antigravity, claude, chatgpt")
    creator.add_argument("task", help="e.g. p0-core, p0-telemetry")
    creator.add_argument("--offline", action="store_true", help="Start from local main; do not access origin.")
    remover = actions.add_parser("remove", help="Remove a clean registered worktree but retain its branch.")
    remover.add_argument("agent")
    remover.add_argument("task")
    actions.add_parser("list", help="Show all local worktrees and their Git state.")
    actions.add_parser("doctor", help="Inspect local repository configuration (read-only).")
    args = parser.parse_args()
    try:
        root = main_worktree()
        if args.command == "create":
            create(root, args.agent, args.task, args.offline)
        elif args.command == "remove":
            remove(root, args.agent, args.task)
        elif args.command == "list":
            print(git("worktree", "list", "--porcelain", cwd=root).stdout, end="")
        else:
            print(f"Primary worktree: {root}")
            print(f"Managed worktrees: {root / '.worktrees'}")
            print(f"Git version: {git('--version').stdout.strip()}")
            print("Other devices use their own clones; coordination is through GitHub branches and PRs.")
    except WorktreeError as exc:
        print(f"Worktree operation refused: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
