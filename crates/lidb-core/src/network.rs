//! Typed network topology, routing, socket state, and report contracts.
//!
//! Network inventory (interfaces, routes, sockets) represents structured records,
//! distinct from numerical [`Snapshot`](crate::Snapshot) metrics. This module
//! provides bounded, versioned report contracts without filesystem, OS-specific,
//! or network dependencies.
use crate::SnapshotMode;
use serde::Serialize;
use std::{error::Error, fmt, net::IpAddr};

/// Schema version for network reports; pre-stable contract.
pub const NETWORK_REPORT_SCHEMA_VERSION: &str = "0.1";

/// Maximum interface records accepted in a single network report.
pub const MAX_NETWORK_INTERFACES: usize = 64;

/// Maximum route records accepted in a single network report.
pub const MAX_NETWORK_ROUTES: usize = 1_024;

/// Maximum socket records accepted in a single network report.
pub const MAX_NETWORK_SOCKETS: usize = 2_048;

/// Maximum characters accepted for a network interface name (standard Linux IFNAMSIZ limit).
pub const MAX_INTERFACE_NAME_CHARS: usize = 16;

/// Maximum characters accepted for a network namespace identifier.
pub const MAX_NAMESPACE_NAME_CHARS: usize = 64;

/// Maximum characters accepted for a MAC / hardware address representation.
pub const MAX_MAC_ADDRESS_CHARS: usize = 32;

/// Maximum characters accepted for human-readable failure explanations.
pub const MAX_EXPLANATION_CHARS: usize = 512;

/// Network address family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressFamily {
    /// IPv4 (32-bit addresses, prefix 0..=32).
    Ipv4,
    /// IPv6 (128-bit addresses, prefix 0..=128).
    Ipv6,
}

impl AddressFamily {
    /// Stable lowercase string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
        }
    }
}

impl fmt::Display for AddressFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validates string bounds and rejects ASCII control characters to prevent terminal injection.
fn validate_string(value: &str, field: &'static str, max_len: usize) -> Result<(), NetworkError> {
    if value.is_empty() {
        return Err(NetworkError::EmptyIdentifier(field));
    }
    if value.len() > max_len {
        return Err(NetworkError::StringTooLong {
            field,
            max: max_len,
        });
    }
    if value.chars().any(char::is_control) {
        return Err(NetworkError::ControlCharacters(field));
    }
    Ok(())
}

/// Identifies the network namespace context of collected observations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NetworkNamespace {
    name: String,
    inode: Option<u64>,
}

impl NetworkNamespace {
    /// Creates a validated namespace identity.
    pub fn new(name: impl Into<String>, inode: Option<u64>) -> Result<Self, NetworkError> {
        let name = name.into();
        validate_string(&name, "namespace_name", MAX_NAMESPACE_NAME_CHARS)?;
        Ok(Self { name, inode })
    }

    /// Human-readable namespace name or descriptor.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Kernel namespace inode number, if identified (e.g. from `/proc/self/ns/net`).
    #[must_use]
    pub const fn inode(&self) -> Option<u64> {
        self.inode
    }
}

/// Operational state of a network interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceOperState {
    /// Interface is operational and ready to send/receive packets.
    Up,
    /// Interface is administratively or operationally down.
    Down,
    /// Interface operational status cannot be determined.
    Unknown,
    /// Interface is in testing mode.
    Testing,
    /// Interface is dormant, awaiting an external event.
    Dormant,
    /// Lower layer interface is down (e.g. carrier missing).
    LowerLayerDown,
    /// Interface hardware is not present.
    NotPresent,
}

/// Metadata and attributes of an individual network interface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InterfaceRecord {
    name: String,
    ifindex: Option<u32>,
    mtu: Option<u32>,
    oper_state: InterfaceOperState,
    mac_address: Option<String>,
    speed_mbps: Option<u64>,
    flags: Vec<String>,
}

