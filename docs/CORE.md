# Linux Core operator and data reference

The runnable baseline is one local, read-only `lidash` process. It reads aggregate procfs data, presents text/JSON or a terminal dashboard, and requires no daemon, account or special privileges. The implementation is a pre-release foundation, not an accepted stable release.

## Workspace boundaries

| Component | Responsibility |
| --- | --- |
| `lidb-core` | Typed observations and capability states, validated snapshots, bounded history and freshness |
| `lidb-collect` | Bounded local procfs reads, strict parsing and stateful counter deltas |
| `apps/lidash` | CLI, doctor, JSON/text export and terminal lifecycle/presentation |
| `lidb-protocol` | Retained pure protocol-version negotiation; no transport or running service |

The core and collector forbid project-owned unsafe Rust. Build dependencies are locked in `Cargo.lock`; `rust-toolchain.toml` selects Rust 1.85.0. Current dependency and native-platform evidence is recorded in the worklog.

## Measurements

| Metric family | Units | Source and interpretation |
| --- | --- | --- |
| `cpu.<field>.ticks` | count | `/proc/stat` aggregate CPU ticks; user/nice already include guest/guest_nice |
| `cpu.busy.percent` | percent | Delta of the first eight aggregate counters, excluding idle and iowait from busy |
| `memory.{total,available,used}.bytes` | bytes | `/proc/meminfo`; used is total minus the kernel's `MemAvailable` estimate |
| `swap.{total,free,used}.bytes` | bytes | `/proc/meminfo`; missing fields remain unavailable |
| `load.{1m,5m,15m}` | count | `/proc/loadavg` averages of runnable and uninterruptible tasks; not CPU percentages |
| `uptime.seconds` | seconds | `/proc/uptime`; fractional elapsed uptime |
| `network.state` | count | Number of parsed interfaces, or a source failure state |
| `network.<interface>.{rx,tx}.bytes` | bytes | `/proc/net/dev` cumulative byte counters per interface |
| `network.<interface>.{rx,tx}.bytes_per_second` | bytes per second | Counter deltas over the collector-local monotonic interval |
| `disk.state` | count | Number of parsed block devices/partitions, or a source failure state |
| `disk.<device>.{read,write}.bytes` | bytes | `/proc/diskstats` cumulative sectors multiplied by the kernel's fixed 512-byte accounting unit |
| `disk.<device>.{read,write}.bytes_per_second` | bytes per second | Counter deltas over the collector-local monotonic interval |
| `pressure.{cpu,memory,io}.some.avg10.percent` | percent | `/proc/pressure/*` kernel PSI average; absent PSI degrades independently |

Disk entries include partitions. Do not sum a parent block device and its partitions into one host total. Network interfaces can represent stacked/virtual paths, so summing them can also double-count traffic. Device names preserve common ASCII characters and percent-encode other UTF-8 bytes, including `%`; for example `vpn+prod` becomes `vpn%2Bprod`. Encoding is reversible and collision-free, with a 100-byte encoded-name limit. See the [collector reference](../crates/lidb-collect/README.md).

The visible procfs view determines coverage in containers and namespaces. These metrics do not promise host-wide visibility, cgroup limits, filesystem free space, process attribution or a diagnosis of root cause.

## Rates, time and missing data

One-shot commands take two samples; the initial rate is `temporarily_unavailable`. Newly observed devices also need a baseline. A decreasing counter creates an explicit reset error and a new baseline. A failed source discards its old rate baseline. No interpolation or unavailable-to-zero conversion occurs. CPU iowait counters can decrease on Linux; that creates a visible gap.

Every snapshot carries schema `0.1`, `clock_source: "collector_monotonic"`, collection duration, `live` or `fixture` mode, and a bounded metric list. Nanoseconds are measured from construction of that collector. They are not Unix time and cannot be compared with a different collector, process or boot. A batch timestamp marks the start of its sequential reads; `collection_duration_ns` exposes the read window, rather than claiming simultaneous hardware measurements.

JSON preserves exact unsigned integer counters. Derived quantities are finite floating point numbers. Each observation retains its name, source, unit and timestamp. An available state has a numeric `value`; other states use `value` for their explanatory string. Consumers must inspect `status` first. See [telemetry contract](OBSERVABILITY_CONTRACT.md). No unchecked public snapshot deserializer is exposed.

## Resource and privacy boundaries

Each of nine source files is limited to 1 MiB. Interface and disk sources accept at most 64 entries each; excess entries invalidate that source with a reason. The baseline emits at most 538 observations, below the core limit of 2048. Snapshot overflow increments `dropped_metrics`. This counter is distinct from UI updates replaced before display and history samples evicted by capacity.

The application stores no persistent history, launches no shell command or connectivity probe, opens no listener and makes no outbound network request. It reads no packet contents, credentials, process environments or process arguments. An explicit custom procfs directory is fixture input; use a trusted, static directory, since the collector does not provide a sandbox for concurrent adversarial directory replacement.

## Validation and remaining work

Fixture tests validate malformed/oversized input, permission/missing states, exact counters and reset behavior. A real Linux smoke test validates readable standard procfs sources. CLI and terminal tests establish output and lifecycle behavior separately. TestBackend fixtures are not live-host measurements or a performance benchmark. Native architecture CI and local test evidence belong to [agent state](../.AGENTS/STATE.md) and the [worklog](../.AGENTS/WORKLOG.md).

The next roadmap slices cover deeper Linux networking and operator usability. Release hardening still needs a wider Linux/container compatibility matrix, measured resource use, dependency advisories/SBOM and verified published provenance.
