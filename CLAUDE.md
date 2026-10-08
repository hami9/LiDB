# LiDashBoard — Claude Code repository instructions

Follow the canonical [AGENTS.md](AGENTS.md) and [.AGENTS/SYSTEM_PROMPT.md](.AGENTS/SYSTEM_PROMPT.md). For multiple agents, read [.AGENTS/WORKTREES.md](.AGENTS/WORKTREES.md) and [.AGENTS/WORKFLOW.md](.AGENTS/WORKFLOW.md) before editing.

- Open a dedicated worktree at `.worktrees/claude/<task>` using the worktree manager. Never share a writable folder or branch with another agent.
- One bounded issue and file ownership scope per worktree. Keep commits and PRs focused and named with Conventional Commits.
- Do not change `main` directly or edit coordinator-owned `.AGENTS/STATE.md` and `.AGENTS/WORKLOG.md` from concurrent feature branches.
- Preserve current constraints: English-only, security-first, Rust userspace + optional C/eBPF, honest hardware verification, no unexpected privileges.
- Log exact tests in your PR; finish with a handoff and an explicit list of incomplete requirements.