impl InterfaceRecord {
    /// Creates a new interface record with required name and operational state.
    pub fn new(
        name: impl Into<String>,
        oper_state: InterfaceOperState,
    ) -> Result<Self, NetworkError> {
        let name = name.into();
        validate_string(&name, "interface_name", MAX_INTERFACE_NAME_CHARS)?;
        Ok(Self {
            name,
            ifindex: None,
            mtu: None,
            oper_state,
            mac_address: None,
            speed_mbps: None,
            flags: Vec::new(),
        })
    }

    /// Attaches the kernel interface index (`ifindex`).
    #[must_use]
    pub const fn with_ifindex(mut self, ifindex: u32) -> Self {
        self.ifindex = Some(ifindex);
        self
    }

    /// Attaches the Maximum Transmission Unit (MTU).
    #[must_use]
    pub const fn with_mtu(mut self, mtu: u32) -> Self {
        self.mtu = Some(mtu);
        self
    }

    /// Attaches the hardware MAC address.
    pub fn with_mac_address(mut self, mac: impl Into<String>) -> Result<Self, NetworkError> {
        let mac = mac.into();
        validate_string(&mac, "mac_address", MAX_MAC_ADDRESS_CHARS)?;
        self.mac_address = Some(mac);
        Ok(self)
    }

    /// Attaches link speed in megabits per second.
    #[must_use]
    pub const fn with_speed_mbps(mut self, speed_mbps: u64) -> Self {
        self.speed_mbps = Some(speed_mbps);
        self
    }

    /// Attaches validated interface flags (e.g. `UP`, `BROADCAST`, `MULTICAST`, `LOOPBACK`).
    pub fn with_flags(mut self, flags: Vec<String>) -> Result<Self, NetworkError> {
        for flag in &flags {
            validate_string(flag, "interface_flag", 32)?;
        }
        self.flags = flags;
        Ok(self)
    }

    /// Network interface name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Kernel interface index.
    #[must_use]
    pub const fn ifindex(&self) -> Option<u32> {
        self.ifindex
    }

    /// MTU in bytes.
    #[must_use]
    pub const fn mtu(&self) -> Option<u32> {
        self.mtu
    }

    /// Operational state.
    #[must_use]
    pub const fn oper_state(&self) -> InterfaceOperState {
        self.oper_state
    }

    /// MAC address string.
    #[must_use]
    pub fn mac_address(&self) -> Option<&str> {
        self.mac_address.as_deref()
    }

    /// Link speed in megabits per second.
    #[must_use]
    pub const fn speed_mbps(&self) -> Option<u64> {
        self.speed_mbps
    }

    /// Interface flags.
    #[must_use]
    pub fn flags(&self) -> &[String] {
        &self.flags
    }
}

/// A structured routing table entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RouteRecord {
    destination: IpAddr,
    prefix_len: u8,
    gateway: Option<IpAddr>,
    interface: String,
    metric: Option<u32>,
    family: AddressFamily,
}

impl RouteRecord {
    /// Creates a validated route record.
    pub fn new(
        destination: IpAddr,
        prefix_len: u8,
        interface: impl Into<String>,
    ) -> Result<Self, NetworkError> {
        let interface = interface.into();
        validate_string(&interface, "interface_name", MAX_INTERFACE_NAME_CHARS)?;

        let family = match destination {
            IpAddr::V4(_) => {
                if prefix_len > 32 {
                    return Err(NetworkError::InvalidPrefixLen {
                        family: AddressFamily::Ipv4,
                        prefix_len,
                    });
                }
                AddressFamily::Ipv4
            }
            IpAddr::V6(_) => {
                if prefix_len > 128 {
                    return Err(NetworkError::InvalidPrefixLen {
                        family: AddressFamily::Ipv6,
                        prefix_len,
                    });
                }
                AddressFamily::Ipv6
            }
        };

        Ok(Self {
            destination,
            prefix_len,
            gateway: None,
            interface,
            metric: None,
            family,
        })
    }

    /// Attaches a gateway/next-hop address, verifying address family consistency.
    pub fn with_gateway(mut self, gateway: IpAddr) -> Result<Self, NetworkError> {
        let gateway_family = match gateway {
            IpAddr::V4(_) => AddressFamily::Ipv4,
            IpAddr::V6(_) => AddressFamily::Ipv6,
        };
        if gateway_family != self.family {
            return Err(NetworkError::AddressFamilyMismatch);
        }
        self.gateway = Some(gateway);
        Ok(self)
    }

