# Phase execution and acceptance plan

See [../ROADMAP.md](../ROADMAP.md) for product-level phase gates. Each phase below has a technical delivery boundary and cannot be skipped without a recorded dependency decision.

## P0 — workspace foundation

Build a Rust workspace with `lidash`, `lidashd`, shared core/protocol modules and a no-root TUI shell; minimum Linux/kernel/toolchain decision; capability and typed availability enums; fixtures; CI for formatting, lint, unit and docs tests. No mandatory GPU SDK or eBPF. **Exit:** workspace builds/tests on chosen Linux baselines, CLI `--help` and headless output work; no invented live metrics; graceful daemon-absent state.

## P1 — Linux host telemetry

Implement safe procfs/sysfs/cgroup v2/PSI collectors with source timestamps, permissions and typed errors. **Exit:** per-field missing-data cases and Linux integration tests on actual supported hosts; performance baseline documented.

## P2 — networking state

Netlink interfaces/routes, socket and namespace attribution, container-aware views and safe DNS/connectivity probes. **Exit:** reproducible namespaces/veth route fixture and error cases; no automatic nftables/routing mutations.

## P3 — eBPF collector

C CO-RE programs, optional BPF loader/helper, fallback, attach/detach lifecycle and lost-event counter. **Exit:** verifier, kernel compatibility, privilege and overhead evidence; TUI remains unprivileged.

## P4 — correlation + TUI maturity

Bounded event streaming, local timelines, searchable diagnosis history, redacted export and accessible terminal views. **Exit:** determinism, time semantics, retention, overload/drop tests, narrow-terminal snapshots.

## P5 — GPU + GB10 awareness

Optional NVML/DCGM adapters, non-fake unsupported fields, GPU device mapping and unified-memory model. **Exit:** validated platform matrix including actual DGX Spark before claiming support.

## P6 — model-serving adapters

Opt-in documented read-only adapters for serving KPIs: tokens/s, queue size, TTFT/TPOT, batching and cache where exposed. **Exit:** auth handling, rate limiting, schema contracts, no sensitive payloads.

## P7 — multi-node topology

Securely authenticated node observations, ConnectX link health, RDMA/RoCE capability, rank and NIC mapping, clock quality. **Exit:** tested 2+ nodes, transport/type evidence, no claim that Spark inter-node traffic is NVLink.

## P8 — NCCL and distributed RCA

Opt-in communication instrumentation, collectives/rank timing, straggler analysis, contradictory evidence, placement recommendations. **Exit:** controlled, repeatable incidents and quantified false-positive risk.

## P9 — plugin/release hardening

Versioned adapter protocol, resource isolation, packaging, audited supply chain, release signatures, SBOM, compatibility migration. **Exit:** plugin crash/version mismatch tests; maintainer security review.

## P10 — public beta

Operator documentation, installation/uninstallation, structured support and benchmark report, end-to-end testing on claimed hardware and five-angle approval. **Exit:** publish an accurately scoped beta release; no unsupported marketing claims.

## Per-phase mandatory deliverables

Scope statement, module manifest, tests (positive/negative), permission and privacy assessment, actual performance findings, fallback UX, documented support states, worklog and reviewer approval.
