# Current implementation state

**Verified implementation snapshot:** 2026-11-06, branch `main` at commit `56e54bb`, integrating the completed Phase 2 (Networking Diagnostics) and Phase 3 (Terminal & Evidence Usability) feature stack (PRs #46, #48, #52, #54, #56, #58, #60).

| Field | Value |
| --- | --- |
| Active roadmap delivery | Completed Phase 2 (Networking Diagnostics) and Phase 3 (Terminal & Usability) on `main` |
| Status | All stacked PRs (#46, #48, #52, #54, #56, #58, #60) verified via native CI / CodeQL and squashed into `main` |
| Application code present | YES: `lidb-core`, `lidb-collect`, `lidb-protocol`, and `apps/lidash` fully runnable |
| Collector / terminal / headless runnable | YES, tested across unprivileged local Linux and Windows environments |
| Observations | CPU, memory, swap, load, uptime, disk stats; interfaces, network namespaces, IPv4/IPv6 routes, TCP/UDP sockets |
| Data contract | Snapshot JSON 0.1, NetworkReport schema 0.1, SocketReport schema 0.1, InterfaceReport schema 0.1, EvidenceExport schema 0.1 |
| Privacy constraints | Strict read-only procfs/sysfs; zero packet payloads, zero decrypted TLS, zero environment variables, zero process arguments, zero secrets (Rule R07) |
| Native architecture evidence | Native x86_64 and aarch64 CI passed on all PRs; CodeQL Actions/Rust clean; all 103 workspace tests pass |
| Runtime privileges / services | Read-only local process; no elevated privileges, daemon, listener or outbound runtime connection |
| Release state | Tags up to `v0.4.0` published; release planner indicates `v0.5.0` (minor bump) on next release |
| Review / tests | 103 unit & integration tests passing (`cargo test --workspace --locked --all-targets`), zero Clippy warnings, 147 Markdown links verified |
| Current next action | Phase 4 (Optional deeper instrumentation / eBPF) or Phase 5 (Release hardening & packaging) scoping |
| Remaining limits | Bounded history ring buffer capped at 120 samples (~700 KB RAM); route cache bounded at 2048 records; socket tables bounded at 2048 entries |

## Public scope

The maintainer authorized a general Linux diagnostic product and completion of Core and Networking Diagnostics. [ADR-021](../docs/DECISIONS.md) records that direction. Active vendor-specific, model-serving and distributed-cluster plans and skills were removed. Historical worklogs are preserved.

## Phase exit ledger

- [x] P0 — core foundations: implementation merged in PR #25
- [x] P1 — useful Linux baseline: implementation merged in PR #25 and #34
- [x] P2 — networking diagnostics: PRs #46, #48, #52, #54, #56, #58 merged into `main`
- [x] P3 — terminal and evidence usability: PR #60 (export, bounded history, ASCII banner) merged into `main`
- [ ] P4 — optional deeper instrumentation
- [ ] P5 — public release hardening

Completed slices and test evidence do not by themselves approve a whole phase or public release. Update this ledger after integration with reviewed commits, environments and acceptance evidence.
