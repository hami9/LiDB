# Product roadmap and implementation gates

**Status: P0 NOT STARTED.** This roadmap describes intended work, not completed functionality. No phase is accepted based on code generation alone.

| Phase | Deliverable | Required evidence and exit gate |
| --- | --- | --- |
| P0 — foundations | Cargo workspace, architecture boundaries, CI, schemas, TUI shell, capability model, fixtures | Compile and test on Linux; zero fake metrics; unprivileged startup |
| P1 — host baseline | CPU/memory/disk/network-interface and procfs collectors | Real Linux integration tests; typed unsupported/missing-permission states |
| P2 — networking | Interfaces, routes, sockets, DNS/probe diagnostics and namespaces | Isolation tests with synthetic namespaces; bounded diagnostics; no auto-mutation |
| P3 — optional eBPF | CO-RE probes, loader, fallback, kernel compatibility matrix | Verifier success on supported kernels; measured overhead; no privileged TUI |
| P4 — event engine and UX | History, timelines, correlations, search, export, UX polish | Deterministic replay; accessibility/SSH tests; bounded memory |
| P5 — single-node AI | GPU adapters, GB10 unified-memory model, inference KPIs | Tested on real supported GPU or explicitly marked not validated |
| P6 — AI serving adapters | Pluggable endpoints for vLLM, TGI, Triton, Ollama etc. | Contract tests; permission/rate limits; never scrape credentials |
| P7 — multi-node fabric | Node discovery, ConnectX link diagnostics, topology, rank mapping | 2+ node tests; clock uncertainty reported; transport evidence |
| P8 — distributed AI RCA | NCCL timing correlation, straggler evidence, placement advice | Reproduced test scenarios; calibrated confidence, false positive evaluation |
| P9 — extensions and release | Capability registry, plugin policy, supply chain, packaging, semver | Threat-model review, signed release artifacts, SBOM, docs and upgrade tests |
| P10 — public beta | Performance, compatibility, accessibility, support triage | Reproducible benchmark matrix and multi-review approval |

**P0 is the next phase.** Implement the smallest useful end-to-end vertical slice before expanding collectors. A feature remains `experimental` until tested on its target platform.

## Milestones and feature gates

Each issue must declare: user problem, required capability, data source, privilege needs, cost/overhead budget, privacy category, acceptance tests, fallback behavior, compatibility impact, documentation change and rollout flag. A phase is finished only after an independent review.

Never block baseline Linux operation on an optional NVIDIA SDK, container runtime, eBPF or cluster service. Optional collection failures must degrade locally and visibly.

For executable agent instructions see [.AGENTS/PHASES.md](.AGENTS/PHASES.md).
