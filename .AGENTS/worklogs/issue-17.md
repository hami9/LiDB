# Issue 17: Connect TUI Capabilities

- Owner: Codex; approved P0 UI/core integration slice.
- Branch: `agent/codex/core-adapter`; isolated worktree: `.worktrees/codex/core-adapter`.
- Task state: implemented and CI Gate passed; [PR #18](https://github.com/hami9/LiDB/pull/18) ready for review, security analysis and maintainer decision pending. Not merged.
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

## Executed checks

- Windows: `python -B scripts/check_docs.py`, exit 0, 133 local Markdown links passed. Python standard-library TOML parsing verified the single local core dependency and lock entry; `git diff --check` passed.
- Initial [CI run](https://github.com/hami9/LiDB/actions/runs/37781078031) at `97612bbf3e281cea3fc2eb4cf3a4043d2308c95f` failed on Rust formatting. Applied the formatter's reported layout changes in `8c79444`; no runtime or test failures were hidden.
- [CI run](https://github.com/hami9/LiDB/actions/runs/37781449098) passed at `150d6bf18348548d59f26d697c50ca9ab479f259`. Native Linux x86_64 and aarch64 ran root and TUI formatting, locked Clippy, locked target tests and documentation commands. TUI headless and smoke commands passed. Per architecture: 14 root tests and 26 TUI tests passed, including six new adapter tests. Documentation suites contain zero cases; no documentation-test coverage is implied.
- The six tests validate all core state badges/reasons, missing registration and immutable snapshot, no live sample after ticks, explicit fixture roundtrip, Unicode/narrow panel rendering and the complete 80x24 dashboard showing both badges.
- CI also passed all 24 Python policy tests and Windows worktree checks. Locked TUI compilation confirms the path dependency and lockfile are accepted alongside the root workspace.
- Core runtime, IPC, CI and other owners' branches remain unchanged by this PR. The complete prototype still contains existing labeled fixtures; this slice integrates only GPU capability metadata, not full telemetry values or collectors.
