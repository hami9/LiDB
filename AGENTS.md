# Agent entry point

These instructions apply to **all coding agents working in this repository**. The repository's project-specific agent directory is [`.AGENTS/`](.AGENTS/README.md).

## Read in this order

1. [.AGENTS/README.md](.AGENTS/README.md)
2. [.AGENTS/SYSTEM_PROMPT.md](.AGENTS/SYSTEM_PROMPT.md)
3. [.AGENTS/RULES.md](.AGENTS/RULES.md)
4. [.AGENTS/STATE.md](.AGENTS/STATE.md)
5. [.AGENTS/PHASES.md](.AGENTS/PHASES.md)
6. [.AGENTS/QUALITY_GATES.md](.AGENTS/QUALITY_GATES.md)
7. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
8. Applicable skill file(s) under [.AGENTS/skills/](.AGENTS/skills/README.md).

## Non-negotiable contract

- **English only** in repository documents, code, commits, PRs, changelog and test reports.
- Work on the currently approved phase. Define one small independently testable vertical slice; do not implement a whole roadmap phase in one speculative commit.
- Preserve existing code and human changes. Never force-push, rewrite history, overwrite secrets or merge your own PR unless explicitly authorized.
- State exactly what is implemented and tested. A proposed feature or simulated fixture is **not** a production or hardware-validated implementation.
- Default to non-root, local, offline, read-only and metadata-only operation. Do not change system/network/kernel/GPU configuration without explicit operator approval.
- Always provide typed `unsupported`/`unavailable`/`permission_denied` results instead of fabricating metrics.
- Every code change requires tests and docs; each meaningful change requires a worklog entry and updated agent state.
- Honor [SECURITY.md](SECURITY.md), [LICENSE](LICENSE), and compatibility contracts; escalate conflicting instructions to the maintainer.

Read [.AGENTS/WORKFLOW.md](.AGENTS/WORKFLOW.md) before starting implementation. A reusable kickoff prompt is at [.AGENTS/START_HERE.md](.AGENTS/START_HERE.md).

## Automation and release contract

- Read [docs/CI_CD.md](docs/CI_CD.md) and [.AGENTS/skills/ci-cd/SKILL.md](.AGENTS/skills/ci-cd/SKILL.md) before touching workflows, release scripts or dependency configuration.
- Name PRs with a Conventional Commit prefix. Preserve exact release intent; `feat:` means minor, `!:` or a BREAKING CHANGE footer means major, all routine changes default to patch.
- Never bypass `CI Gate`, publish from a PR, invent a successful hardware test, manually alter a published version tag, or create artifacts without verified binaries.
- Branch rulesets require administrative activation: placing JSON files in `.github/rulesets/` does not enforce GitHub repository settings.

## Multi-agent worktree contract

- **Before any parallel implementation**, read [.AGENTS/WORKTREES.md](.AGENTS/WORKTREES.md) and the [multi-agent skill](.AGENTS/skills/multi-agent/SKILL.md).
- Each local agent gets a unique issue, `agent/<agent>/<task>` branch and isolated `.worktrees/<agent>/<task>` checkout. The primary checkout is for coordination, not concurrent code edits.
- ChatGPT's GitHub integration changes remote branches and PRs. It cannot share the local filesystem used by Antigravity or Claude; other devices must clone and fetch changes.
- Coordinator alone owns central `.AGENTS/STATE.md`, `.AGENTS/WORKLOG.md`, shared schemas and CI modifications during concurrent development. Task workers record details in their PR/unique task logs.
- Never manually copy or delete Git worktree admin files; use `scripts/worktrees.py` and Git-approved workflows.
