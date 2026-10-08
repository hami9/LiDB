//! Pure, platform-agnostic LiDB domain contracts.
//!
//! This crate has no privileged operations, kernel hooks, GPU driver
//! dependencies, filesystem access or network side effects.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Capability discovery and availability reporting.
pub mod capability;
/// Provenance-aware, typed telemetry observation contracts.
pub mod telemetry;

pub use capability::{Capability, CapabilityError, CapabilityRegistry, CapabilityState};
pub use telemetry::{MetricObservation, MetricState, TelemetryError, Unit};
