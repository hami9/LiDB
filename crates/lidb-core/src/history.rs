//! Bounded, source-scoped in-memory history for telemetry observations.
//!
//! A history contains observations of exactly one metric and source in one
//! caller-guaranteed clock domain. It never collects data or invents values.
//! Counts constrain the *number* of samples; producers must also bound payload
//! sizes and redact sensitive information at their own trust boundary.
use crate::telemetry::MetricObservation;
use std::collections::VecDeque;
use std::fmt;

/// Maximum sample count for one history instance.
pub const MAX_HISTORY_SAMPLES: usize = 4_096;

/// Errors reported without altering buffered observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryError {
    /// The requested capacity was zero.
    ZeroCapacity,
    /// The requested capacity exceeds the per-history safety limit.
    CapacityTooLarge,
    /// Incoming sample name differs from the first accepted sample.
    DifferentMetric,
    /// Incoming sample source differs from the first accepted sample.
    DifferentSource,
    /// Incoming timestamp precedes the most recently accepted sample.
    NonMonotonicTimestamp,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let detail = match self {
            Self::ZeroCapacity => "history capacity must be positive",
            Self::CapacityTooLarge => "history capacity exceeds the supported limit",
            Self::DifferentMetric => "observation belongs to a different metric",
            Self::DifferentSource => "observation belongs to a different source",
            Self::NonMonotonicTimestamp => "observation timestamp moved backwards",
        };
        f.write_str(detail)
    }
}

impl std::error::Error for HistoryError {}

/// Assessment of an observation against a caller-supplied monotonic clock.
///
/// The two timestamps MUST be from the same clock domain on the same node.
/// Stale data is never implicitly converted to a valid zero or fresh sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    /// Age is at or below the maximum acceptable age.
    Current {
        /// Elapsed nanoseconds since collection.
        age_ns: u64,
    },
    /// Age exceeds the allowed threshold.
    Stale {
        /// Elapsed nanoseconds since collection.
        age_ns: u64,
    },
    /// The supplied clock is earlier than the observation.
    ClockMovedBackwards {
        /// Nanoseconds by which the observation is ahead of the clock.
        ahead_ns: u64,
    },
}

/// Assess one timestamp using a monotonic clock from the same domain.
///
/// Exactly the threshold age is considered current; a clock moving backward
/// is NOT mistaken for a new measurement.
#[must_use]
pub fn assess_freshness(sample_monotonic_ns: u64, now_monotonic_ns: u64, max_age_ns: u64) -> Freshness {
    match now_monotonic_ns.checked_sub(sample_monotonic_ns) {
        None => Freshness::ClockMovedBackwards {
            ahead_ns: sample_monotonic_ns - now_monotonic_ns,
        },
        Some(age_ns) if age_ns > max_age_ns => Freshness::Stale { age_ns },
        Some(age_ns) => Freshness::Current { age_ns },
    }
}

/// Bounded chronological samples from one metric, source and clock domain.
///
/// The first accepted sample establishes metric name and source; later samples
/// with different identifiers or earlier timestamps are rejected atomically.
/// Equal monotonic timestamps are valid for different observations in a batch.
///
/// **Caller responsibility:** a source identifier cannot prove that two samples
/// belong to the same node or clock domain. Keep per-node and per-boot histories
/// separate, and never compare unrelated monotonic timestamps.
#[derive(Debug)]
pub struct TelemetryHistory<T> {
    capacity: usize,
    samples: VecDeque<MetricObservation<T>>,
    series: Option<(String, String)>,
    evicted: u64,
}