    /// Attaches route metric/priority.
    #[must_use]
    pub const fn with_metric(mut self, metric: u32) -> Self {
        self.metric = Some(metric);
        self
    }

    /// Destination network IP address.
    #[must_use]
    pub const fn destination(&self) -> IpAddr {
        self.destination
    }

    /// Network prefix length in bits (0..=32 for IPv4, 0..=128 for IPv6).
    #[must_use]
    pub const fn prefix_len(&self) -> u8 {
        self.prefix_len
    }

    /// Gateway address, or `None` if directly connected.
    #[must_use]
    pub const fn gateway(&self) -> Option<IpAddr> {
        self.gateway
    }

    /// Egress network interface name.
    #[must_use]
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Route priority or metric.
    #[must_use]
    pub const fn metric(&self) -> Option<u32> {
        self.metric
    }

    /// Address family (IPv4 or IPv6).
    #[must_use]
    pub const fn family(&self) -> AddressFamily {
        self.family
    }
}

/// Transport protocol for socket observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SocketProtocol {
    /// Transmission Control Protocol.
    Tcp,
    /// User Datagram Protocol.
    Udp,
}

/// Finite socket connection state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SocketState {
    /// Active connection established.
    Established,
    /// SYN packet sent; waiting for response.
    SynSent,
    /// SYN packet received; waiting for ACK.
    SynRecv,
    /// Connection close initiated; waiting for ACK.
    FinWait1,
    /// Connection close acknowledged; waiting for remote close.
    FinWait2,
    /// Waiting for enough time to ensure remote received close ACK.
    TimeWait,
    /// Socket is closed.
    Closed,
    /// Remote end shut down; waiting for local close.
    CloseWait,
    /// Awaiting ACK of local close.
    LastAck,
    /// Socket is listening for incoming connections.
    Listen,
    /// Both sides closed concurrently.
    Closing,
    /// Protocol state could not be determined.
    Unknown,
}

/// An observed socket endpoint record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocketRecord {
    protocol: SocketProtocol,
    local_address: IpAddr,
    local_port: u16,
    remote_address: Option<IpAddr>,
    remote_port: Option<u16>,
    state: SocketState,
    inode: Option<u64>,
    family: AddressFamily,
}

impl SocketRecord {
    /// Creates a socket record with required local endpoint and state.
    #[must_use]
    pub fn new(
        protocol: SocketProtocol,
        local_address: IpAddr,
        local_port: u16,
        state: SocketState,
    ) -> Self {
        let family = match local_address {
            IpAddr::V4(_) => AddressFamily::Ipv4,
            IpAddr::V6(_) => AddressFamily::Ipv6,
        };
        Self {
            protocol,
            local_address,
            local_port,
            remote_address: None,
            remote_port: None,
            state,
            inode: None,
            family,
        }
    }

    /// Attaches a remote peer endpoint, verifying address family consistency.
    pub fn with_remote(
        mut self,
        remote_address: IpAddr,
        remote_port: u16,
    ) -> Result<Self, NetworkError> {
        let remote_family = match remote_address {
            IpAddr::V4(_) => AddressFamily::Ipv4,
            IpAddr::V6(_) => AddressFamily::Ipv6,
        };
        if remote_family != self.family {
            return Err(NetworkError::AddressFamilyMismatch);
        }
        self.remote_address = Some(remote_address);
        self.remote_port = Some(remote_port);
        Ok(self)
    }

    /// Attaches the socket inode number.
    #[must_use]
    pub const fn with_inode(mut self, inode: u64) -> Self {
        self.inode = Some(inode);
        self
    }

    /// Transport protocol.
    #[must_use]
    pub const fn protocol(&self) -> SocketProtocol {
        self.protocol
    }

    /// Local binding address.
    #[must_use]
    pub const fn local_address(&self) -> IpAddr {
        self.local_address
    }

