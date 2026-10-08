pub mod cpu_mem;
pub mod diagnostics;
pub mod gpu_ai;
pub mod network;
pub mod processes;
pub mod settings;
pub mod system;

use serde::{Deserialize, Serialize};

/// Indicates the validity and source of a telemetry stream or field.
/// Adheres to LiDB Non-negotiable Contract:
/// - Never invent live metrics or claim fake data is real.
/// - Clearly distinguish live data, simulated fixtures, and typed unavailable states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataSourceStatus {
    /// Telemetry gathered from live kernel/hardware interfaces.
    Live,
    /// Explicit simulated fixture for development or headless testing.
    SimulatedFixture { fixture_name: String },
    /// Subsystem is unsupported on this platform (e.g. GPU in PRoot).
    Unsupported { reason: String },
    /// Subsystem requires root or elevated capability (CAP_NET_ADMIN, CAP_SYS_ADMIN).
    PermissionDenied { capability: String },
    /// Subsystem capability detection not yet executed on host.
    NotProbed { reason: String },
    /// Telemetry stream paused by operator.
    Paused,
}

impl DataSourceStatus {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Live => "[LIVE TELEMETRY]",
            Self::SimulatedFixture { .. } => "[SIMULATED FIXTURE]",
            Self::Unsupported { .. } => "[UNSUPPORTED ON HOST]",
            Self::PermissionDenied { .. } => "[PERMISSION REQUIRED]",
            Self::NotProbed { .. } => "[NOT PROBED]",
            Self::Paused => "[PAUSED]",
        }
    }

    pub fn is_simulated(&self) -> bool {
        matches!(self, Self::SimulatedFixture { .. })
    }
}
