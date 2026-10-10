//! Export contracts. No deserializer is exposed for unchecked external input.
use crate::{MetricObservation, Unit};
use serde::Serialize;
use std::fmt;

/// Initial public snapshot contract; not the independent IPC protocol version.
pub const SNAPSHOT_SCHEMA_VERSION: &str = "0.1";
/// Limit across all metrics in one snapshot.
pub const MAX_SNAPSHOT_METRICS: usize = 2_048;

/// Preserve integer counter precision while supporting derived fractional values.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MetricValue {
    /// Exact unsigned counter or byte quantity.
    Integer(u64),
    /// Finite derived or fractional reading, validated by Snapshot::push.
    Number(f64),
}

impl MetricValue {
    /// Presentation conversion; exact export retains the integer variant.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Integer(value) => value as f64,
            Self::Number(value) => value,
        }
    }
}

/// Live data and injected test data must remain visibly distinct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotMode {
    /// Reads from the standard local Linux procfs.
    Live,
    /// Reads from an explicitly selected fixture directory.
    Fixture,
}

/// Bounded, collector-local batch. Metric timestamps share one collector lifetime.
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    schema_version: &'static str,
    clock_source: &'static str,
    monotonic_ns: u64,
    collection_duration_ns: u64,
    mode: SnapshotMode,
    metrics: Vec<MetricObservation<MetricValue>>,
    dropped_metrics: u64,
}

impl Snapshot {
    /// Start a snapshot; no observations are implied by an empty batch.
    #[must_use]
    pub fn new(monotonic_ns: u64, collection_duration_ns: u64, mode: SnapshotMode) -> Self {
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            clock_source: "collector_monotonic",
            monotonic_ns,
            collection_duration_ns,
            mode,
            metrics: Vec::new(),
            dropped_metrics: 0,
        }
    }

    /// Append one distinct observation, checking quantity, time and resource limits.
    pub fn push(&mut self, metric: MetricObservation<MetricValue>) -> Result<(), SnapshotError> {
        if self.metrics.len() >= MAX_SNAPSHOT_METRICS {
            self.dropped_metrics = self.dropped_metrics.saturating_add(1);
            return Err(SnapshotError::Capacity);
        }
        if self
            .metrics
            .iter()
            .any(|existing| existing.name() == metric.name())
        {
            return Err(SnapshotError::DuplicateMetric);
        }
        if metric.monotonic_ns() > self.monotonic_ns {
            return Err(SnapshotError::FutureObservation);
        }
        if let Some(value) = metric.state().available_value() {
            let numeric = value.as_f64();
            if !numeric.is_finite()
                || (numeric < 0.0 && metric.unit() != Unit::Celsius)
                || (metric.unit() == Unit::Percent && numeric > 100.0)
            {
                return Err(SnapshotError::InvalidValue);
            }
        }
        self.metrics.push(metric);
        Ok(())
    }

    /// Replace the measured collection duration after the read completes.
    pub fn set_collection_duration_ns(&mut self, duration_ns: u64) {
        self.collection_duration_ns = duration_ns;
    }

    /// Observations in stable collector-defined order.
    #[must_use]
    pub fn metrics(&self) -> &[MetricObservation<MetricValue>] {
        &self.metrics
    }
    /// Find one named metric; absence never creates an available value.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&MetricObservation<MetricValue>> {
        self.metrics.iter().find(|metric| metric.name() == name)
    }
    /// Collector-local monotonic batch time.
    #[must_use]
    pub const fn monotonic_ns(&self) -> u64 {
        self.monotonic_ns
    }
    /// Time spent collecting this batch.
    #[must_use]
    pub const fn collection_duration_ns(&self) -> u64 {
        self.collection_duration_ns
    }
    /// Whether readings came from live procfs or an injected fixture.
    #[must_use]
    pub const fn mode(&self) -> SnapshotMode {
        self.mode
    }
    /// Number of metrics omitted because the batch exceeded its bound.
    #[must_use]
    pub const fn dropped_metrics(&self) -> u64 {
        self.dropped_metrics
    }
}

