# Copy-ready implementation kickoff

LiDashBoard (LiDB) is a terminal-native, open-source, local Linux diagnostic product. It uses Rust for userspace; optional C/eBPF belongs to future scoped instrumentation.

Read `AGENTS.md`, `.AGENTS/SYSTEM_PROMPT.md`, `.AGENTS/RULES.md`, `.AGENTS/STATE.md`, `.AGENTS/PHASES.md`, `.AGENTS/QUALITY_GATES.md`, `docs/ARCHITECTURE.md`, `docs/PRODUCT_REQUIREMENTS.md` and relevant local skills. Inspect the actual code, branch and existing changes; preserve the implemented core rather than replacing it with speculative scaffolds.

Choose one bounded active-phase slice and declare files, public contracts, source/permission/privacy boundaries, resource limits, meaningful tests and compatibility. Implement it, verify actual results and update operator docs. No root TUI, mandatory daemon, hidden listener, automatic network changes or invented metric/benchmark claims.

Review Linux source correctness, telemetry integrity, security/privacy, reliability/performance and operator UX. Record unavailable platform tests honestly. Do not call a phase accepted until its gates and independent review have evidence.

## Parallel work

Read [WORKTREES.md](WORKTREES.md) and reserve a unique issue/branch/file scope. Each implementation worker uses an isolated worktree. The coordinator alone integrates shared schemas, CI and central state/worklog. Workers put evidence in scoped task logs and hand off a focused commit/PR as instructed.
