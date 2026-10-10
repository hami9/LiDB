//! Bounded, non-privileged host observations from Linux procfs.
//!
//! [`LinuxCollector`] reads aggregate host metadata only. Sources fail
//! independently; missing data never becomes an available zero. Counter rates
//! require two successful observations in the same collector lifetime.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod parse;
mod read;

use lidb_core::{MetricObservation, MetricState, MetricValue, Snapshot, SnapshotMode, Unit};
use parse::{CpuCounters, TransferCounters};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

type State = MetricState<MetricValue>;
type PreviousTransfers = Option<(u64, BTreeMap<String, TransferCounters>)>;

/// Maximum bytes accepted from any one source file.
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
/// Maximum interfaces or block devices accepted from one source.
pub const MAX_DEVICES: usize = 64;

/// Stateful, read-only aggregate Linux telemetry collector.
///
/// The exact `/proc` path selects live mode. Any other root selects fixture
/// mode, including paths that resolve to `/proc`. Host reads are supported on
/// Linux x86_64 and aarch64. Other targets report `unsupported` without reads.
pub struct LinuxCollector {
    proc_root: PathBuf,
    mode: SnapshotMode,
    started: Instant,
    previous_cpu: Option<(u64, CpuCounters)>,
    previous_network: PreviousTransfers,
    previous_disk: PreviousTransfers,
}

impl LinuxCollector {
    /// Select the standard `/proc` root or an explicit fixture directory.
    #[must_use]
    pub fn new(proc_root: impl Into<PathBuf>) -> Self {
        let proc_root = proc_root.into();
        let mode = if proc_root.as_os_str() == Path::new("/proc").as_os_str() {
            SnapshotMode::Live
        } else {
            SnapshotMode::Fixture
        };
        Self {
            proc_root,
            mode,
            started: Instant::now(),
            previous_cpu: None,
            previous_network: None,
            previous_disk: None,
        }
    }

    /// Label this collector's output as fixture data even for `/proc`.
    ///
    /// This supports an explicitly supplied CLI source without implying live
    /// production provenance. It does not change the selected source path.
    #[must_use]
    pub fn with_fixture_mode(mut self) -> Self {
        self.mode = SnapshotMode::Fixture;
        self
    }

    /// Collect one bounded batch with explicit per-source failure states.
    ///
    /// Observation timestamps equal the enclosing batch timestamp and use
    /// monotonic nanoseconds since this collector was constructed. CPU busy
    /// excludes idle and iowait, and avoids double counting guest ticks.
    /// Transfer rates use this clock and reset their baseline after failures.
    #[must_use]
    pub fn sample(&mut self) -> Snapshot {
        let began = Instant::now();
        let timestamp = duration_ns(self.started.elapsed());
        let mut snapshot = self.sample_at(timestamp);
        snapshot.set_collection_duration_ns(duration_ns(began.elapsed()));
        snapshot
    }

    fn sample_at(&mut self, timestamp: u64) -> Snapshot {
        let mut snapshot = Snapshot::new(timestamp, 0, self.mode);
        self.cpu(&mut snapshot);
        self.memory(&mut snapshot);
        self.scalars(&mut snapshot);
        self.transfers(&mut snapshot, false);
        self.transfers(&mut snapshot, true);
        for kind in ["cpu", "memory", "io"] {
            let source = format!("proc.pressure.{kind}");
            let result = self
                .read(&format!("pressure/{kind}"))
                .and_then(|text| parse::pressure(&text));
            add(
                &mut snapshot,
                &format!("pressure.{kind}.some.avg10.percent"),
                &source,
                Unit::Percent,
                number_result(result),
            );
        }
        snapshot
    }

    fn read(&self, source: &str) -> Result<String, State> {
        let source_mode = if self.proc_root.as_os_str() == Path::new("/proc").as_os_str() {
            SnapshotMode::Live
        } else {
            SnapshotMode::Fixture
        };
        read::source(&self.proc_root, source, source_mode)
    }

