# Current implementation state

**Verified implementation snapshot:** 2026-11-10, branch `main` at commit `34bf924`, integrating the completed Phase 2 (Networking Diagnostics), Phase 3 (Terminal & Evidence Usability), Phase 4 (Optional read-only eBPF instrumentation, PR #64), Phase 5 (Public release hardening, packaging guide, standalone installer script, support matrix, license SBOM, PR #66), dependency updates (PR #42 `serde_json 1.0.154`, PR #37 `unicode-segmentation 1.13.3`, PR #69 `crossterm 0.29.0` and `signal-hook 0.4.6`), interactive eBPF socket lifecycle dashboard tab (PR #70), installer tag discovery hardening (PR #71), interactive Network tab socket table scrolling (PR #72), universal detail modal view across tabs (PR #73), live interface bandwidth rates (PR #74), CPU/memory subsystem deep-dive inspector modal with uptime formatting (PR #75, release v0.0.18), and tab-aware navigation with routing table inspection (Task AG-20).

| Field | Value |
| --- | --- |
| Active roadmap delivery | Completed all roadmap phases (Phase 0, 1, 2, 3, 4, and 5) on `main` |
| Status | All roadmap phases verified via native CI / CodeQL and squashed into `main` |
| Application code present | YES: `lidb-core`, `lidb-collect`, `lidb-protocol`, and `apps/lidash` fully runnable |
| Collector / terminal / headless runnable | YES, tested across unprivileged local Linux and Windows environments |
| Observations | CPU, memory, swap, load, uptime, disk stats; interfaces, network namespaces, IPv4/IPv6 routes, TCP/UDP sockets, interface error/drop counters, optional read-only TCP lifecycle instrumentation (`EbpfReport`), interactive TUI eBPF tab, navigable socket tables, detailed modal inspectors across all tabs, live interface bandwidth rates, formatted uptime durations, active routing table inspector, tab-aware scrolling |
| Data contract | Snapshot JSON 0.1, NetworkReport schema 0.1, SocketReport schema 0.1, InterfaceReport schema 0.1, EvidenceExport schema 0.1, EbpfReport schema 0.1 |
| Privacy constraints | Strict read-only procfs/sysfs/eBPF tracepoints; zero packet payloads, zero decrypted TLS, zero environment variables, zero process arguments, zero secrets (Rule R07) |
| Native architecture evidence | Native x86_64 and aarch64 CI passed on all PRs; CodeQL Actions/Rust clean; all 127 workspace tests pass |
| Runtime privileges / services | Read-only local process; no elevated privileges, daemon, listener or outbound runtime connection |
| Release state | Clean patch-only release versioning (0.0.1+): releases v0.0.1 through v0.0.18 published with verified cross-architecture binaries |
| Review / tests | 127 unit & integration tests passing (`cargo test --workspace --locked --all-targets`), zero Clippy warnings, 157 Markdown links verified, 23 CI Python tests passing |
| Current next action | Merge PR for tab-aware navigation, complete network subsystem inspection, and modal text scrolling, releasing v0.0.19 |
| Remaining limits | Bounded history ring buffer capped at 120 samples (~700 KB RAM); route cache bounded at 2048 records; socket tables bounded at 2048 entries; eBPF event queue bounded at 2048 entries |

## Public scope

The maintainer authorized a general Linux diagnostic product and completion of Core and Networking Diagnostics. [ADR-021](../docs/DECISIONS.md) records that direction. Active vendor-specific, model-serving and distributed-cluster plans and skills were removed. Historical worklogs are preserved.

## Phase exit ledger

- [x] P0 — core foundations: implementation merged in PR #25
- [x] P1 — useful Linux baseline: implementation merged in PR #25 and #34
- [x] P2 — networking diagnostics: PRs #46, #48, #52, #54, #56, #58 merged into `main`
- [x] P3 — terminal and evidence usability: PR #60 (export, bounded history, ASCII banner) merged into `main`
- [x] P4 — optional deeper instrumentation: PR #64 (safe eBPF CO-RE tcp lifecycle, capability fallback, CLI integration) merged into `main`
- [x] P5 — public release hardening: PR #66 (standalone installer script, packaging guide, support matrix, license SBOM, test suite) merged into `main`

Completed slices and test evidence do not by themselves approve a whole phase or public release. Update this ledger after integration with reviewed commits, environments and acceptance evidence.