    /// Local port number.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Remote peer address, if connected.
    #[must_use]
    pub const fn remote_address(&self) -> Option<IpAddr> {
        self.remote_address
    }

    /// Remote peer port, if connected.
    #[must_use]
    pub const fn remote_port(&self) -> Option<u16> {
        self.remote_port
    }

    /// Socket state.
    #[must_use]
    pub const fn state(&self) -> SocketState {
        self.state
    }

    /// Socket inode identifier.
    #[must_use]
    pub const fn inode(&self) -> Option<u64> {
        self.inode
    }

    /// Address family.
    #[must_use]
    pub const fn family(&self) -> AddressFamily {
        self.family
    }
}

/// Truthful availability for a structured table source.
///
/// Differentiates between a successfully read source that is legitimately empty
/// (`Available` with 0 records) and a source collection failure (`PermissionDenied`,
/// `Unsupported`, `TemporarilyUnavailable`, etc.).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum SourceAvailability {
    /// The source table was successfully read (may contain 0 or more records).
    Available,
    /// The kernel or operating system does not support this table.
    Unsupported(String),
    /// Intentionally disabled in current configuration.
    Disabled(String),
    /// The collector lacks read permission.
    PermissionDenied(String),
    /// Transient failure prevented reading.
    TemporarilyUnavailable(String),
    /// An unexpected parsing or collection error occurred.
    Error(String),
}

impl SourceAvailability {
    /// Validates an explanation string for an unavailable source.
    fn validated_reason(reason: impl Into<String>) -> Result<String, NetworkError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(NetworkError::MissingReason);
        }
        if reason.len() > MAX_EXPLANATION_CHARS {
            return Err(NetworkError::StringTooLong {
                field: "availability_reason",
                max: MAX_EXPLANATION_CHARS,
            });
        }
        if reason.chars().any(char::is_control) {
            return Err(NetworkError::ControlCharacters("availability_reason"));
        }
        Ok(reason)
    }

    /// Creates an `Unsupported` availability with a validated explanation.
    pub fn unsupported(reason: impl Into<String>) -> Result<Self, NetworkError> {
        Ok(Self::Unsupported(Self::validated_reason(reason)?))
    }

    /// Creates a `Disabled` availability with a validated explanation.
    pub fn disabled(reason: impl Into<String>) -> Result<Self, NetworkError> {
        Ok(Self::Disabled(Self::validated_reason(reason)?))
    }

    /// Creates a `PermissionDenied` availability with a validated explanation.
    pub fn permission_denied(reason: impl Into<String>) -> Result<Self, NetworkError> {
        Ok(Self::PermissionDenied(Self::validated_reason(reason)?))
    }

    /// Creates a `TemporarilyUnavailable` availability with a validated explanation.
    pub fn temporarily_unavailable(reason: impl Into<String>) -> Result<Self, NetworkError> {
        Ok(Self::TemporarilyUnavailable(Self::validated_reason(
            reason,
        )?))
    }

    /// Creates an `Error` availability with a validated explanation.
    pub fn error(reason: impl Into<String>) -> Result<Self, NetworkError> {
        Ok(Self::Error(Self::validated_reason(reason)?))
    }

    /// Returns `true` if the source was successfully collected.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Machine-readable status name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unsupported(_) => "unsupported",
            Self::Disabled(_) => "disabled",
            Self::PermissionDenied(_) => "permission_denied",
            Self::TemporarilyUnavailable(_) => "temporarily_unavailable",
            Self::Error(_) => "error",
        }
    }

    /// Explanation of unavailability, if not available.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Unsupported(r)
            | Self::Disabled(r)
            | Self::PermissionDenied(r)
            | Self::TemporarilyUnavailable(r)
            | Self::Error(r) => Some(r),
        }
    }
}

