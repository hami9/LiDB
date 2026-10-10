# Multi-agent Git Worktree Operating Model

**Status:** Local orchestration tooling, not a centralized agent execution service. A Git worktree exists on the **user's machine**, not in the GitHub repository itself. Each machine/cloud agent host must clone the repository and run the scripts there. No tool can start an Antigravity or Claude session from a GitHub document alone.

## Repository paths and roles

- **`.worktrees/` (ignored):** local linked Git working trees. Never commit Git administrative data or private agent state.
- **`AGENTS.md` (root):** universal instructions.
- **`CLAUDE.md`:** Claude Code entry point referencing canonical rules.
- **`.agents/rules/`:** native Antigravity scoped rules; **lowercase** is intentional and distinct from `.AGENTS/`.
- **`.AGENTS/`:** project agent policy, worklogs, skills, phases and review gates.
- **`scripts/worktrees.py`:** source of truth for create/list/remove/doctor.
- **`scripts/worktree.sh` and `scripts/worktree.ps1`:** Linux/macOS Git Bash and Windows PowerShell entry points.

## Quick start (Windows PowerShell)

```powershell
git clone https://github.com/hami9/LiDB.git
cd LiDB
.\scripts\worktree.ps1 create antigravity p0-core
.\scripts\worktree.ps1 create claude p0-telemetry
.\scripts\worktree.ps1 create chatgpt p0-fixtures
.\scripts\worktree.ps1 list
# Open each .worktrees/<agent>/<task> folder in its own editor/agent session.
```

Linux/macOS equivalent:

```bash
git clone https://github.com/hami9/LiDB.git
cd LiDB
bash scripts/worktree.sh create antigravity p0-core
bash scripts/worktree.sh create claude p0-telemetry
bash scripts/worktree.sh create chatgpt p0-fixtures
bash scripts/worktree.sh list
```

The manager fetches `origin/main` for each fresh branch, rejects branch collisions and creates `agent/<agent>/<task>` only in the current local repository. Use `--offline` only when intentionally starting from a potentially stale local `main`.

## Assignment and ownership

1. Coordinator creates one GitHub issue per independently testable task and assigns **one owning agent**. Avoid overlapping code files, generated files, shared schemas, CI and `.AGENTS/STATE.md`.
2. Owner creates a unique worktree and branch, records issue/branch/expected file scope in the issue and uses its OWN workspace.
3. Owner implements, tests, commits and pushes ONLY their branch, then opens a PR targeting `main`.
4. Review is done by a separate person/agent in a **read-only reviewer worktree or GitHub PR view**. Review is not approval to merge without maintainer permission.
5. CI Gate, human decision, protected `main` and squash merge are the only route to integration.
6. Coordinator updates `.AGENTS/STATE.md` and central `.AGENTS/WORKLOG.md` during integration/handoff. Each agent puts task evidence in its own PR or a unique `.AGENTS/worklogs/<task-id>.md` file to avoid central-write conflicts.

**Important:** Git worktrees do **not** create a distributed lock. On separate devices, the same branch name or task can be claimed twice. GitHub issues, PRs and human ownership provide the cross-device coordination mechanism. A normal Git worktree shares a repository's refs and object database with other local worktrees but maintains its own HEAD, index and working files.

## Integration boundaries and anti-conflict rules

| Scope | Owner in example P0 allocation | Parallel-safe? |
| --- | --- | --- |
| Cargo workspace manifests, core telemetry schema | Single integration owner | **No**; land first |
| CLI/TUI shell once API frozen | Antigravity | Yes, if limited to dedicated crates |
| Diagnostic fixtures / tests | Claude | Yes, if not editing the same files |
| Procfs fixtures / contract review | ChatGPT | Yes in owned tests/docs; schema edits need coordinator lock |
| `.AGENTS/STATE.md`, central `WORKLOG.md`, CI workflows | Integrator | **No**; serialize edits |
| Optional networking/eBPF future phases | Assigned module owner | Only after P0 API acceptance |

Examples are suggested allocations, **not claims of active assignments or launched agents**.

## Safe commands

```bash
# From repo root or any linked tree:
python3 scripts/worktrees.py doctor
python3 scripts/worktrees.py list
python3 scripts/worktrees.py remove claude p0-telemetry
```

Removal refuses dirty trees and never deletes the local branch or remote branch. Do not use `git worktree remove --force`, `git branch -D`, or `git push --force` to resolve collaboration conflicts. Never remove a worktree containing someone else's uncommitted files. Merge to `main` only through a passing protected PR.

## Remote agent constraints

ChatGPT GitHub connector edits the remote branch and can open PRs but has **no shared local filesystem** with your Antigravity IDE. To review ChatGPT's remote branch locally, first fetch it, then use a distinct read-only review worktree or clone. If an agent runs in its own cloud machine, it must clone and create its own worktree in that environment. Multiple agents using the exact same directory at the same time are prohibited.

## Starting and ending an agent session

Start: read `AGENTS.md`, `.AGENTS/STATE.md`, `.AGENTS/WORKTREES.md`, current issue and skill. Verify current branch, required files and task ownership. **End:** push branch, open a PR, link tests, document unresolved risks and handoff; do not auto-merge or falsely claim tests succeeded.

## Limitations

- Git Worktree does not sync unsaved edits; changes appear on GitHub only after commit/push.
- Worktree directories are intentionally *not* included when cloning or browsing GitHub.
- Files with secrets, hardware traces or host-specific config must stay untracked and out of logs.
- Windows path length limits, mixed-case paths and antivirus filesystem locks can affect local worktrees.
