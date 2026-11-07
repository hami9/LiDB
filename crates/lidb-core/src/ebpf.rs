//! Pure, platform-agnostic eBPF telemetry and report contracts.
//!
//! LiDB provides optional deeper instrumentation through Linux eBPF tracepoints
//! (e.g. short-lived TCP connection lifecycles and retransmissions) when explicitly
//! activated by the operator.
//!
//! This module defines pure, bounded event records and report contracts without
//! filesystem access, kernel dependencies, or privileged syscalls.

use crate::{
    network::{AddressFamily, SocketState, SourceAvailability},
    SnapshotMode,
};
use serde::Serialize;
use std::{error::Error, fmt, net::IpAddr};

/// Schema version for eBPF diagnostic reports; pre-stable contract.
pub const EBPF_REPORT_SCHEMA_VERSION: &str = "0.1";

/// Maximum eBPF lifecycle events retained in a single report buffer.
pub const MAX_EBPF_EVENTS: usize = 1_024;

/// Maximum characters accepted for a Linux task comm (TASK_COMM_LEN limit).
pub const MAX_COMM_CHARS: usize = 16;

/// Kind of observed eBPF network lifecycle event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EbpfEventType {
    /// Outgoing TCP connection attempt (e.g. `connect`).
    TcpConnect,
    /// Incoming accepted TCP connection (e.g. `accept`).
    TcpAccept,
    /// TCP connection teardown / close.
    TcpClose,
    /// TCP packet retransmission detected.
    TcpRetransmit,
}

impl EbpfEventType {
    /// Stable lowercase string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TcpConnect => "tcp_connect",
            Self::TcpAccept => "tcp_accept",
            Self::TcpClose => "tcp_close",
            Self::TcpRetransmit => "tcp_retransmit",
        }
    }
}

impl fmt::Display for EbpfEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors occurring during eBPF record validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EbpfError {
    /// A string field exceeds its bounded maximum length.
    StringTooLong {
        /// Name of the bounded field.
        field: &'static str,
        /// Maximum allowed character length.
        max: usize,
    },
    /// A field contains forbidden ASCII control characters.
    ControlCharacters(&'static str),
    /// A required string identifier was empty.
    EmptyIdentifier(&'static str),
    /// Address family of local and remote endpoints do not match.
    AddressFamilyMismatch,
    /// Port number is invalid (0 is invalid for established connections).
    ZeroPort(&'static str),
    /// An availability explanation was empty or missing.
    MissingReason,
}

impl fmt::Display for EbpfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StringTooLong { field, max } => {
                write!(f, "{field} exceeds maximum length of {max} characters")
            }
            Self::ControlCharacters(field) => {
                write!(f, "{field} contains forbidden ASCII control characters")
            }
            Self::EmptyIdentifier(field) => write!(f, "{field} cannot be empty"),
            Self::AddressFamilyMismatch => {
                write!(f, "local and remote address families must match")
            }
            Self::ZeroPort(field) => write!(f, "{field} cannot be zero"),
            Self::MissingReason => write!(f, "availability failure reason cannot be empty"),
        }
    }
}

impl Error for EbpfError {}

/// Sanitizes and validates a task comm string.
fn validate_comm(comm: &str) -> Result<String, EbpfError> {
    if comm.is_empty() {
        return Err(EbpfError::EmptyIdentifier("comm"));
    }
    if comm.len() > MAX_COMM_CHARS {
        return Err(EbpfError::StringTooLong {
            field: "comm",
            max: MAX_COMM_CHARS,
        });
    }
    if comm.chars().any(char::is_control) {
        return Err(EbpfError::ControlCharacters("comm"));
    }
    Ok(comm.to_string())
}

/// A structured, read-only event observed from an eBPF tracepoint probe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EbpfLifecycleEvent {
    event_type: EbpfEventType,
    timestamp_ns: u64,
    pid: u32,
    comm: String,
    family: AddressFamily,
    local_address: IpAddr,
    local_port: u16,
    remote_address: IpAddr,
    remote_port: u16,
    old_state: Option<SocketState>,
    new_state: Option<SocketState>,
    round_trip_us: Option<u64>,
}

