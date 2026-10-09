//! Capability discovery without pretending unimplemented modules exist.
use std::collections::BTreeMap;
use std::fmt;

/// States used by Linux diagnostic capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityState {
    /// An adapter is available and has passed its own checks.
    Available,
    /// The platform does not expose this capability.
    Unsupported,
    /// Intentionally disabled or not implemented in the current release.
    Disabled,
    /// The current process lacks authorization.
    PermissionDenied,
    /// An adapter may become available after a transient failure.
    TemporarilyUnavailable,
    /// Last observation is too old to be reliable.
    Stale,
    /// An unexpected adapter or collection error occurred.
    Error,
}

impl CapabilityState {
    /// Stable, lowercase machine-readable state name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unsupported => "unsupported",
            Self::Disabled => "disabled",
            Self::PermissionDenied => "permission_denied",
            Self::TemporarilyUnavailable => "temporarily_unavailable",
            Self::Stale => "stale",
            Self::Error => "error",
        }
    }
}

/// A self-describing capability registration, not a declaration of measured health.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    id: String,
    state: CapabilityState,
    reason: Option<String>,
}

impl Capability {
    /// Register a capability ID and a truthful availability state.
    ///
    /// Unavailable capabilities require a nonempty human-readable reason.
    pub fn new(
        id: impl Into<String>,
        state: CapabilityState,
        reason: Option<String>,
    ) -> Result<Self, CapabilityError> {
        let id = id.into();
        if !valid_id(&id) {
            return Err(CapabilityError::InvalidId(id));
        }
        match state {
            CapabilityState::Available if reason.is_some() => {
                return Err(CapabilityError::UnexpectedReason);
            }
            CapabilityState::Available => {}
            _ if reason.as_ref().is_none_or(|value| value.trim().is_empty()) => {
                return Err(CapabilityError::MissingReason);
            }
            _ => {}
        }
        if reason
            .as_ref()
            .is_some_and(|value| value.len() > 512 || value.chars().any(char::is_control))
        {
            return Err(CapabilityError::InvalidReason);
        }
        Ok(Self { id, state, reason })
    }

    /// Globally meaningful capability identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Availability state.
    #[must_use]
    pub const fn state(&self) -> CapabilityState {
        self.state
    }

    /// Reason a capability is unavailable, if applicable.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

/// Errors returned when defining or registering capabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityError {
    /// Capability ID is not a safe lowercase dotted identifier.
    InvalidId(String),
    /// An unavailable capability omitted an explanatory reason.
    MissingReason,
    /// An available capability provided an impossible unavailability reason.
    UnexpectedReason,
    /// Failure reason contains terminal controls or exceeds 512 bytes.
    InvalidReason,
    /// A duplicate ID was registered.
    DuplicateId(String),
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(value) => write!(f, "invalid capability ID: {value}"),
            Self::MissingReason => f.write_str("unavailable capability requires a reason"),
            Self::UnexpectedReason => {
                f.write_str("available capability must not have an error reason")
            }
            Self::InvalidReason => f.write_str("invalid capability reason"),
            Self::DuplicateId(value) => write!(f, "duplicate capability: {value}"),
        }
    }
}

impl std::error::Error for CapabilityError {}

fn valid_id(value: &str) -> bool {
    value.len() <= 128
        && value.starts_with("org.lidb.")
        && !value.contains("..")
        && !value.ends_with('.')
        && value.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
        })
}

/// Deterministic, duplicate-safe capability registry.
#[derive(Clone, Debug, Default)]
pub struct CapabilityRegistry {
    entries: BTreeMap<String, Capability>,
}

impl CapabilityRegistry {
    /// Create an empty registry with no implied platform support.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one capability, rejecting duplicate identifiers.
    pub fn register(&mut self, item: Capability) -> Result<(), CapabilityError> {
        if self.entries.contains_key(item.id()) {
            return Err(CapabilityError::DuplicateId(item.id().to_owned()));
        }
        self.entries.insert(item.id().to_owned(), item);
        Ok(())
    }

    /// Look up a known capability without creating an implicit success value.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.entries.get(id)
    }

    /// Iterate in stable identifier order.
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.entries.values()
    }

    /// Count explicitly registered capabilities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Report whether no capability is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Known P0 product areas; they remain **disabled**, not fabricated as available.
#[must_use]
pub fn bootstrap_registry() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    for id in ["org.lidb.linux.host", "org.lidb.linux.network"] {
        registry
            .register(
                Capability::new(
                    id,
                    CapabilityState::Disabled,
                    Some("Collector not implemented in P0".to_owned()),
                )
                .expect("constant P0 capability is valid"),
            )
            .expect("constant P0 IDs are unique");
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ids_and_missing_reasons_are_rejected() {
        for id in [
            "",
            "gpu",
            "org.lidb..gpu",
            "org.lidb.AI",
            "org.lidb.gpu/../../x",
        ] {
            assert!(matches!(
                Capability::new(id, CapabilityState::Available, None),
                Err(CapabilityError::InvalidId(_))
            ));
        }
        assert_eq!(
            Capability::new("org.lidb.ai.gpu", CapabilityState::Disabled, None),
            Err(CapabilityError::MissingReason)
        );
        assert_eq!(
            Capability::new(
                "org.lidb.ai.gpu",
                CapabilityState::Available,
                Some("bad".into())
            ),
            Err(CapabilityError::UnexpectedReason)
        );
    }

    #[test]
    fn registry_rejects_duplicates_and_is_sorted() {
        let mut registry = CapabilityRegistry::new();
        let new = |id| Capability::new(id, CapabilityState::Available, None).unwrap();
        registry.register(new("org.lidb.z")).unwrap();
        registry.register(new("org.lidb.a")).unwrap();
        assert_eq!(
            registry.iter().map(Capability::id).collect::<Vec<_>>(),
            vec!["org.lidb.a", "org.lidb.z"]
        );
        assert_eq!(
            registry.register(new("org.lidb.a")),
            Err(CapabilityError::DuplicateId("org.lidb.a".into()))
        );
    }

    #[test]
    fn bootstrap_does_not_claim_active_collectors() {
        let registry = bootstrap_registry();
        assert_eq!(registry.len(), 2);
        assert!(registry
            .iter()
            .all(|item| item.state() == CapabilityState::Disabled));
    }

    #[test]
    fn bounds_explanations_and_rejects_terminal_controls() {
        for reason in ["x".repeat(513), "denied\u{1b}[2J".into()] {
            assert_eq!(
                Capability::new("org.lidb.linux.cpu", CapabilityState::Error, Some(reason)),
                Err(CapabilityError::InvalidReason)
            );
        }
    }
}