/// A versioned, bounded report of structured Linux network state.
#[derive(Clone, Debug, Serialize)]
pub struct NetworkReport {
    schema_version: &'static str,
    clock_source: &'static str,
    monotonic_ns: u64,
    collection_duration_ns: u64,
    mode: SnapshotMode,
    namespace: Option<NetworkNamespace>,
    interfaces_status: SourceAvailability,
    routes_status: SourceAvailability,
    sockets_status: SourceAvailability,
    interfaces: Vec<InterfaceRecord>,
    routes: Vec<RouteRecord>,
    sockets: Vec<SocketRecord>,
    dropped_interfaces: u64,
    dropped_routes: u64,
    dropped_sockets: u64,
}

impl NetworkReport {
    /// Creates an initial network report; all sources default to `Available`.
    #[must_use]
    pub fn new(monotonic_ns: u64, collection_duration_ns: u64, mode: SnapshotMode) -> Self {
        Self {
            schema_version: NETWORK_REPORT_SCHEMA_VERSION,
            clock_source: "collector_monotonic",
            monotonic_ns,
            collection_duration_ns,
            mode,
            namespace: None,
            interfaces_status: SourceAvailability::Available,
            routes_status: SourceAvailability::Available,
            sockets_status: SourceAvailability::Available,
            interfaces: Vec::new(),
            routes: Vec::new(),
            sockets: Vec::new(),
            dropped_interfaces: 0,
            dropped_routes: 0,
            dropped_sockets: 0,
        }
    }

    /// Sets the network namespace identity.
    #[must_use]
    pub fn with_namespace(mut self, namespace: NetworkNamespace) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Replaces the measurement duration after collection finishes.
    pub fn set_collection_duration_ns(&mut self, duration_ns: u64) {
        self.collection_duration_ns = duration_ns;
    }

    /// Sets the source availability status for interfaces.
    pub fn set_interfaces_status(&mut self, status: SourceAvailability) {
        self.interfaces_status = status;
    }

    /// Sets the source availability status for routes.
    pub fn set_routes_status(&mut self, status: SourceAvailability) {
        self.routes_status = status;
    }

    /// Sets the source availability status for sockets.
    pub fn set_sockets_status(&mut self, status: SourceAvailability) {
        self.sockets_status = status;
    }

    /// Appends an interface record, enforcing capacity limits.
    pub fn push_interface(&mut self, record: InterfaceRecord) -> Result<(), NetworkError> {
        if self.interfaces.len() >= MAX_NETWORK_INTERFACES {
            self.dropped_interfaces = self.dropped_interfaces.saturating_add(1);
            return Err(NetworkError::Capacity("interfaces"));
        }
        self.interfaces.push(record);
        Ok(())
    }

    /// Appends a route record, enforcing capacity limits.
    pub fn push_route(&mut self, record: RouteRecord) -> Result<(), NetworkError> {
        if self.routes.len() >= MAX_NETWORK_ROUTES {
            self.dropped_routes = self.dropped_routes.saturating_add(1);
            return Err(NetworkError::Capacity("routes"));
        }
        self.routes.push(record);
        Ok(())
    }

    /// Appends a socket record, enforcing capacity limits.
    pub fn push_socket(&mut self, record: SocketRecord) -> Result<(), NetworkError> {
        if self.sockets.len() >= MAX_NETWORK_SOCKETS {
            self.dropped_sockets = self.dropped_sockets.saturating_add(1);
            return Err(NetworkError::Capacity("sockets"));
        }
        self.sockets.push(record);
        Ok(())
    }