impl EbpfLifecycleEvent {
    /// Creates a validated eBPF lifecycle event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_type: EbpfEventType,
        timestamp_ns: u64,
        pid: u32,
        comm: impl AsRef<str>,
        local_address: IpAddr,
        local_port: u16,
        remote_address: IpAddr,
        remote_port: u16,
    ) -> Result<Self, EbpfError> {
        let comm = validate_comm(comm.as_ref())?;
        let family = match (local_address, remote_address) {
            (IpAddr::V4(_), IpAddr::V4(_)) => AddressFamily::Ipv4,
            (IpAddr::V6(_), IpAddr::V6(_)) => AddressFamily::Ipv6,
            _ => return Err(EbpfError::AddressFamilyMismatch),
        };
        if local_port == 0 {
            return Err(EbpfError::ZeroPort("local_port"));
        }
        if remote_port == 0 {
            return Err(EbpfError::ZeroPort("remote_port"));
        }

        Ok(Self {
            event_type,
            timestamp_ns,
            pid,
            comm,
            family,
            local_address,
            local_port,
            remote_address,
            remote_port,
            old_state: None,
            new_state: None,
            round_trip_us: None,
        })
    }

    /// Attaches old and new TCP socket states to the event.
    #[must_use]
    pub fn with_states(
        mut self,
        old_state: Option<SocketState>,
        new_state: Option<SocketState>,
    ) -> Self {
        self.old_state = old_state;
        self.new_state = new_state;
        self
    }

    /// Attaches measured round-trip time in microseconds.
    #[must_use]
    pub const fn with_round_trip_us(mut self, rtt_us: Option<u64>) -> Self {
        self.round_trip_us = rtt_us;
        self
    }

    /// Kind of lifecycle event.
    #[must_use]
    pub const fn event_type(&self) -> EbpfEventType {
        self.event_type
    }

    /// Collector-monotonic timestamp in nanoseconds when the event occurred.
    #[must_use]
    pub const fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    /// Process ID that triggered the event.
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Process command name.
    #[must_use]
    pub fn comm(&self) -> &str {
        &self.comm
    }

    /// Network address family.
    #[must_use]
    pub const fn family(&self) -> AddressFamily {
        self.family
    }

    /// Local IP address.
    #[must_use]
    pub const fn local_address(&self) -> IpAddr {
        self.local_address
    }

    /// Local port number.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Remote IP address.
    #[must_use]
    pub const fn remote_address(&self) -> IpAddr {
        self.remote_address
    }

    /// Remote port number.
    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.remote_port
    }

    /// Previous socket state, if applicable.
    #[must_use]
    pub const fn old_state(&self) -> Option<SocketState> {
        self.old_state
    }

    /// New socket state, if applicable.
    #[must_use]
    pub const fn new_state(&self) -> Option<SocketState> {
        self.new_state
    }

    /// Smoothed round-trip time in microseconds, if measured.
    #[must_use]
    pub const fn round_trip_us(&self) -> Option<u64> {
        self.round_trip_us
    }
}

/// A structured diagnostic report containing eBPF telemetry events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EbpfReport {
    schema: &'static str,
    mode: SnapshotMode,
    status: SourceAvailability,
    clock_source: &'static str,
    collection_time_ns: u64,
    collection_duration_ns: u64,
    events: Vec<EbpfLifecycleEvent>,
    dropped_events: u64,
}

impl EbpfReport {
    /// Creates a new eBPF report initialized in the `Disabled` state.
    #[must_use]
    pub fn new(mode: SnapshotMode, collection_time_ns: u64) -> Self {
        Self {
            schema: EBPF_REPORT_SCHEMA_VERSION,
            mode,
            status: SourceAvailability::Disabled("eBPF collection not enabled".to_string()),
            clock_source: "collector_monotonic",
            collection_time_ns,
            collection_duration_ns: 0,
            events: Vec::new(),
            dropped_events: 0,
        }
    }

    /// Sets the status of the eBPF subsystem.
    pub fn set_status(&mut self, status: SourceAvailability) {
        self.status = status;
    }

    /// Sets collection duration in nanoseconds.
    pub fn set_collection_duration_ns(&mut self, duration_ns: u64) {
        self.collection_duration_ns = duration_ns;
    }

    /// Adds an event to the report, bounded by [`MAX_EBPF_EVENTS`].
    ///
    /// If capacity is exceeded, the event is dropped and `dropped_events` is incremented.
    pub fn push_event(&mut self, event: EbpfLifecycleEvent) {
        if self.events.len() < MAX_EBPF_EVENTS {
            self.events.push(event);
        } else {
            self.dropped_events = self.dropped_events.saturating_add(1);
        }
    }

