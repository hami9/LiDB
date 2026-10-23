# Current implementation state

**Verified implementation snapshot:** 2026-10-23, branch `agent/codex/route-report`, explicitly based on route candidate `87531e0` (PR #48), contract candidate #46 and main `842f877`.

| Field | Value |
| --- | --- |
| Active roadmap delivery | Integrated P0/P1 baseline; proposed P2 networking slices |
| Status | Core and CPU/Memory tab integrated; PRs #35/#45 guidance/lifecycle and #46/#48 networking remain under review; issue #49 report-integrity follow-up in progress |
| Application code present | YES: retained core/history/protocol, collector and `apps/lidash` |
| Collector / terminal / headless runnable | YES, tested as unprivileged UID 1000 |
| Observations | CPU, memory, swap, load, uptime, interface/disk counters and rates; optional PSI; route candidate adds IPv4/IPv6 records with independent availability |
| Data contract | Snapshot JSON 0.1 unchanged; route-only report schema 0.1 adds source IDs, mode and batch timing; uncollected generic network sources are explicitly disabled |
| Local environment | Debian 13, Linux 6.18.55 x86_64, Rust 1.85.0, unprivileged UID 1000 |
| Native architecture evidence | Actual PR #48 head `87531e0` passed [native x86_64/aarch64 CI](https://github.com/hami9/LiDB/actions/runs/39275008572) and [CodeQL](https://github.com/hami9/LiDB/actions/runs/39275008682); this follow-up's exact head checks remain separate |
| Runtime privileges / services | Read-only local process; no elevated privileges, daemon, listener or outbound runtime connection |
| Release state | Prerelease [v0.2.0](https://github.com/hami9/LiDB/releases/tag/v0.2.0) exists; this task publishes nothing; stable compatibility and production performance unclaimed |
| Review / tests | See the factual [worklog](WORKLOG.md), scoped task evidence and PR checks |
| Current next action | Resume report-integrity [issue #49](worklogs/issue-49.md), then [parser follow-up #50](https://github.com/hami9/LiDB/issues/50) before routing completeness claims; preserve other owners' candidates |
| Remaining limits | IPv6 source selectors/route flags and temporary parser bounds; namespace capture, cross-collector clock identity and synchronous read deadlines; wider compatibility, measured overhead and release hardening |
| Parallel candidate boundaries | This route branch does not include PR #45's 5s/100ms TUI lifecycle change or PR #35's installed-skill guidance; inspect those actual stacks before integration |
| Persistent continuation | Canonical [queue on PR #45's branch](https://github.com/hami9/LiDB/blob/agent/codex/collector-shutdown/.AGENTS/ANTIGRAVITY.md); shared [github-style](https://github.com/hami9/LiDB/blob/agent/codex/agent-handoff/.agents/skills/github-style/SKILL.md) read/applied |

## Public scope

The maintainer authorized a general Linux diagnostic product and completion of Core. [ADR-021](../docs/DECISIONS.md) records that direction. Active vendor-specific, model-serving and distributed-cluster plans and skills were removed. Historical worklogs are preserved. The existing foundation from PRs #9 and #20 is retained, rather than replacing other owners' branches.

## Phase exit ledger

- [ ] P0 — core foundations: implementation merged in PR #25; phase acceptance pending
- [ ] P1 — useful Linux baseline: implementation merged in PR #25 and #34; phase acceptance pending
- [ ] P2 — networking diagnostics
- [ ] P3 — terminal and evidence usability
- [ ] P4 — optional deeper instrumentation
- [ ] P5 — public release hardening

Completed slices and test evidence do not by themselves approve a whole phase or public release. Update this ledger after integration with reviewed commits, environments and acceptance evidence.
