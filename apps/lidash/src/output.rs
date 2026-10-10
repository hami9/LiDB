use lidb_core::{MetricObservation, MetricValue, Snapshot, SnapshotMode, SNAPSHOT_SCHEMA_VERSION};
use serde::Serialize;
use std::{collections::BTreeMap, fmt::Write};

pub fn mode_name(mode: SnapshotMode) -> &'static str {
    match mode {
        SnapshotMode::Live => "live",
        SnapshotMode::Fixture => "fixture",
    }
}

pub fn value_text(metric: &MetricObservation<MetricValue>) -> String {
    match metric.state().available_value() {
        Some(MetricValue::Integer(value)) => value.to_string(),
        Some(MetricValue::Number(value)) => format!("{value:.2}"),
        None => format!(
            "{}: {}",
            metric.state().name(),
            metric.state().reason().unwrap_or("no reason reported")
        ),
    }
}

pub fn snapshot_text(snapshot: &Snapshot) -> String {
    let mut out = format!(
        "LiDB {} snapshot | collector_monotonic={}ns | collection={}ns | dropped_metrics={}\n",
        mode_name(snapshot.mode()),
        snapshot.monotonic_ns(),
        snapshot.collection_duration_ns(),
        snapshot.dropped_metrics()
    );
    for metric in snapshot.metrics() {
        let _ = writeln!(
            out,
            "{} = {} {} [{} @ {}ns]",
            metric.name(),
            value_text(metric),
            metric.unit().as_str(),
            metric.source(),
            metric.monotonic_ns()
        );
    }
    out
}

#[derive(Serialize)]
pub struct Failure<'a> {
    metric: &'a str,
    status: &'a str,
    reason: &'a str,
}

#[derive(Serialize)]
pub struct SourceCapability<'a> {
    source: &'a str,
    status: &'a str,
    available_metrics: usize,
    unavailable_metrics: usize,
    failures: Vec<Failure<'a>>,
}

#[derive(Serialize)]
pub struct DoctorReport<'a> {
    schema_version: &'static str,
    mode: &'static str,
    collection_status: &'static str,
    usable: bool,
    monotonic_ns: u64,
    collection_duration_ns: u64,
    dropped_metrics: u64,
    capabilities: Vec<SourceCapability<'a>>,
}

impl<'a> DoctorReport<'a> {
    pub fn from_snapshot(snapshot: &'a Snapshot) -> Self {
        let mut sources = BTreeMap::<&str, SourceCapability<'_>>::new();
        let mut available = 0;
        let mut missing = 0;
        for metric in snapshot.metrics() {
            let source = sources.entry(metric.source()).or_insert(SourceCapability {
                source: metric.source(),
                status: "unavailable",
                available_metrics: 0,
                unavailable_metrics: 0,
                failures: Vec::new(),
            });
            if metric.state().available_value().is_some() {
                source.available_metrics += 1;
                available += 1;
            } else {
                source.unavailable_metrics += 1;
                missing += 1;
                source.failures.push(Failure {
                    metric: metric.name(),
                    status: metric.state().name(),
                    reason: metric.state().reason().unwrap_or("no reason reported"),
                });
            }
        }
        for source in sources.values_mut() {
            source.status = if source.available_metrics > 0 {
                if source.unavailable_metrics > 0 {
                    "degraded"
                } else {
                    "available"
                }
            } else if source
                .failures
                .iter()
                .all(|failure| failure.status == source.failures[0].status)
            {
                source.failures[0].status
            } else {
                "unavailable"
            };
        }
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            mode: mode_name(snapshot.mode()),
            collection_status: if available == 0 {
                "unavailable"
            } else if missing > 0 || snapshot.dropped_metrics() > 0 {
                "degraded"
            } else {
                "available"
            },
            usable: available > 0,
            monotonic_ns: snapshot.monotonic_ns(),
            collection_duration_ns: snapshot.collection_duration_ns(),
            dropped_metrics: snapshot.dropped_metrics(),
            capabilities: sources.into_values().collect(),
        }
    }

    pub fn exit_code(&self) -> u8 {
        u8::from(!self.usable)
    }

    pub fn text(&self) -> String {
        let mut out = format!(
            "LiDB {} doctor: {} (source collection only; read-only; no active probes)\ncollection={}ns | dropped_metrics={}\n",
            self.mode, self.collection_status, self.collection_duration_ns, self.dropped_metrics
        );
        for source in &self.capabilities {
            let _ = writeln!(
                out,
                "{}: {} ({} available, {} unavailable)",
                source.source, source.status, source.available_metrics, source.unavailable_metrics
            );
            for failure in &source.failures {
                let _ = writeln!(
                    out,
                    "  {}: {} — {}",
                    failure.metric, failure.status, failure.reason
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::{MetricState, Unit};

    #[test]
    fn doctor_reports_partial_sources_and_real_zero_as_usable() {
        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "zero",
                    "procfs/stat",
                    Unit::Count,
                    0,
                    MetricState::Available(MetricValue::Integer(0)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "rate",
                    "procfs/stat",
                    Unit::Percent,
                    0,
                    MetricState::TemporarilyUnavailable("need two samples".into()),
                )
                .unwrap(),
            )
            .unwrap();
        let report = DoctorReport::from_snapshot(&snapshot);
        assert_eq!(report.exit_code(), 0);
        assert_eq!(report.collection_status, "degraded");
        assert_eq!(report.capabilities[0].status, "degraded");
        assert!(report.text().contains("need two samples"));
        assert!(snapshot_text(&snapshot).contains("zero = 0 count"));
        let empty = Snapshot::new(0, 0, SnapshotMode::Live);
        assert_eq!(DoctorReport::from_snapshot(&empty).exit_code(), 1);
    }
}
