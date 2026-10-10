# lidb-collect

Bounded, read-only Linux host telemetry for `lidash`. The collector runs without
privilege escalation, network requests, active probes, or process payload reads.
It supports Linux x86_64 and aarch64; other targets produce typed `unsupported`
observations without filesystem reads. This is an aggregate host baseline, with
no GPU, eBPF, cgroup, process attribution, or diagnosis implementation.

```rust
use lidb_collect::LinuxCollector;

let mut collector = LinuxCollector::new("/proc");
let snapshot = collector.sample();
assert!(snapshot.get("cpu.busy.percent").is_some());
```

`sample()` returns a snapshot even when individual sources fail. Every observation
contains a stable name, source, unit, collector-local monotonic timestamp, and
explicit availability. All observation timestamps equal the batch start time;
`collection_duration_ns` measures the time spent reading that batch. Timestamps
are comparable only within the same collector lifetime. Procfs sources are read
sequentially; the shared batch timestamp does not imply an atomic host snapshot.

## Sources and metric contract

| Source | Metric names | Unit and interpretation |
| --- | --- | --- |
| `/proc/stat` | `cpu.{user,nice,system,idle,iowait,irq,softirq,steal,guest,guest_nice}.ticks` | Exact unsigned tick counters, `count`; kernel tick frequency is not assumed |
| `/proc/stat` | `cpu.busy.percent` | `percent`, derived from two successful aggregate CPU samples |
| `/proc/meminfo` | `memory.{total,available,used}.bytes` | `bytes`; used is total minus `MemAvailable` |
| `/proc/meminfo` | `swap.{total,free,used}.bytes` | `bytes`; used is total minus free |
| `/proc/loadavg` | `load.{1m,5m,15m}` | Nonnegative, fractional `count`; system load averages |
| `/proc/uptime` | `uptime.seconds` | Nonnegative fractional `seconds` |
| `/proc/net/dev` | `network.state` | `count` of interfaces, or one typed source failure |
| `/proc/net/dev` | `network.<interface>.{rx,tx}.bytes` | Exact cumulative `bytes`, including loopback when exposed |
| `/proc/net/dev` | `network.<interface>.{rx,tx}.bytes_per_second` | Derived `bytes_per_second` over the collector's measured monotonic interval |
| `/proc/net/dev` | `network.<interface>.{rx,tx}.errors` | Exact cumulative `count` from the directional `errs` column |
| `/proc/net/dev` | `network.<interface>.{rx,tx}.drops` | Exact cumulative `count` from the directional `drop` column |
| `/proc/diskstats` | `disk.state` | `count` of devices, or one typed source failure |
| `/proc/diskstats` | `disk.<device>.{read,write}.bytes` | Exact cumulative `bytes`, using Linux's fixed 512-byte diskstats sectors |
| `/proc/diskstats` | `disk.<device>.{read,write}.bytes_per_second` | Derived `bytes_per_second` over the measured monotonic interval |
| `/proc/pressure/{cpu,memory,io}` | `pressure.{cpu,memory,io}.some.avg10.percent` | Optional PSI `percent`; missing pressure files are `unsupported` |

CPU accounting uses the first eight kernel counters. Guest and guest-nice ticks
are already included in user and nice, and are not added a second time. Idle and
iowait count as non-busy. Linux iowait can decrease: any decreasing included CPU
counter invalidates the current derived interval and establishes a new baseline.
CPU percentages divide before multiplying to avoid rounding above 100 percent.

Memory values use checked `kB * 1024` conversion. `MemFree` is never substituted
for missing `MemAvailable`. An inconsistent available/free value above total
makes the derived used value an error. A genuine zero, such as an exposed
zero-size swap area, remains an available zero.

Disk observations include partitions and their parent devices when exposed.
Do not sum both into an aggregate because they can represent the same I/O.
Rates detect counter decreases, but cannot prove device identity across a
hotplug that reuses a name and happens to retain increasing counters.

