# LiDB core architecture

LiDB is a terminal-first, local Linux diagnostic product. The maintainer-authorized public scope replaces the earlier vendor-specific infrastructure blueprint; see [ADR-021](DECISIONS.md#adr-021--public-linux-product-scope-2026-10-09).

## Running baseline

```text
Linux procfs sources
        |
        v
bounded read-only collector -> lidb-core observations / snapshot
                                         |
                                         v
                           lidash text / JSON / terminal UI
```

The CLI owns presentation and lifecycle. The `lidb-collect` Linux collector owns source reads, parsing, sampling and capability reporting. `lidb-core` owns typed domain contracts, availability, units, provenance, snapshots and bounded single-series history. These boundaries are ordinary in-process library calls. Collection must not block terminal input indefinitely.

`lidb-protocol` is retained from the existing core: it implements pure major/minor compatibility negotiation. It opens no socket and provides no serialization, peer authentication or running IPC transport. The baseline has no daemon, service installation, privileged helper, remote listener or background updater.

## Sources and semantics

| Host observation | Source | Interpretation |
| --- | --- | --- |
| CPU counters and sampled activity | `/proc/stat` | Cumulative counters; utilization needs consecutive compatible samples |
| Memory and swap | `/proc/meminfo` | Explicit bytes; `MemAvailable` is different from free memory |
| Load averages | `/proc/loadavg` | Runnable/uninterruptible workload averages, not CPU utilization percentages |
| Uptime | `/proc/uptime` | Host uptime from the selected procfs view |
| Interface counters | `/proc/net/dev` | Cumulative byte/packet/error/drop counts in the visible namespace |
| Block-device counters | `/proc/diskstats` | Device-level cumulative I/O statistics, not filesystem capacity |
| Pressure stalls | `/proc/pressure/{cpu,memory,io}` | Optional kernel PSI averages/counters with explicitly reported support |

The collector reads at most 1 MiB per source file and uses configurable procfs roots for deterministic fixtures. `LinuxCollector::new(proc_root)` owns a collector-local clock and `sample()` returns a `Snapshot`. A fixture is labeled test evidence, and cannot establish real-host support. Source files, input sizes, collection count and refresh interval are bounded. A missing, malformed, denied or changing source affects its observations visibly; it must not silently invalidate every other collector.

Derived rates require elapsed monotonic time and two counter samples. First-sample, zero-interval and reset/wrap conditions must yield an explicit unavailable state. Aggregating block devices or interfaces without understanding layered devices can double-count traffic; expose per-device evidence rather than inventing a host-total throughput.

## Data contracts

Existing `MetricObservation<T>` values carry a metric name, source, unit, collector-local monotonic timestamp and `MetricState<T>`. `CapabilityRegistry` provides duplicate-safe, deterministic registration with failure reasons. Snapshot consumers preserve these contracts instead of substituting unavailable values with zero. A `Snapshot` holds at most 2048 metrics, rejects duplicate names, future observation timestamps and non-finite/negative/out-of-range numeric values, and reports capacity omissions through `dropped_metrics`. Integer counters remain exact `u64` values in JSON.

`TelemetryHistory<T>` caps the number of samples and rejects mixed metric/source or regressing timestamps. It does not bound arbitrary payload bytes or prove that samples share a boot and clock domain. Producers must separately bound payloads and keep one local clock domain. [Core contracts](P0_CORE_CONTRACTS.md), [history contract](P0_TELEMETRY_HISTORY.md) and [observability contract](OBSERVABILITY_CONTRACT.md) describe these limits.

Text, JSON and TUI views use the same collected snapshot. JSON is a pre-stable machine interface; document schema changes and preserve stable availability names. There is no inference of a verified cause from one metric threshold.

## Terminal lifecycle

`lidash` selects an interactive dashboard when a terminal is available and produces a finite text snapshot when output is redirected. Explicit `snapshot`, `doctor` and `tui` commands make behavior discoverable. The UI restores terminal modes on normal quit and handled failures; unsupported or narrow terminals receive actionable behavior. Color is supplementary.

Sampling interval is an operator setting within 250–60000 ms. Rendering and collection do not require a database, browser, network access or root. No history is persisted unless a later, documented feature introduces it.

## Trust and privacy

Kernel-derived text, device names and fixture files are untrusted input. Parse them defensively, reject invalid/non-finite numeric fields, bound reads and sanitize terminal control characters before display. Do not inspect process environments, command lines, packet contents or secrets. Export only the observations the operator requested.

No commands alter routes, firewalls, namespaces, services, kernel settings or storage. Active network probes and optional eBPF require separate review, explicit activation and safe fallback. [SECURITY_MODEL.md](SECURITY_MODEL.md) defines the applicable threat model.

## Future boundary

Deeper network state, evidence history/export and opt-in eBPF are future slices in [ROADMAP.md](../ROADMAP.md). Introduce a service or IPC only when a concrete use case justifies its privilege and lifecycle cost. Vendor runtimes, distributed workloads, orchestration and automatic remediation are outside this product scope.

## Compatibility and evidence

Rust 1.85 is the minimum userspace toolchain. The live baseline depends on the Linux procfs interfaces it actually reads, not a blanket minimum-kernel claim. PSI and individual fields are capability-tested. Linux x86_64 local evidence and aarch64 CI evidence must be recorded separately; cross-compilation or fixtures do not substitute for native validation. No overhead or production readiness target is claimed as measured without a reproducible report.