/// Invalid observation insertion; valid existing samples remain intact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotError {
    /// Observation count exceeded the hard limit.
    Capacity,
    /// A second observation used an existing metric name.
    DuplicateMetric,
    /// Non-finite, out-of-range percentage or invalid negative quantity.
    InvalidValue,
    /// Observation is newer than its enclosing batch.
    FutureObservation,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid snapshot: {self:?}")
    }
}
impl std::error::Error for SnapshotError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MetricState;
    fn observation(
        name: &str,
        time: u64,
        value: MetricValue,
        unit: Unit,
    ) -> MetricObservation<MetricValue> {
        MetricObservation::new(name, "fixture", unit, time, MetricState::Available(value)).unwrap()
    }
    #[test]
    fn validates_numbers_and_unique_names_without_mutating_valid_data() {
        let mut snapshot = Snapshot::new(5, 0, SnapshotMode::Fixture);
        for value in [f64::NAN, f64::INFINITY, -1.0, 101.0] {
            assert_eq!(
                snapshot.push(observation(
                    "cpu",
                    5,
                    MetricValue::Number(value),
                    Unit::Percent
                )),
                Err(SnapshotError::InvalidValue)
            );
        }
        assert_eq!(
            snapshot.push(observation("cpu", 6, MetricValue::Integer(1), Unit::Count)),
            Err(SnapshotError::FutureObservation)
        );
        snapshot
            .push(observation("cpu", 5, MetricValue::Integer(0), Unit::Count))
            .unwrap();
        assert_eq!(
            snapshot.push(observation("cpu", 5, MetricValue::Integer(2), Unit::Count)),
            Err(SnapshotError::DuplicateMetric)
        );
        assert_eq!(snapshot.metrics().len(), 1);
    }
    #[test]
    fn serializes_exact_counters_and_truthful_unavailability() {
        let mut snapshot = Snapshot::new(5, 0, SnapshotMode::Fixture);
        snapshot
            .push(observation(
                "counter",
                5,
                MetricValue::Integer(u64::MAX),
                Unit::Bytes,
            ))
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "missing",
                    "fixture",
                    Unit::Bytes,
                    5,
                    MetricState::Unsupported("not exposed".into()),
                )
                .unwrap(),
            )
            .unwrap();
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["schema_version"], "0.1");
        assert_eq!(json["mode"], "fixture");
        assert_eq!(
            json["metrics"][0]["state"]["value"].as_u64(),
            Some(u64::MAX)
        );
        assert_eq!(json["metrics"][1]["state"]["status"], "unsupported");
        assert_eq!(json["metrics"][1]["state"]["value"], "not exposed");
    }

    #[test]
    fn permits_signed_temperature_without_permitting_negative_counters() {
        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        snapshot
            .push(observation(
                "temperature",
                0,
                MetricValue::Number(-20.0),
                Unit::Celsius,
            ))
            .unwrap();
        assert_eq!(
            snapshot.push(observation(
                "bytes",
                0,
                MetricValue::Number(-20.0),
                Unit::Bytes
            )),
            Err(SnapshotError::InvalidValue)
        );
    }
    #[test]
    fn enforces_batch_limit_and_counts_omissions() {
        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Live);
        for index in 0..MAX_SNAPSHOT_METRICS {
            snapshot
                .push(observation(
                    &format!("m{index}"),
                    0,
                    MetricValue::Integer(0),
                    Unit::Count,
                ))
                .unwrap();
        }
        assert_eq!(
            snapshot.push(observation(
                "overflow",
                0,
                MetricValue::Integer(0),
                Unit::Count
            )),
            Err(SnapshotError::Capacity)
        );
        assert_eq!(snapshot.dropped_metrics(), 1);
    }
}