Device labels retain ASCII letters, digits, `_`, `-`, and `.`. Other valid name
bytes use uppercase UTF-8 percent encoding, including `%` as `%25`, so names
remain unique and safe to render. For example, `vpn+prod` becomes `vpn%2Bprod`
and `cciss!c0d0` becomes `cciss%21c0d0`. Controls, whitespace, slash, colon, and
labels exceeding 100 encoded bytes produce a source error. Observations are
ordered by source, then by encoded device name.

Network errors and drops are raw cumulative observations available from the
first successful sample, including genuine zeros. A decrease publishes the new
counter, without inventing a rate or retaining the previous value. They do not
measure end-to-end packet loss or identify a cause. RX `drop` in procfs combines
`rx_dropped` and `rx_missed_errors`; error and drop categories can overlap and
must not be summed into a loss total. See the
[Linux interface statistics reference](https://docs.kernel.org/networking/statistics.html).
Coverage is limited to interfaces visible through the selected procfs network
namespace. No namespace identity or host-wide visibility is inferred.

The metrics appear automatically in snapshot text/JSON, doctor source counts,
and the generic terminal metric list. This is an additive schema 0.1 metric
extension using existing `count`, source and availability contracts. A failed
network source emits only the existing `network.state` failure; it does not
reuse cached interfaces or fabricate zero counters.

## Failure and resource limits

- Missing sources or optional counters are `unsupported`.
- Access denial is `permission_denied` with a fixed message that omits paths.
- Malformed data, integer overflow, invalid quantities, unsafe names, excessive
  source size/entity count, and counter decreases are `error`.
- Initial rates, newly observed devices, unchanged CPU accounting, and
  nonadvancing monotonic intervals are `temporarily_unavailable`.
- Source failures clear the corresponding rate baseline. Recovery needs a new
  second successful observation. Counter decreases reset the affected baseline.

Each of nine source files is limited to 1 MiB, with one extra detection byte.
Each network or disk source accepts at most 64 distinct devices; exceeding the
limit fails that source visibly through its `.state` metric. No entities are
silently truncated. A maximum accepted batch has 794 observations, below the
core snapshot limit of 2048. Read-time terminal control characters and invalid
UTF-8 fail the source. Numeric counters retain exact `u64` precision; rates and
fractional values use finite nonnegative floating-point numbers.

Only regular files are accepted. Metadata checks and Linux
`O_NONBLOCK | O_NOFOLLOW` flags prevent blocking on FIFOs and following a
substituted final symlink. Reads are synchronous; the byte limits do not provide
a wall-clock deadline for an unresponsive filesystem.

## Fixtures and validation

The exact `/proc` root selects live mode. Other explicit roots select fixture
mode. `with_fixture_mode()` can force the fixture label, including for an
explicit `/proc` argument. The standard `/proc/net` kernel symlink is accepted
only when reading the exact `/proc` root. Fixture files and their directories
must be real files/directories rather than symlinks.

Fixture directories must be trusted and static during collection. Path checks
do not prevent concurrent replacement of intermediate directories and do not
claim adversarial filesystem confinement. The collector reads only its fixed
source filenames and never includes paths, raw input, or OS error text in
observations.

Run `cargo test -p lidb-collect`, `cargo clippy -p lidb-collect --all-targets --
-D warnings`, and `cargo fmt --all -- --check`. Tests cover deterministic
fixtures, overflow/negative/nonfinite values, source isolation, reset/gap
behavior, bounded source/device inputs, permission error mapping, duplicate PSI
fields, name encoding, symlinks, and FIFOs. The FIFO test uses the Linux `mkfifo`
utility only to build a disposable test fixture. The live Linux smoke test reads
actual `/proc`; deterministic fixtures are not hardware validation. The
denied-file fixture is exercised on unprivileged runners and reports a bypass
if the runner can read a mode-000 file. Aarch64 runtime validation and non-Linux
compile checks remain separate environment checks.
