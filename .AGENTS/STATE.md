# Current implementation state

**Verified implementation snapshot:** 2026-10-23 coordination checkpoint on `agent/codex/collector-shutdown`, explicitly stacked on PR #35 candidate `5360ace`; current main remains `842f877` (merged Core and CPU/Memory tab). Network candidates below are separate branches, not code present in this checkout.

| Field | Value |
| --- | --- |
| Active roadmap delivery | P0 foundations + P1 local Linux baseline |
| Status | Core and CPU/Memory tab integrated; guidance PR #35 and AG-01 lifecycle PR #45 under review; phase-exit/stable-release acceptance unclaimed |
| Application code present | YES: retained core/history/protocol, collector and `apps/lidash` |
| Collector / terminal / headless runnable | YES, tested as unprivileged UID 1000 |
| Observations | CPU, memory, swap, load, uptime, interface/disk counters and rates; optional PSI; CPU/Memory overview tab |
| Data contract | JSON schema 0.1; exact integer counters, collector-local monotonic time, explicit failure states |
| Local environment | Debian 13.6, Linux 6.18.44 x86_64, Rust 1.85.0 |
| Native architecture evidence | PR #35 head `5360ace` passed native x86_64/aarch64 CI; head-specific AG-01 results are recorded on [PR #45](https://github.com/hami9/LiDB/pull/45) and its checks; dedicated reader regression remains separately owned by PR #27 |
| Runtime privileges / services | Read-only local process; no elevated privileges, daemon, listener or outbound runtime connection |
| Release state | Repository prerelease [v0.2.0](https://github.com/hami9/LiDB/releases/tag/v0.2.0) is published; this task publishes no release; stable/performance guarantees unclaimed |
| Review / tests | See the factual [worklog](WORKLOG.md), scoped task evidence and PR checks |
| Current next action | Inspect review/checks on [PR #51](https://github.com/hami9/LiDB/pull/51) (Codex report fix, base Antigravity #48/#46), then claim the reproduced parser repair [issue #50](https://github.com/hami9/LiDB/issues/50) in a new isolated worktree; preserve #35/#45 and other owners' branches |
| Remaining limits | Arbitrary kernel reads remain uncancellable; snapshot/doctor have no deadline; trusted static fixtures, wider Linux/container matrix, measured overhead and release supply-chain hardening remain open |
| Agent delivery | Shared github-style tracked and installed for local future sessions; always-on Antigravity startup and task queue written; no remote/native agent session launched |
| Architecture review | [Actual review](../docs/ARCHITECTURE_REVIEW.md): acyclic layers; maintainer ARM/bootstrap fixes retained; lifecycle/clock/release limits tracked |
| Interactive lifecycle candidate | 5s batch watchdog; terminal restored before 100ms shutdown grace; explicit timeout and permanent worker-admission latch after abandonment; [ADR-023](../docs/DECISIONS.md#adr-023--bounded-interactive-collector-lifecycle-2026-10-16) |
| Review evidence limits | Independent agent lifecycle source review found/fixed a timeout-classification race; no human approval inferred. PR #35 CodeRabbit explicitly skipped review despite a successful status wrapper |
| Network candidates | AG-02 [#46](https://github.com/hami9/LiDB/pull/46) and AG-03 [#48](https://github.com/hami9/LiDB/pull/48) have passing native CI at their recorded heads; report provenance/empty/failure/omission fixes are in [#51](https://github.com/hami9/LiDB/pull/51), under review |
| Network limits | Issue #50 reproduces distinct IPv6 source prefixes collapsing as duplicates; route flags, temporary parser retention and namespace capture remain open. Do not claim a complete routing-policy lookup |

## Public scope

The maintainer authorized a general Linux diagnostic product and completion of Core. [ADR-021](../docs/DECISIONS.md) records that direction. Active vendor-specific, model-serving and distributed-cluster plans and skills were removed. Historical worklogs are preserved. The existing foundation from PRs #9 and #20 is retained, rather than replacing other owners' branches.

## Phase exit ledger

- [ ] P0 — core foundations: implementation integrated in PR #25; phase acceptance not recorded
- [ ] P1 — useful Linux baseline: implementation integrated in PR #25, extended in #34; phase acceptance not recorded
- [ ] P2 — networking diagnostics
- [ ] P3 — terminal and evidence usability
- [ ] P4 — optional deeper instrumentation
- [ ] P5 — public release hardening

Completed slices and test evidence do not by themselves approve a whole phase or public release. Update this ledger after integration with reviewed commits, environments and acceptance evidence.