    fn cpu(&mut self, snapshot: &mut Snapshot) {
        let result = self.read("stat").and_then(|text| parse::cpu(&text));
        let fields = [
            "user",
            "nice",
            "system",
            "idle",
            "iowait",
            "irq",
            "softirq",
            "steal",
            "guest",
            "guest_nice",
        ];
        match result {
            Ok(current) => {
                for (field, value) in fields.iter().zip(current.0) {
                    add(
                        snapshot,
                        &format!("cpu.{field}.ticks"),
                        "proc.stat",
                        Unit::Count,
                        value.map_or_else(
                            || unsupported("counter is not exposed by this kernel"),
                            integer,
                        ),
                    );
                }
                let state = match self.previous_cpu.as_ref() {
                    Some((last_time, _)) if snapshot.monotonic_ns() <= *last_time => {
                        temporarily_unavailable("monotonic interval has not advanced")
                    }
                    previous => cpu_busy(previous.map(|(_, counters)| counters), &current),
                };
                add(
                    snapshot,
                    "cpu.busy.percent",
                    "proc.stat",
                    Unit::Percent,
                    state,
                );
                self.previous_cpu = Some((snapshot.monotonic_ns(), current));
            }
            Err(state) => {
                for field in fields {
                    add(
                        snapshot,
                        &format!("cpu.{field}.ticks"),
                        "proc.stat",
                        Unit::Count,
                        state.clone(),
                    );
                }
                add(
                    snapshot,
                    "cpu.busy.percent",
                    "proc.stat",
                    Unit::Percent,
                    state,
                );
                self.previous_cpu = None;
            }
        }
    }

    fn memory(&self, snapshot: &mut Snapshot) {
        let result = self.read("meminfo").map(|text| parse::memory(&text));
        let keys = [
            ("memory.total.bytes", "MemTotal"),
            ("memory.available.bytes", "MemAvailable"),
            ("swap.total.bytes", "SwapTotal"),
            ("swap.free.bytes", "SwapFree"),
        ];
        let values: Vec<State> = keys
            .iter()
            .map(|(_, key)| match &result {
                Ok(fields) => fields
                    .get(*key)
                    .cloned()
                    .unwrap_or_else(|| unsupported("field is not exposed by this kernel")),
                Err(state) => state.clone(),
            })
            .collect();
        for ((name, _), value) in keys.iter().zip(&values) {
            add(snapshot, name, "proc.meminfo", Unit::Bytes, value.clone());
        }
        for (name, total, free) in [
            ("memory.used.bytes", &values[0], &values[1]),
            ("swap.used.bytes", &values[2], &values[3]),
        ] {
            let value = match (total, free) {
                (
                    MetricState::Available(MetricValue::Integer(total)),
                    MetricState::Available(MetricValue::Integer(free)),
                ) => total
                    .checked_sub(*free)
                    .map_or_else(|| error("available bytes exceed total bytes"), integer),
                (MetricState::Available(_), missing) => missing.clone(),
                (missing, _) => missing.clone(),
            };
            add(snapshot, name, "proc.meminfo", Unit::Bytes, value);
        }
    }

    fn scalars(&self, snapshot: &mut Snapshot) {
        let loads = self.read("loadavg").and_then(|text| parse::load(&text));
        for (index, name) in ["load.1m", "load.5m", "load.15m"].iter().enumerate() {
            let state = match &loads {
                Ok(values) => number(values[index]),
                Err(state) => state.clone(),
            };
            add(snapshot, name, "proc.loadavg", Unit::Count, state);
        }
        let uptime = self.read("uptime").and_then(|text| parse::uptime(&text));
        add(
            snapshot,
            "uptime.seconds",
            "proc.uptime",
            Unit::Seconds,
            number_result(uptime),
        );
    }

