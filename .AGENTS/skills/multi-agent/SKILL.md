---
name: lidb-multi-agent-worktrees
description: Git worktree isolation, cross-device task assignment and non-conflicting agent handoffs.
---

# Multi-agent and worktree coordination skill

Use when orchestrating Antigravity, Claude, ChatGPT, Codex or several agent sessions.

1. Discover Git branch, main, other worktrees, PRs and current `.AGENTS/STATE.md`.
2. Allocate one GitHub issue, unique agent/task slug and non-overlapping file scope per worker.
3. Create with `python3 scripts/worktrees.py create <agent> <task>`, never by copying a Git directory.
4. Give each agent its own editor window/process rooted at its worktree; do not share mutable files or run multiple writes to the same branch.
5. Keep coordinator-only files (`.AGENTS/STATE.md`, `.AGENTS/WORKLOG.md`, CI and cross-cutting schemas) serialized; per-task evidence belongs in PRs.
6. On push, open PR, wait for CI Gate and independent review. No forced pushes or unauthorized self-merges.
7. Do not claim Git Worktree is a networked distributed lock or that user devices share one local worktree.
8. Remove only clean registered worktrees with `scripts/worktrees.py remove`; keep branches until PR verified merged.

See [Worktree guide](../../WORKTREES.md), [Workflow](../../WORKFLOW.md) and [Security rules](../../RULES.md).
