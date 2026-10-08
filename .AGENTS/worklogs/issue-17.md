# Issue 17: Connect TUI Capabilities

- Owner: Codex; approved P0 UI/core integration slice.
- Branch: `agent/codex/core-adapter`; isolated worktree: `.worktrees/codex/core-adapter`.
- Task state: implemented locally; Rust CI validation and maintainer review pending.
- Base: integration snapshot `76aa72c3adc939edca15204b4656a10b91e4027d`, containing the exact #9/#11/#13 heads and CI implementation `3210e66`. PR stacks on `agent/codex/tui-validation`; do not merge that validation branch into main.
- Scope: local core path dependency, GPU capability presentation adapter, application injection, GPU banner, tests, prototype docs and [ADR](../../docs/adr/0001-core-capabilities.md).
- Acceptance: display every core state and reason; missing registration remains unprobed; available capability never implies a live sample; ticks and explicit demo toggles preserve the capability snapshot; headless narrow/Unicode renders, locked native x86_64/aarch64 CI and smoke checks pass.
- No owner branch edits, core schema changes, metrics fabricated, collectors, listeners, GPU access, dependency upgrades or privileged operations. The adapter consumes metadata already supplied by the core.
- Local Cargo: NOT RUN because no toolchain is installed. Manifest/lock parsed with Python standard-library `tomllib`; `git diff --check` passed. CI results must be added after execution.
- State ownership: this task record tracks the slice while CI PRs #15/#16 and runtime PRs remain open. Central phase state/worklog and shared changelog consolidation belong to the integration coordinator after merge.

## Five-axis review

- Kernel/networking: no host probes or network operations; core state is an injected registration snapshot.
- AI/GPU/fabric: no hardware claims; GPU observations stay unprobed and empty even when capability is available.
- Security/privacy: existing non-root operation; no new external crate, secret access, transport or unsafe code.
- Reliability: exhaustive core state matching; absent registration preserved; deterministic render tests and no background task.
- UX/ecosystem: capability and telemetry badges are distinct, reason/source visible, fixtures remain explicitly selected. Root workspace remains separate. Long reasons may be truncated by the prototype's existing panel width.

Self-review is not independent approval. Hardware, authenticated IPC, MSRV, real-terminal lifecycle and full telemetry adapter validation remain pending.
