# Public product roadmap

LiDB is a local Linux diagnostic product. This roadmap tracks small, independently testable deliveries; a phase is accepted only when its evidence and review gates pass. The repository's authoritative implementation ledger is [.AGENTS/STATE.md](.AGENTS/STATE.md).

## Core delivery

The first public core combines the existing capability, observation, bounded-history and pure protocol-contract libraries with unprivileged Linux collection and an operator CLI/TUI. No daemon, privileged helper or IPC transport is required for this baseline. The internal workspace version is pre-release; this is not a public release declaration.

| Phase | Scope | Acceptance evidence |
| --- | --- | --- |
| P0 — core foundations | Rust workspace, typed availability/provenance, bounded snapshots/history, CLI/TUI shell, pure contracts and CI | Format/lint/tests, schema/error tests, help/version, terminal restoration and redirected output |
| P1 — useful Linux baseline | CPU, memory, load, uptime, disk/interface counters and PSI; text/JSON snapshot; capability doctor | Real local Linux smoke run, fixture parsing, malformed/missing/denied sources, rate/reset handling and bounded reads |
| P2 — networking diagnostics | Read-only routes, sockets, interface metadata and best-effort namespace attribution | Disposable namespace fixtures, source/permission fallbacks and privacy review; active probes separately opt-in |
| P3 — terminal and evidence usability | Narrow-terminal handling, navigable views, bounded history, truthful derived rates and offline exports | Terminal tests, deterministic replay, collection budgets, export review and documented limits |
| P4 — optional deeper instrumentation | Scoped Linux eBPF only where baseline sources cannot answer a demonstrated operator need | Kernel verifier/compatibility evidence, explicit activation, bounded overhead, fallback and clean detach |
| P5 — public release hardening | Packaging, installation/removal, compatibility matrix, dependency review, supply-chain evidence and support policy | Native architecture test reports, measured resource use, release review and verified artifacts |

P0/P1 work is delivered as bounded slices, rather than treating all future collectors and interfaces as one implementation task. x86_64 and aarch64 are intended Linux targets; pending CI or untested environments remain explicitly unvalidated. There is no vendor-specific, model-runtime, cluster or automatic-remediation phase.

## Issue and feature gates

Each task states the operator problem, data source, permissions, resource bounds, privacy category, acceptance tests, fallback and compatibility effect. Optional features must never block the ordinary procfs baseline. A successful build proves compilation on that environment, not hardware support or measured overhead.

For executable agent instructions see [.AGENTS/PHASES.md](.AGENTS/PHASES.md). [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) owns the runtime boundary; this file owns the phase names.
