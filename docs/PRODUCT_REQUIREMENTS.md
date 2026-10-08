# Product requirements (PRD)

Status: proposed | Owner: maintainers | Audience: contributors, coding agents, operators

## Problem and audience

LiDashBoard serves (1) Linux/VPS operators debugging connectivity, (2) engineers operating Docker/network namespaces, (3) AI inference developers tracking latency and throughput, (4) DGX Spark workstation and cluster operators, and (5) users working exclusively over SSH or small terminals.

## Jobs to be done

1. **Explain a failing service connection:** identify process, socket, namespace, route, DNS and applicable firewall signals without pretending every packet follows one fixed pipeline.
2. **Explain slow inference:** correlate TTFT/TPOT, request queue, CPU, unified-memory pressure, GPU and network signals in a shared time window.
3. **Explain uneven GPU/cluster throughput:** show per-node and per-rank observations, collective timings and fabric health when instrumentation exists.
4. **Safely investigate:** default to read-only; preview requested changes and clearly disclose risk, privileges and rollback.
5. **Work offline and under SSH:** no cloud account or remote telemetry requirement for a useful local baseline.

## Functional product requirements

- CLI and navigable, keyboard-first TUI; deterministic machine-readable export.
- Capability discovery, platform/permission matrix and honest unavailable state.
- Read-only process, socket, interface, route, system-pressure and GPU observations.
- Correlation and RCA with evidence links, alternatives and uncertainty.
- Optional local timelines with configurable retention and resource budgets.
- Pluggable AI-runtime adapters; avoid model output, input and token/prompt payload collection.
- Cluster visibility without claiming to be a scheduler. Recommendations require evidence.
- Stable schemas and evolution policy; safe plugin lifecycle and supervised adapters.
- Documentation, security reporting, packaging, reproducible validation.

## Product-level nonfunctional targets

- Never require root for the terminal UI. Privileged functionality uses a small explicitly enabled component.
- Work with missing eBPF, GPU, NVIDIA libraries, containers and network access.
- Never silently drop security-relevant errors or silently invent missing metrics.
- Configurable refresh/sample rates and retention. Support `--no-color`, narrow terminals and screen readers as feasible.
- Provide installation and removal procedures with no unexpected daemon enablement.
- No outbound network traffic without an explicit feature that requires it.
- Honor privacy choices and redact secrets from logs, crash reports and support bundles.

## Non-goals for initial public releases

LiDB is not a packet interception platform, firewall replacement, GPU training orchestrator, model scheduler, remote execution service, enterprise fleet controller or browser dashboard. Automatic remediation and cross-host credential management are deferred pending a dedicated threat model.

## Success metrics (proposed, to validate)

Measure: time-to-diagnosis in reproducible scenarios; fraction of claims with provenance; collector CPU/RSS overhead and dropped events; graceful fallback rate; test coverage by platform and permissions; usability on SSH; false positive rate for RCA hypotheses. Do not claim target performance is achieved before hardware measurements.

## Launch criteria

A feature requires user documentation, discoverable limitations, CLI/TUI visibility, versioned contract, tests, security review and compatibility labeling (`stable`, `experimental`, `unsupported`, `disabled`). Public beta must have installation guides, license, vulnerability contact, signed artifacts, changelog and operational support policy.