impl<T> TelemetryHistory<T> {
    /// Create a bounded history, rejecting unbounded or zero sample capacities.
    pub fn new(capacity: usize) -> Result<Self, HistoryError> {
        if capacity == 0 {
            return Err(HistoryError::ZeroCapacity);
        }
        if capacity > MAX_HISTORY_SAMPLES {
            return Err(HistoryError::CapacityTooLarge);
        }
        Ok(Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
            series: None,
            evicted: 0,
        })
    }

    /// Store an observation and return whether one old sample was evicted.
    ///
    /// Failure leaves the history and eviction counter unchanged. An
    /// unavailable observation remains unavailable and is never rewritten.
    pub fn push(&mut self, observation: MetricObservation<T>) -> Result<bool, HistoryError> {
        if let Some((name, source)) = &self.series {
            if observation.name() != name {
                return Err(HistoryError::DifferentMetric);
            }
            if observation.source() != source {
                return Err(HistoryError::DifferentSource);
            }
            if let Some(last) = self.samples.back() {
                if observation.monotonic_ns() < last.monotonic_ns() {
                    return Err(HistoryError::NonMonotonicTimestamp);
                }
            }
        } else {
            self.series = Some((
                observation.name().to_owned(),
                observation.source().to_owned(),
            ));
        }
        let evicted = self.samples.len() == self.capacity;
        if evicted {
            self.samples.pop_front();
            self.evicted = self.evicted.saturating_add(1);
        }
        self.samples.push_back(observation);
        Ok(evicted)
    }

    /// Return the most recently accepted observation, if any.
    #[must_use]
    pub fn latest(&self) -> Option<&MetricObservation<T>> {
        self.samples.back()
    }

    /// Iterate oldest to newest without copying observations.
    pub fn iter(&self) -> impl Iterator<Item = &MetricObservation<T>> {
        self.samples.iter()
    }

    /// Assess the newest sample against a caller-controlled monotonic clock.
    ///
    /// Returns `None` if no samples have been accepted.
    #[must_use]
    pub fn latest_freshness(&self, now_monotonic_ns: u64, max_age_ns: u64) -> Option<Freshness> {
        self.latest().map(|sample| {
            assess_freshness(sample.monotonic_ns(), now_monotonic_ns, max_age_ns)
        })
    }

    /// Current number of retained observations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether no observations are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Maximum number of retained observations.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of successfully received observations evicted due to capacity.
    ///
    /// This is *not* a transport loss counter and does not include rejected
    /// observations. The counter saturates on `u64::MAX`.
    #[must_use]
    pub const fn evicted_count(&self) -> u64 {
        self.evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{MetricState, Unit};

    fn obs(name: &str, source: &str, ts: u64, state: MetricState<u64>) -> MetricObservation<u64> {
        MetricObservation::new(name, source, Unit::Bytes, ts, state).unwrap()
    }

    #[test]
    fn invalid_capacity_rejected() {
        assert!(matches!(TelemetryHistory::<u64>::new(0), Err(HistoryError::ZeroCapacity)));
        assert!(matches!(
            TelemetryHistory::<u64>::new(MAX_HISTORY_SAMPLES + 1),
            Err(HistoryError::CapacityTooLarge)
        ));
        assert_eq!(TelemetryHistory::<u64>::new(MAX_HISTORY_SAMPLES).unwrap().capacity(), MAX_HISTORY_SAMPLES);
    }

    #[test]
    fn evicts_oldest_without_losing_unavailability_state() {
        let mut history = TelemetryHistory::new(2).unwrap();
        assert_eq!(history.push(obs("memory.bytes", "linux.meminfo", 1, MetricState::Available(0))), Ok(false));
        assert_eq!(history.push(obs("memory.bytes", "linux.meminfo", 2, MetricState::PermissionDenied("denied".into()))), Ok(false));
        assert_eq!(history.push(obs("memory.bytes", "linux.meminfo", 3, MetricState::Available(33))), Ok(true));
        assert_eq!(history.len(), 2);
        assert_eq!(history.evicted_count(), 1);
        assert_eq!(history.iter().map(MetricObservation::monotonic_ns).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!(history.iter().next().unwrap().state().available_value(), None);
        assert_eq!(history.latest().unwrap().state().available_value(), Some(&33));
    }

    #[test]
    fn reject_cross_series_and_clock_regression_without_mutating() {
        let mut history = TelemetryHistory::new(1).unwrap();
        history.push(obs("cpu.load", "linux.proc", 100, MetricState::Available(7))).unwrap();
        assert_eq!(
            history.push(obs("memory.bytes", "linux.proc", 101, MetricState::Available(9))),
            Err(HistoryError::DifferentMetric)
        );
        assert_eq!(
            history.push(obs("cpu.load", "fixture", 101, MetricState::Available(9))),
            Err(HistoryError::DifferentSource)
        );
        assert_eq!(
            history.push(obs("cpu.load", "linux.proc", 99, MetricState::Available(9))),
            Err(HistoryError::NonMonotonicTimestamp)
        );
        assert_eq!(history.evicted_count(), 0);
        assert_eq!(history.latest().unwrap().state().available_value(), Some(&7));
    }

    #[test]
    fn equal_timestamps_are_accepted_for_same_series() {
        let mut history = TelemetryHistory::new(2).unwrap();
        history.push(obs("cpu.load", "fixture", 1, MetricState::Available(1))).unwrap();
        history.push(obs("cpu.load", "fixture", 1, MetricState::Available(2))).unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn empty_history_has_no_latest_or_freshness() {
        let history: TelemetryHistory<u64> = TelemetryHistory::new(1).unwrap();
        assert!(history.is_empty());
        assert!(history.latest().is_none());
        assert!(history.latest_freshness(100, 10).is_none());
    }

    #[test]
    fn threshold_and_time_regression_are_distinct() {
        assert_eq!(assess_freshness(90, 100, 10), Freshness::Current { age_ns: 10 });
        assert_eq!(assess_freshness(90, 101, 10), Freshness::Stale { age_ns: 11 });
        assert_eq!(assess_freshness(110, 100, 10), Freshness::ClockMovedBackwards { ahead_ns: 10 });
        assert_eq!(assess_freshness(0, u64::MAX, 0), Freshness::Stale { age_ns: u64::MAX });
    }

    #[test]
    fn freshness_does_not_convert_missing_data_to_available() {
        let mut history = TelemetryHistory::new(1).unwrap();
        history.push(obs("cpu.load", "fixture", 50, MetricState::Unsupported("no sensor".into()))).unwrap();
        assert_eq!(history.latest_freshness(51, 10), Some(Freshness::Current { age_ns: 1 }));
        assert!(history.latest().unwrap().state().available_value().is_none());
    }
}
