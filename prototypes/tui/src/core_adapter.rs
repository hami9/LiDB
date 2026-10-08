//! Presentation of core capability contracts without inferring telemetry.
use lidb_core::{Capability, CapabilityRegistry, CapabilityState};

pub const GPU_CAPABILITY_ID: &str = "org.lidb.ai.gpu";

/// Owns a core registration snapshot; no host probe or collector is executed.
#[derive(Debug, Clone)]
pub struct CapabilityView {
    capability: Option<Capability>,
}

impl CapabilityView {
    pub fn gpu(registry: &CapabilityRegistry) -> Self {
        Self {
            capability: registry.get(GPU_CAPABILITY_ID).cloned(),
        }
    }

    /// Missing registration stays unknown rather than becoming unsupported.
    pub fn state(&self) -> Option<CapabilityState> {
        self.capability.as_ref().map(Capability::state)
    }

    pub fn badge_label(&self) -> &'static str {
        match self.state() {
            Some(CapabilityState::Available) => "[AVAILABLE]",
            Some(CapabilityState::Unsupported) => "[UNSUPPORTED]",
            Some(CapabilityState::Disabled) => "[DISABLED]",
            Some(CapabilityState::PermissionDenied) => "[PERMISSION DENIED]",
            Some(CapabilityState::TemporarilyUnavailable) => "[TEMPORARILY UNAVAILABLE]",
            Some(CapabilityState::Stale) => "[STALE]",
            Some(CapabilityState::Error) => "[ERROR]",
            None => "[NOT PROBED]",
        }
    }

    pub fn reason(&self) -> &str {
        match &self.capability {
            Some(capability) => capability
                .reason()
                .unwrap_or("Capability available; telemetry must be collected separately."),
            None => "Capability not registered; host support is unknown.",
        }
    }
}
