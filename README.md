# LiDashBoard (LiDB)

**An open-source, terminal-first Linux and AI infrastructure diagnostic platform.**

LiDashBoard is designed to correlate Linux networking, system resources, GPU telemetry, AI inference workloads, and multi-node communication in one CLI/TUI. The aim is not to display more graphs: it is to show an operator **what changed, what evidence exists, where a bottleneck may be, and which safe diagnostic to run next**.

> **Status: design and agent-workflow bootstrap.** This repository does not yet contain a working collector or TUI. All product capabilities below are *planned*, not delivered.

## What makes it different?

- An **AI-aware troubleshooting workflow** combining network, CPU, memory, GPU, model-serving, and distributed-communication signals.
- First-class planned support for **NVIDIA DGX Spark (GB10)**, with correct accounting for unified memory and ConnectX-based inter-node connectivity.
- Optional, capability-detected instrumentation for **NCCL, RoCE, NVLink/NVSwitch (supported systems)** and model serving runtimes. Metrics are never invented when an integration is unavailable.
- **One binary experience in a terminal** for SSH workflows; modular local collectors and an optional privileged helper.
- **Open-source by design**, minimal privileges, opt-in diagnostics, stable contracts, versioned APIs and honest unsupported-feature reporting.

## Architectural foundation

| Area | Planned approach |
| --- | --- |
| Host and TUI | Rust, Ratatui, Crossterm; Linux x86_64 and aarch64 |
| Kernel networking | Netlink, procfs/sysfs, rtnetlink; optional C/libbpf CO-RE/eBPF |
| Protocol observation | Socket/flow metadata and opt-in bounded diagnostics; no packet payload collection by default |
| GPU and AI | NVIDIA adapters when installed; workload instrumentation via explicit read-only integrations |
| Distributed systems | Topology graph, rank mapping, link health, per-rank timing and straggler correlations |
| Trust boundary | Non-root operator UI; narrowly scoped local collector/helper; read-only baseline |
| Extension model | Versioned capability registry, out-of-process third-party adapters, strict isolation |
| Data model | Typed events with provenance, confidence, freshness, units, availability and time source |

**Hardware distinction:** NVLink-C2C links CPU and GPU *within* DGX Spark. Multiple DGX Spark nodes communicate using ConnectX networking; real GPU-to-GPU NVLink/NVSwitch is a different adapter on supported hardware.

## Documentation

- [Architecture and references](docs/ARCHITECTURE.md)
- [Product requirements and personas](docs/PRODUCT_REQUIREMENTS.md)
- [Feature and capability model](docs/CAPABILITY_MODEL.md)
- [Research-driven ideas](docs/TECHNICAL_IDEAS.md)
- [Technology and dependency policy](docs/TECH_STACK.md)
- [Integrations and platform matrix](docs/INTEGRATION_MATRIX.md)
- [Telemetry and evidence contract](docs/OBSERVABILITY_CONTRACT.md)
- [Security and threat model](docs/SECURITY_MODEL.md)
- [Test and quality strategy](docs/TEST_STRATEGY.md)
- [Architecture decision records](docs/DECISIONS.md)
- [Phased implementation roadmap](ROADMAP.md)

## Autonomous agent collaboration

Read [AGENTS.md](AGENTS.md), then [.AGENTS/README.md](.AGENTS/README.md). Phase state and [worklog](.AGENTS/WORKLOG.md) must be updated after every completed vertical slice. **Agents cannot silently promote proposed features to implemented status.**

## Community

LiDB is licensed under [MIT](LICENSE). Contribution workflow: [CONTRIBUTING.md](CONTRIBUTING.md). Report vulnerabilities according to [SECURITY.md](SECURITY.md). Governance is documented in [GOVERNANCE.md](GOVERNANCE.md).

Maintainers have not yet published releases or measured production overhead. Benchmarks require reproducible evidence from labeled hardware.

## CI/CD and release automation

- [Continuous integration and releases](docs/CI_CD.md) — gated Linux x86_64/aarch64 testing, CodeQL, Dependabot and guarded semantic releases.
- [Main and release-tag protection](docs/BRANCH_PROTECTION.md) — version-controlled rulesets; requires one-time activation by a repository administrator.
- [Agent CI/CD skill](.AGENTS/skills/ci-cd/SKILL.md) — maintain workflow security, SemVer and handoff conventions.

Current implementation state remains pre-P0 application code. No source/binary release exists yet.

## Parallel development with AI agents

The repository supports **separate local Git worktrees for Antigravity, Claude, ChatGPT and other agents**. Every agent gets an independent branch and checkout, while GitHub Issues, PRs and CI Gate provide cross-device coordination. Worktree folders themselves are **Git-ignored and cannot be created on your own machine by a GitHub commit**.

- [Full multi-agent/worktree guide](.AGENTS/WORKTREES.md)
- [Cross-platform Python worktree manager](scripts/worktrees.py)
- [Windows PowerShell wrapper](scripts/worktree.ps1)
- [Linux / Git Bash wrapper](scripts/worktree.sh)
- [Claude Code instructions](CLAUDE.md)
- [Antigravity scoped rules](.agents/rules/multi-agent.md)

Quick start after cloning:

```powershell
.\scripts\worktree.ps1 create antigravity p0-core
.\scripts\worktree.ps1 create claude p0-telemetry
.\scripts\worktree.ps1 list
```

A coding agent must always work inside its own worktree and submit a focused PR; the coordinator alone updates shared status after integration.
