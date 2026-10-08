//! Typed metrics and explicit missing/unsupported data.
//!
//! Observations use a monotonic clock domain provided by the collector. A
//! timestamp from one node is NOT comparable to another node unless explicit
//! clock synchronization uncertainty is represented by a higher layer.
use std::fmt;

/// Units of telemetry observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// Bytes in storage or memory.
    Bytes,
    /// Bytes transferred per second.
    BytesPerSecond,
    /// A unitless nonnegative count, if supported by the collector.
    Count,
    /// 0–100 percentage with interpretation documented by the source.
    Percent,
    /// Duration in nanoseconds.
    Nanoseconds,
    /// Measured electrical power in watts.
    Watts,
    /// Thermal temperature in Celsius.
    Celsius,
    /// Model inference tokens per second, with workload details.
    TokensPerSecond,
}

impl Unit {
    /// Stable machine-readable unit name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::BytesPerSecond => "bytes_per_second",
            Self::Count => "count",
            Self::Percent => "percent",
            Self::Nanoseconds => "nanoseconds",
            Self::Watts => "watts",
            Self::Celsius => "celsius",
            Self::TokensPerSecond => "tokens_per_second",
        }
    }
}

/// A sensor value or one explicit reason why no value should be trusted.
#[derive(Clone, Debug, PartialEq)]
pub enum MetricState<T> {
    /// Value was actually measured by a named source.
    Available(T),
    /// Hardware, OS, or API does not support the metric.
    Unsupported(String),
    /// Metric intentionally not collected, e.g. behind an off feature flag.
    Disabled(String),
    /// The collector lacks the necessary permission.
    PermissionDenied(String),
    /// A temporary probe failure prevented collection.
    TemporarilyUnavailable(String),
    /// A previous observation expired; not safe to reuse as live.
    Stale(String),
    /// An error occurred while collecting or validating.
    Error(String),
}

impl<T> MetricState<T> {
    /// Machine-readable availability.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Available(_) => "available",
            Self::Unsupported(_) => "unsupported",
            Self::Disabled(_) => "disabled",
            Self::PermissionDenied(_) => "permission_denied",
            Self::TemporarilyUnavailable(_) => "temporarily_unavailable",
            Self::Stale(_) => "stale",
            Self::Error(_) => "error",
        }
    }

    /// Why this metric is unavailable.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available(_) => None,
            Self::Unsupported(x)
            | Self::Disabled(x)
            | Self::PermissionDenied(x)
            | Self::TemporarilyUnavailable(x)
            | Self::Stale(x)
            | Self::Error(x) => Some(x),
        }
    }

    /// Value only if currently available; never substitute zero for absence.
    #[must_use]
    pub fn available_value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            _ => None,
        }
    }
}

/// One observation, including origin, unit, and source clock timestamp.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricObservation<T> {
    name: String,
    source: String,
    unit: Unit,
    monotonic_ns: u64,
    state: MetricState<T>,
}

impl<T> MetricObservation<T> {
    /// Construct only with known source and a non-empty metric name.
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        unit: Unit,
        monotonic_ns: u64,
        state: MetricState<T>,
    ) -> Result<Self, TelemetryError> {
        let name = name.into();
        let source = source.into();
        if name.trim().is_empty() || name.len() > 128 {
            return Err(TelemetryError::InvalidName);
        }
        if source.trim().is_empty() || source.len() > 128 {
            return Err(TelemetryError::InvalidSource);
        }
        if state.reason().is_some_and(|value| value.trim().is_empty()) {
            return Err(TelemetryError::EmptyFailureReason);
        }
        Ok(Self {
            name,
            source,
            unit,
            monotonic_ns,
            state,
        })
    }

    /// Stable metric name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Data source that made the measurement.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Measurement unit.
    #[must_use]
    pub const fn unit(&self) -> Unit {
        self.unit
    }

    /// Collector-local monotonic time (not automatically comparable between hosts).
    #[must_use]
    pub const fn monotonic_ns(&self) -> u64 {
        self.monotonic_ns
    }

    /// Explicit data availability state.
    #[must_use]
    pub fn state(&self) -> &MetricState<T> {
        &self.state
    }
}

/// Validation errors for telemetry contracts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryError {
    /// Metric name is blank or exceeds the permitted size.
    InvalidName,
    /// Source name is blank or exceeds the permitted size.
    InvalidSource,
    /// Unavailable value carries an empty explanation.
    EmptyFailureReason,
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => f.write_str("invalid metric name"),
            Self::InvalidSource => f.write_str("invalid metric source"),
            Self::EmptyFailureReason => f.write_str("unavailable metric requires a reason"),
        }
    }
}

impl std::error::Error for TelemetryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_is_never_zero() {
        let unavailable: MetricState<u64> =
            MetricState::PermissionDenied("no sensor access".into());
        assert_eq!(unavailable.available_value(), None);
        assert_eq!(unavailable.name(), "permission_denied");
        assert_eq!(unavailable.reason(), Some("no sensor access"));
        let real_zero = MetricState::Available(0_u64);
        assert_eq!(real_zero.available_value(), Some(&0));
    }

    #[test]
    fn invalid_sample_and_missing_reason_are_rejected() {
        assert_eq!(
            MetricObservation::<f64>::new(
                "",
                "linux.psi",
                Unit::Percent,
                123,
                MetricState::Available(2.0)
            ),
            Err(TelemetryError::InvalidName)
        );
        assert_eq!(
            MetricObservation::<f64>::new(
                "memory.psi",
                "",
                Unit::Percent,
                123,
                MetricState::Available(2.0)
            ),
            Err(TelemetryError::InvalidSource)
        );
        assert_eq!(
            MetricObservation::<f64>::new(
                "memory.psi",
                "linux.psi",
                Unit::Percent,
                123,
                MetricState::Disabled("".into())
            ),
            Err(TelemetryError::EmptyFailureReason)
        );
    }

    #[test]
    fn sample_retains_provenance_and_clock() {
        let obs = MetricObservation::new(
            "cpu.percent",
            "fixture",
            Unit::Percent,
            999,
            MetricState::Available(50.0),
        )
        .unwrap();
        assert_eq!(obs.name(), "cpu.percent");
        assert_eq!(obs.source(), "fixture");
        assert_eq!(obs.monotonic_ns(), 999);
        assert_eq!(obs.unit().as_str(), "percent");
    }
}
