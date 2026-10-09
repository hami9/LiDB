//! Pure, platform-agnostic LiDB domain contracts.
//!
//! This crate has no privileged operations, kernel hooks, GPU driver
//! dependencies, filesystem access or network side effects.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Capability discovery and availability reporting.
pub mod capability;
/// Bounded, single-series telemetry history and freshness checks.
pub mod history;
/// Versioned, bounded host snapshots for presentation and export.
pub mod snapshot;
/// Provenance-aware, typed telemetry observation contracts.
pub mod telemetry;

pub use capability::{Capability, CapabilityError, CapabilityRegistry, CapabilityState};
pub use history::{
    assess_freshness, Freshness, HistoryError, TelemetryHistory, MAX_HISTORY_SAMPLES,
};
pub use snapshot::{
    MetricValue, Snapshot, SnapshotError, SnapshotMode, MAX_SNAPSHOT_METRICS,
    SNAPSHOT_SCHEMA_VERSION,
};
pub use telemetry::{MetricObservation, MetricState, TelemetryError, Unit};