    /// Schema version string.
    #[must_use]
    pub const fn schema_version(&self) -> &'static str {
        self.schema_version
    }

    /// Clock source description.
    #[must_use]
    pub const fn clock_source(&self) -> &'static str {
        self.clock_source
    }

    /// Monotonic timestamp at collection start.
    #[must_use]
    pub const fn monotonic_ns(&self) -> u64 {
        self.monotonic_ns
    }

    /// Collection duration in nanoseconds.
    #[must_use]
    pub const fn collection_duration_ns(&self) -> u64 {
        self.collection_duration_ns
    }

    /// Live host or fixture collection mode.
    #[must_use]
    pub const fn mode(&self) -> SnapshotMode {
        self.mode
    }

    /// Network namespace, if captured.
    #[must_use]
    pub fn namespace(&self) -> Option<&NetworkNamespace> {
        self.namespace.as_ref()
    }

    /// Availability status of the interfaces source.
    #[must_use]
    pub const fn interfaces_status(&self) -> &SourceAvailability {
        &self.interfaces_status
    }

    /// Availability status of the routes source.
    #[must_use]
    pub const fn routes_status(&self) -> &SourceAvailability {
        &self.routes_status
    }

    /// Availability status of the sockets source.
    #[must_use]
    pub const fn sockets_status(&self) -> &SourceAvailability {
        &self.sockets_status
    }

    /// Collected interface records.
    #[must_use]
    pub fn interfaces(&self) -> &[InterfaceRecord] {
        &self.interfaces
    }

    /// Collected route records.
    #[must_use]
    pub fn routes(&self) -> &[RouteRecord] {
        &self.routes
    }

    /// Collected socket records.
    #[must_use]
    pub fn sockets(&self) -> &[SocketRecord] {
        &self.sockets
    }

    /// Total count of dropped records due to capacity constraints across all tables.
    #[must_use]
    pub fn dropped_records(&self) -> u64 {
        self.dropped_interfaces
            .saturating_add(self.dropped_routes)
            .saturating_add(self.dropped_sockets)
    }

    /// Count of dropped interface records.
    #[must_use]
    pub const fn dropped_interfaces(&self) -> u64 {
        self.dropped_interfaces
    }

    /// Count of dropped route records.
    #[must_use]
    pub const fn dropped_routes(&self) -> u64 {
        self.dropped_routes
    }

    /// Count of dropped socket records.
    #[must_use]
    pub const fn dropped_sockets(&self) -> u64 {
        self.dropped_sockets
    }
}

/// Errors returned when validating or constructing network records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkError {
    /// A table capacity limit was exceeded.
    Capacity(&'static str),
    /// An identifier string was empty.
    EmptyIdentifier(&'static str),
    /// A string exceeded its maximum length limit.
    StringTooLong {
        /// Name of the affected field.
        field: &'static str,
        /// Maximum allowed character length.
        max: usize,
    },
    /// A string contained forbidden ASCII control characters.
    ControlCharacters(&'static str),
    /// A route prefix length was invalid for the specified address family.
    InvalidPrefixLen {
        /// The route's address family.
        family: AddressFamily,
        /// The invalid prefix length provided.
        prefix_len: u8,
    },
    /// Addresses in the same record belonged to conflicting address families.
    AddressFamilyMismatch,
    /// An unavailable source omitted an explanatory reason.
    MissingReason,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity(table) => write!(f, "network report table capacity exceeded: {table}"),
            Self::EmptyIdentifier(field) => write!(f, "network field must not be empty: {field}"),
            Self::StringTooLong { field, max } => {
                write!(
                    f,
                    "network field '{field}' exceeds limit of {max} characters"
                )
            }
            Self::ControlCharacters(field) => {
                write!(
                    f,
                    "network field contains forbidden control characters: {field}"
                )
            }
            Self::InvalidPrefixLen { family, prefix_len } => {
                write!(f, "invalid prefix length {prefix_len} for {family}")
            }
            Self::AddressFamilyMismatch => {
                f.write_str("addresses in record must belong to the same address family")
            }
            Self::MissingReason => f.write_str("unavailable source requires an explanatory reason"),
        }
    }
}