    fn transfers(&mut self, snapshot: &mut Snapshot, disk: bool) {
        let (path, source, prefix, directions) = if disk {
            ("diskstats", "proc.diskstats", "disk", ["read", "write"])
        } else {
            ("net/dev", "proc.net.dev", "network", ["rx", "tx"])
        };
        let mut network_counters = BTreeMap::new();
        let result = self.read(path).and_then(|text| {
            if disk {
                parse::disks(&text)
            } else {
                parse::network(&text).map(|devices| {
                    let bytes = devices
                        .iter()
                        .map(|(name, counters)| (name.clone(), counters.bytes.clone()))
                        .collect();
                    network_counters = devices;
                    bytes
                })
            }
        });
        let previous = if disk {
            &mut self.previous_disk
        } else {
            &mut self.previous_network
        };
        match result {
            Ok(devices) => {
                add(
                    snapshot,
                    &format!("{prefix}.state"),
                    source,
                    Unit::Count,
                    integer(devices.len() as u64),
                );
                for (name, counters) in &devices {
                    let rates = transfer_rates(previous, snapshot.monotonic_ns(), name, counters);
                    for ((direction, counter), rate) in directions.iter().zip(counters.0).zip(rates)
                    {
                        add(
                            snapshot,
                            &format!("{prefix}.{name}.{direction}.bytes"),
                            source,
                            Unit::Bytes,
                            integer(counter),
                        );
                        add(
                            snapshot,
                            &format!("{prefix}.{name}.{direction}.bytes_per_second"),
                            source,
                            Unit::BytesPerSecond,
                            rate,
                        );
                    }
                    if let Some(counters) = network_counters.get(name) {
                        for (index, direction) in directions.iter().enumerate() {
                            for (field, value) in [
                                ("errors", counters.errors[index]),
                                ("drops", counters.drops[index]),
                            ] {
                                add(
                                    snapshot,
                                    &format!("network.{name}.{direction}.{field}"),
                                    source,
                                    Unit::Count,
                                    integer(value),
                                );
                            }
                        }
                    }
                }
                *previous = Some((snapshot.monotonic_ns(), devices));
            }
            Err(state) => {
                add(
                    snapshot,
                    &format!("{prefix}.state"),
                    source,
                    Unit::Count,
                    state,
                );
                *previous = None;
            }
        }
    }
}

fn cpu_busy(previous: Option<&CpuCounters>, current: &CpuCounters) -> State {
    let Some(current) = current.accounting() else {
        return unsupported("CPU accounting fields are not exposed");
    };
    let Some(previous) = previous.and_then(CpuCounters::accounting) else {
        return initial_rate();
    };
    let mut delta = [0; 8];
    for ((value, current), previous) in delta.iter_mut().zip(current).zip(previous) {
        let Some(difference) = current.checked_sub(previous) else {
            return error("CPU counters decreased; baseline reset");
        };
        *value = difference;
    }
    let Some(total) = delta
        .iter()
        .try_fold(0_u64, |sum, value| sum.checked_add(*value))
    else {
        return error("CPU counter delta overflow");
    };
    if total == 0 {
        return temporarily_unavailable("CPU accounting has not advanced");
    }
    // Guest and guest_nice are already included in user and nice. Idle and
    // iowait are excluded from busy; all other first-eight counters are busy.
    let busy = total - delta[3] - delta[4];
    number((busy as f64 / total as f64) * 100.0)
}

fn transfer_rates(
    previous: &PreviousTransfers,
    timestamp: u64,
    name: &str,
    current: &TransferCounters,
) -> [State; 2] {
    let Some((last_time, devices)) = previous else {
        return [initial_rate(), initial_rate()];
    };
    let Some(last) = devices.get(name) else {
        return [initial_rate(), initial_rate()];
    };
    let Some(elapsed) = timestamp.checked_sub(*last_time).filter(|value| *value > 0) else {
        let state = temporarily_unavailable("monotonic interval has not advanced");
        return [state.clone(), state];
    };
    std::array::from_fn(|index| match current.0[index].checked_sub(last.0[index]) {
        Some(delta) => number(delta as f64 / (elapsed as f64 / 1_000_000_000.0)),
        None => error("transfer counter decreased; baseline reset"),
    })
}

fn add(snapshot: &mut Snapshot, name: &str, source: &str, unit: Unit, state: State) {
    // Names, sources, reasons and finite values are constructed locally or
    // strictly parsed. The 64-device source limits keep the batch below 2048.
    let observation = MetricObservation::new(name, source, unit, snapshot.monotonic_ns(), state)
        .expect("collector constructs valid observation metadata");
    snapshot
        .push(observation)
        .expect("bounded collector constructs valid unique metrics");
}

fn duration_ns(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}
fn integer(value: u64) -> State {
    MetricState::Available(MetricValue::Integer(value))
}
fn number(value: f64) -> State {
    MetricState::Available(MetricValue::Number(value))
}
fn number_result(result: Result<f64, State>) -> State {
    result.map_or_else(|state| state, number)
}
fn unsupported(reason: &str) -> State {
    MetricState::Unsupported(reason.into())
}
fn temporarily_unavailable(reason: &str) -> State {
    MetricState::TemporarilyUnavailable(reason.into())
}
fn error(reason: &str) -> State {
    MetricState::Error(reason.into())
}
fn initial_rate() -> State {
    temporarily_unavailable("rate requires a second successful observation")
}

#[cfg(test)]
mod tests;
