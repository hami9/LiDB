# Copy-ready implementation kickoff

The repository is LiDashBoard (LiDB): a terminal-native, open-source Linux and AI infrastructure diagnostics platform implemented primarily in Rust with narrowly scoped C/eBPF instrumentation.

**Your first task is P0, not the entire roadmap.** Start by reading `AGENTS.md`, `.AGENTS/SYSTEM_PROMPT.md`, `.AGENTS/RULES.md`, `.AGENTS/STATE.md`, `.AGENTS/PHASES.md`, `.AGENTS/QUALITY_GATES.md`, `docs/ARCHITECTURE.md`, `docs/PRODUCT_REQUIREMENTS.md`, and relevant `.AGENTS/skills/*/SKILL.md`.

Inspect the existing repository and branch. Propose a minimal P0 vertical slice with:
- files to touch and why;
- public APIs and feature/dependency boundaries;
- permissions, privacy and degraded behavior;
- exact tests and acceptance criteria;
- compatibility requirements for Linux x86_64/aarch64.

Implement only that slice after checking for conflicting work. No root TUI, no auto-network changes, no fabricated performance/hardware claims. Show real command outputs and update `.AGENTS/WORKLOG.md` and `.AGENTS/STATE.md`. Open a focused PR or provide a reviewable patch. Review the result across kernel correctness, AI/hardware semantics, security, reliability/performance and product/UX/extensibility. Treat unsupported hardware paths honestly.

Do not say a phase is complete unless all its written exit gates have been checked.

## If multiple agents run at once

Before coding, read [WORKTREES.md](WORKTREES.md), reserve a unique GitHub issue/branch scope and open the corresponding local worktree. Claude and Antigravity must use separate editor windows. GitHub-based agents work in remote feature branches, not on the user's unsynced local changes. Do not rewrite shared task-state files from independent branches.