impl Error for NetworkError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn address_family_names_and_display() {
        assert_eq!(AddressFamily::Ipv4.as_str(), "ipv4");
        assert_eq!(AddressFamily::Ipv6.as_str(), "ipv6");
        assert_eq!(AddressFamily::Ipv4.to_string(), "ipv4");
        assert_eq!(AddressFamily::Ipv6.to_string(), "ipv6");
    }

    #[test]
    fn interface_record_validation_and_builder() {
        let iface = InterfaceRecord::new("eth0", InterfaceOperState::Up)
            .unwrap()
            .with_ifindex(2)
            .with_mtu(1500)
            .with_mac_address("52:54:00:12:34:56")
            .unwrap()
            .with_speed_mbps(1000)
            .with_flags(vec!["UP".into(), "BROADCAST".into(), "MULTICAST".into()])
            .unwrap();

        assert_eq!(iface.name(), "eth0");
        assert_eq!(iface.ifindex(), Some(2));
        assert_eq!(iface.mtu(), Some(1500));
        assert_eq!(iface.oper_state(), InterfaceOperState::Up);
        assert_eq!(iface.mac_address(), Some("52:54:00:12:34:56"));
        assert_eq!(iface.speed_mbps(), Some(1000));
        assert_eq!(iface.flags().len(), 3);

        // Rejection of empty interface name
        assert_eq!(
            InterfaceRecord::new("", InterfaceOperState::Down),
            Err(NetworkError::EmptyIdentifier("interface_name"))
        );

        // Rejection of oversized interface name
        assert_eq!(
            InterfaceRecord::new("a".repeat(17), InterfaceOperState::Down),
            Err(NetworkError::StringTooLong {
                field: "interface_name",
                max: MAX_INTERFACE_NAME_CHARS
            })
        );

        // Rejection of control characters in name
        assert_eq!(
            InterfaceRecord::new("eth0\n", InterfaceOperState::Down),
            Err(NetworkError::ControlCharacters("interface_name"))
        );

        // Rejection of control characters in MAC
        assert!(iface.with_mac_address("aa:bb\x1b[31m").is_err());
    }

    #[test]
    fn route_record_ipv4_and_ipv6_validation() {
        let v4_dest = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0));
        let v4_gw = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let route_v4 = RouteRecord::new(v4_dest, 24, "eth0")
            .unwrap()
            .with_gateway(v4_gw)
            .unwrap()
            .with_metric(100);

        assert_eq!(route_v4.family(), AddressFamily::Ipv4);
        assert_eq!(route_v4.prefix_len(), 24);
        assert_eq!(route_v4.gateway(), Some(v4_gw));
        assert_eq!(route_v4.metric(), Some(100));

        // Invalid IPv4 prefix (> 32)
        assert_eq!(
            RouteRecord::new(v4_dest, 33, "eth0"),
            Err(NetworkError::InvalidPrefixLen {
                family: AddressFamily::Ipv4,
                prefix_len: 33
            })
        );

        let v6_dest = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0));
        let v6_gw = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let route_v6 = RouteRecord::new(v6_dest, 64, "eth0")
            .unwrap()
            .with_gateway(v6_gw)
            .unwrap();

        assert_eq!(route_v6.family(), AddressFamily::Ipv6);
        assert_eq!(route_v6.prefix_len(), 64);

        // Invalid IPv6 prefix (> 128)
        assert_eq!(
            RouteRecord::new(v6_dest, 129, "eth0"),
            Err(NetworkError::InvalidPrefixLen {
                family: AddressFamily::Ipv6,
                prefix_len: 129
            })
        );

        // Address family mismatch (IPv4 dest with IPv6 gateway)
        assert_eq!(
            route_v4.with_gateway(v6_gw),
            Err(NetworkError::AddressFamilyMismatch)
        );
    }

    #[test]
    fn socket_record_validation() {
        let v4_local = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let v4_remote = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));
        let sock = SocketRecord::new(SocketProtocol::Tcp, v4_local, 8080, SocketState::Listen)
            .with_inode(12345);

        assert_eq!(sock.family(), AddressFamily::Ipv4);
        assert_eq!(sock.protocol(), SocketProtocol::Tcp);
        assert_eq!(sock.local_port(), 8080);
        assert_eq!(sock.state(), SocketState::Listen);
        assert_eq!(sock.inode(), Some(12345));
        assert!(sock.remote_address().is_none());

        let connected = sock.clone().with_remote(v4_remote, 443).unwrap();
        assert_eq!(connected.remote_address(), Some(v4_remote));
        assert_eq!(connected.remote_port(), Some(443));

        // Family mismatch
        let v6_remote = IpAddr::V6(Ipv6Addr::LOCALHOST);
        assert_eq!(
            sock.with_remote(v6_remote, 443),
            Err(NetworkError::AddressFamilyMismatch)
        );
    }

    #[test]
    fn source_availability_differentiates_empty_from_failed() {
        let avail = SourceAvailability::Available;
        assert!(avail.is_available());
        assert_eq!(avail.as_str(), "available");
        assert!(avail.reason().is_none());

        let denied = SourceAvailability::permission_denied("read /proc/net/tcp denied").unwrap();
        assert!(!denied.is_available());
        assert_eq!(denied.as_str(), "permission_denied");
        assert_eq!(denied.reason(), Some("read /proc/net/tcp denied"));

        // Reject empty or whitespace reason
        assert_eq!(
            SourceAvailability::unsupported("   "),
            Err(NetworkError::MissingReason)
        );

        // Reject control characters in explanation
        assert_eq!(
            SourceAvailability::error("error\r\nexploit"),
            Err(NetworkError::ControlCharacters("availability_reason"))
        );
    }

    #[test]
    fn network_report_capacity_and_dropped_counters() {
        let mut report = NetworkReport::new(1_000_000, 50_000, SnapshotMode::Live);
        assert_eq!(report.schema_version(), "0.1");
        assert_eq!(report.clock_source(), "collector_monotonic");
        assert_eq!(report.dropped_records(), 0);

        // Fill interfaces to capacity
        for i in 0..MAX_NETWORK_INTERFACES {
            report
                .push_interface(
                    InterfaceRecord::new(format!("eth{i}"), InterfaceOperState::Up).unwrap(),
                )
                .unwrap();
        }
        assert_eq!(report.interfaces().len(), MAX_NETWORK_INTERFACES);
        assert_eq!(report.dropped_interfaces(), 0);

        // Exceeding capacity drops record and increments dropped counter
        let overflow = InterfaceRecord::new("overflow0", InterfaceOperState::Down).unwrap();
        assert_eq!(
            report.push_interface(overflow),
            Err(NetworkError::Capacity("interfaces"))
        );
        assert_eq!(report.dropped_interfaces(), 1);
        assert_eq!(report.dropped_records(), 1);

        // Fill routes to capacity
        let dest = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0));
        for _ in 0..MAX_NETWORK_ROUTES {
            report
                .push_route(RouteRecord::new(dest, 16, "eth0").unwrap())
                .unwrap();
        }
        assert_eq!(report.routes().len(), MAX_NETWORK_ROUTES);

        let overflow_route = RouteRecord::new(dest, 8, "eth0").unwrap();
        assert_eq!(
            report.push_route(overflow_route),
            Err(NetworkError::Capacity("routes"))
        );
        assert_eq!(report.dropped_routes(), 1);
        assert_eq!(report.dropped_records(), 2);
    }

    #[test]
    fn network_report_json_serialization_roundtrip() {
        let mut report = NetworkReport::new(100_000_000, 250_000, SnapshotMode::Live)
            .with_namespace(NetworkNamespace::new("init", Some(4026531992)).unwrap());

        report
            .push_interface(
                InterfaceRecord::new("eth0", InterfaceOperState::Up)
                    .unwrap()
                    .with_ifindex(1)
                    .with_mtu(1500)
                    .with_mac_address("00:11:22:33:44:55")
                    .unwrap(),
            )
            .unwrap();

        report
            .push_route(
                RouteRecord::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0, "eth0")
                    .unwrap()
                    .with_gateway(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
                    .unwrap()
                    .with_metric(100),
            )
            .unwrap();

        report
            .push_socket(SocketRecord::new(
                SocketProtocol::Tcp,
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                22,
                SocketState::Listen,
            ))
            .unwrap();

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"schema_version\":\"0.1\""));
        assert!(json.contains("\"name\":\"eth0\""));
        assert!(json.contains("\"destination\":\"0.0.0.0\""));
        assert!(json.contains("\"gateway\":\"192.168.1.1\""));
        assert!(json.contains("\"protocol\":\"tcp\""));
        assert!(json.contains("\"local_port\":22"));
        assert!(json.contains("\"state\":\"listen\""));
        assert!(json.contains("\"dropped_interfaces\":0"));
    }
}
