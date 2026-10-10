# Public product requirements

## Audience and problem

LiDB serves Linux users, developers, VPS owners and system operators who need useful host evidence in a terminal or SSH session. They should be able to inspect resource pressure and interface activity without installing an agent service, supplying cloud credentials or granting root.

## Operator jobs

1. Inspect CPU activity, memory availability, load, uptime, disk/interface counters and pressure in one place.
2. Tell observed values from missing, denied, unsupported or malformed data.
3. Copy a reproducible machine-readable snapshot into a local investigation.
4. Use the same baseline interactively and in redirected scripts.
5. Understand collection sources, limitations and permissions before requesting deeper diagnostics.

## Core requirements

- One `lidash` CLI with terminal dashboard, finite text/JSON snapshot and capability doctor.
- Rust core contracts for units, source, monotonic time, availability and bounded histories/snapshots.
- Read-only Linux collection with per-source failure reporting and bounded file reads.
- Configurable refresh interval with a bounded allowed range; custom procfs roots for fixtures.
- Clean terminal restoration, keyboard navigation and usable non-interactive output.
- Build/install/removal instructions, honest platform labels and reproducible tests.

## Nonfunctional requirements

No root, mandatory daemon, database, cloud account, external service or outbound application traffic. No host mutation, packet capture, process-environment/command-line collection or hidden persistence. Input and resource usage are bounded. Missing metrics remain missing; untested platforms and unmeasured performance remain labeled accordingly.

## Product boundaries

Vendor-specific compute telemetry, model-serving adapters, multi-node fabrics, schedulers, remote fleet management, browser dashboards and automatic remediation are outside scope. Deeper Linux networking and eBPF can follow only when they solve a concrete general Linux operator problem and retain a useful unprivileged fallback.

## Acceptance and launch

Core acceptance requires a real Linux smoke run, meaningful source/error tests, CLI help and JSON behavior, terminal lifecycle verification and current documentation. Public release additionally requires validated platform claims, measured resource use, packaging/removal checks, dependency/license review and release evidence. See [test strategy](TEST_STRATEGY.md) and [roadmap](../ROADMAP.md). No release is claimed by this document.
