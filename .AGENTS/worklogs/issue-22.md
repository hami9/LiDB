# Issue #22 — public Linux product scope

- Date: 2026-10-09
- Issue: [#22](https://github.com/hami9/LiDB/issues/22)
- Owner/branch: Codex documentation worker, `agent/codex/public-docs`
- Scope: README/roadmap/contribution instructions, product/core documentation, active agent guidance/skills and GitHub contributor templates. Coordinator-owned central state/worklog, root agent instructions, CI policy and new CORE document are excluded.
- Authorization: maintainer requested removal of non-general-product sections and completion of Core; scope recorded in ADR-021.
- Retained: existing core observation/capability/history APIs, pure protocol negotiation contracts, legal files, historical logs and general repository automation.
- Removed: active vendor/runtime/distributed product phases, specialized hardware/fabric skills and related agent/template commitments.
- Runtime boundary: one local read-only Linux collector and CLI/TUI; no running daemon/helper/IPC. Optional deeper Linux networking/eBPF remains future scoped work.
- Documented source/counter limits, fixture labeling, typed failure states, bounded JSON schema, minimum Rust 1.85 and CLI aliases/options/terminal keys supplied by implementation owners.

## Verification

- `python3 scripts/check_docs.py`: PASS, 131 local Markdown targets checked before this task log was added; final count recorded in handoff.
- `git diff --check`: PASS.
- Source/CLI tests: NOT RUN by documentation worker. Core/collector/CLI owners and coordinator validate the integrated implementation; this worktree's starting scaffold is not their final code.
- Native aarch64, minimum-toolchain and production overhead evidence: not established by documentation checks.

## Review notes

- Linux/source correctness: distinguish counters from rates, load from utilization, memory availability from free memory, block counters from filesystem capacity and optional PSI support.
- Telemetry: preserve original APIs and local clock limits; describe snapshot cap/JSON availability accurately from coordinator-owned code.
- Security/privacy: keep read-only local baseline and bounded hostile input; no process command/environment or packet-body collection.
- Reliability: terminal cleanup and meaningful source failure tests remain implementation gates, with no fabricated results.
- Operator UX: installation/removal, default/headless commands, alias behavior, refresh/fixture options and keyboard shortcuts documented.
- This is worker review, not independent phase acceptance, release approval or a claim of validated platform support.