    /// Schema version string.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        self.schema
    }

    /// Execution mode (live host vs static fixture).
    #[must_use]
    pub const fn mode(&self) -> SnapshotMode {
        self.mode
    }

    /// Availability status of the eBPF source.
    #[must_use]
    pub const fn status(&self) -> &SourceAvailability {
        &self.status
    }

    /// Collector-monotonic timestamp in nanoseconds when batch collection started.
    #[must_use]
    pub const fn collection_time_ns(&self) -> u64 {
        self.collection_time_ns
    }

    /// Measured elapsed time in nanoseconds for collecting the batch.
    #[must_use]
    pub const fn collection_duration_ns(&self) -> u64 {
        self.collection_duration_ns
    }

    /// Slice of observed lifecycle events.
    #[must_use]
    pub fn events(&self) -> &[EbpfLifecycleEvent] {
        &self.events
    }

    /// Count of events omitted due to buffer capacity bounds.
    #[must_use]
    pub const fn dropped_events(&self) -> u64 {
        self.dropped_events
    }

    /// Sets the count of dropped events from upstream ring buffer overflow.
    pub fn set_dropped_events(&mut self, count: u64) {
        self.dropped_events = count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn event_creation_and_address_family_consistency() {
        let v4_loc = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let v4_rem = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let v6_loc = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let v6_rem = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));

        let ev4 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpConnect,
            1_000,
            1234,
            "curl",
            v4_loc,
            54321,
            v4_rem,
            80,
        )
        .unwrap()
        .with_states(Some(SocketState::SynSent), Some(SocketState::Established))
        .with_round_trip_us(Some(250));

        assert_eq!(ev4.event_type(), EbpfEventType::TcpConnect);
        assert_eq!(ev4.family(), AddressFamily::Ipv4);
        assert_eq!(ev4.pid(), 1234);
        assert_eq!(ev4.comm(), "curl");
        assert_eq!(ev4.old_state(), Some(SocketState::SynSent));
        assert_eq!(ev4.new_state(), Some(SocketState::Established));
        assert_eq!(ev4.round_trip_us(), Some(250));

        let ev6 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpAccept,
            2_000,
            5678,
            "nginx",
            v6_loc,
            443,
            v6_rem,
            54322,
        )
        .unwrap();
        assert_eq!(ev6.family(), AddressFamily::Ipv6);

        // Address family mismatch must fail
        assert_eq!(
            EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                1_000,
                1234,
                "curl",
                v4_loc,
                54321,
                v6_rem,
                80,
            ),
            Err(EbpfError::AddressFamilyMismatch)
        );

        // Zero port must fail
        assert_eq!(
            EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                1_000,
                1234,
                "curl",
                v4_loc,
                0,
                v4_rem,
                80,
            ),
            Err(EbpfError::ZeroPort("local_port"))
        );
    }

    #[test]
    fn comm_bounds_and_control_characters() {
        let v4_loc = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let v4_rem = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Empty comm fails
        assert_eq!(
            EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                100,
                1,
                "",
                v4_loc,
                1000,
                v4_rem,
                80,
            ),
            Err(EbpfError::EmptyIdentifier("comm"))
        );

        // Comm exceeding 16 chars fails
        assert_eq!(
            EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                100,
                1,
                "this_process_name_is_too_long",
                v4_loc,
                1000,
                v4_rem,
                80,
            ),
            Err(EbpfError::StringTooLong {
                field: "comm",
                max: 16,
            })
        );

        // Control characters fail
        assert_eq!(
            EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                100,
                1,
                "bad\nname",
                v4_loc,
                1000,
                v4_rem,
                80,
            ),
            Err(EbpfError::ControlCharacters("comm"))
        );
    }

    #[test]
    fn report_capacity_and_drop_accounting() {
        let mut report = EbpfReport::new(SnapshotMode::Live, 5_000);
        assert_eq!(report.schema(), "0.1");
        assert_eq!(report.mode(), SnapshotMode::Live);
        assert!(!report.status().is_available());
        assert_eq!(report.events().len(), 0);

        report.set_status(SourceAvailability::Available);
        report.set_collection_duration_ns(150_000);

        let v4_loc = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let v4_rem = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        for i in 0..MAX_EBPF_EVENTS {
            let ev = EbpfLifecycleEvent::new(
                EbpfEventType::TcpConnect,
                i as u64,
                100 + i as u32,
                "worker",
                v4_loc,
                10000 + (i % 50000) as u16,
                v4_rem,
                80,
            )
            .unwrap();
            report.push_event(ev);
        }

        assert_eq!(report.events().len(), MAX_EBPF_EVENTS);
        assert_eq!(report.dropped_events(), 0);

        // Pushing excess events increments dropped_events
        let overflow_ev = EbpfLifecycleEvent::new(
            EbpfEventType::TcpClose,
            999_999,
            999,
            "overflow",
            v4_loc,
            12345,
            v4_rem,
            80,
        )
        .unwrap();
        report.push_event(overflow_ev);

        assert_eq!(report.events().len(), MAX_EBPF_EVENTS);
        assert_eq!(report.dropped_events(), 1);
    }

    #[test]
    fn serialization_roundtrip() {
        let mut report = EbpfReport::new(SnapshotMode::Fixture, 10_000);
        report.set_status(SourceAvailability::Available);
        let ev = EbpfLifecycleEvent::new(
            EbpfEventType::TcpRetransmit,
            10_500,
            4242,
            "synproxy",
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
            443,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
            55555,
        )
        .unwrap()
        .with_states(
            Some(SocketState::Established),
            Some(SocketState::Established),
        );
        report.push_event(ev);

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"schema\":\"0.1\""));
        assert!(json.contains("\"status\":\"available\""));
        assert!(json.contains("\"event_type\":\"tcp_retransmit\""));
        assert!(json.contains("\"comm\":\"synproxy\""));
    }
}
