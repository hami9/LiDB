//! Pure, platform-agnostic LiDB domain contracts.
//!
//! This crate has no privileged operations, kernel hooks, vendor driver
//! dependencies, filesystem access or network side effects.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Capability discovery and availability reporting.
pub mod capability;
/// Bounded, single-series telemetry history and freshness checks.
pub mod history;
/// Structured network topology, routing, and socket state contracts.
pub mod network;
/// Versioned, bounded host snapshots for presentation and export.
pub mod snapshot;
/// Provenance-aware, typed telemetry observation contracts.
pub mod telemetry;

pub use capability::{Capability, CapabilityError, CapabilityRegistry, CapabilityState};
pub use history::{
    assess_freshness, Freshness, HistoryError, TelemetryHistory, MAX_HISTORY_SAMPLES,
};
pub use network::{
    AddressFamily, InterfaceOperState, InterfaceRecord, NetworkError, NetworkNamespace,
    NetworkReport, RouteRecord, SocketProtocol, SocketRecord, SocketState, SourceAvailability,
    MAX_INTERFACE_NAME_CHARS, MAX_MAC_ADDRESS_CHARS, MAX_NAMESPACE_NAME_CHARS,
    MAX_NETWORK_INTERFACES, MAX_NETWORK_ROUTES, MAX_NETWORK_SOCKETS, NETWORK_REPORT_SCHEMA_VERSION,
};
pub use snapshot::{
    MetricValue, Snapshot, SnapshotError, SnapshotMode, MAX_SNAPSHOT_METRICS,
    SNAPSHOT_SCHEMA_VERSION,
};
pub use telemetry::{MetricObservation, MetricState, TelemetryError, Unit};
