---
trigger: model_decision
description: "Apply whenever Antigravity is asked to code in LiDB alongside Claude, ChatGPT, Codex or other concurrent agents, or to create worktrees, branches or PRs."
---

# Antigravity parallel-worktree rule

The top-level `AGENTS.md` and `.AGENTS/WORKTREES.md` are authoritative.

- Confirm the active worktree branch before editing. Never work directly on `main` or share an existing agent's writable directory.
- Use `scripts/worktree.ps1 create antigravity <task>` on Windows or `bash scripts/worktree.sh create antigravity <task>` on Linux, from the cloned repo root.
- The normal path is `.worktrees/antigravity/<task>` and branch `agent/antigravity/<task>`.
- Agree on a GitHub issue and mutually exclusive scope before starting. Do not race other agents on core schemas, version controls or central agent state.
- Use GitHub PRs for cross-machine collaboration. Do not pretend local files sync between cloud sessions.
- Follow the naming rules in `CONTRIBUTING.md`: short lowercase commits like `fix tab scroll`, 1 to 3 word task names, Conventional Commit PR titles.
- Commit and push only scoped work, run real tests and request review before merging. Keep product and architecture source files English-only.
