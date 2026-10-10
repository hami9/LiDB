# Public core execution and acceptance plan

[../ROADMAP.md](../ROADMAP.md) owns phase names. Work from the actual implementation ledger in [STATE.md](STATE.md); a runnable foundation is not an accepted public release.

## P0 — core foundations

Retain the existing `lidb-core` observation/capability/history contracts and pure `lidb-protocol` negotiation. Add bounded snapshots and CLI/TUI presentation through `apps/lidash`. No daemon/helper/IPC is needed. **Exit:** minimum Rust 1.85 verified, formatting/lint/tests pass, contract/error tests, help/version and terminal/headless lifecycle evidence.

## P1 — useful Linux baseline

Implement bounded, unprivileged procfs collection for CPU, memory, load, uptime, interface/disk counters and optional PSI. Snapshot and doctor commands expose typed source failures. **Exit:** real local Linux smoke run, fixture/malformed/missing/denied inputs, counter reset/timing tests and truthful source labels. Record native x86_64/aarch64 evidence separately.

## P2 — networking diagnostics

Read-only routes, sockets, interface metadata and best-effort namespace identity. **Exit:** disposable namespace fixtures, permission/partial-data cases, process privacy and no automatic configuration changes. Active probes need separately scoped opt-in review.

## P3 — terminal and evidence usability

Navigation, narrow-terminal access, bounded history, rate displays and offline evidence exports. **Exit:** keyboard/restore tests, deterministic replay, payload/retention bounds, export privacy and documented limits.

## P4 — optional deeper instrumentation

Add a specific Linux eBPF collector only for a demonstrated operator need. **Exit:** verifier/kernel evidence, explicit activation, bounded maps/events, clean detach, measured overhead and a working unprivileged fallback.

## P5 — public release hardening

Packaging/removal, compatibility/support policy, dependencies, provenance and measured resource use. **Exit:** native platform reports, installed binary smoke tests, supply-chain/license review, accurate docs and independently reviewed artifacts.

## Per-slice requirements

State operator need, ownership, source, permissions, privacy, resource bounds, fallbacks and meaningful acceptance tests. Report commands and observed results; fixtures are not hardware validation. Phase exit requires independent review and recorded evidence. Vendor/runtime, distributed orchestration and automatic-remediation work is outside this roadmap.
