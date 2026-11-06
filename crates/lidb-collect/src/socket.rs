//! Bounded, read-only TCP and UDP socket observations from Linux procfs.
//!
//! Parses `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, and `/proc/net/udp6`
//! into structured, validated [`SocketRecord`] entries. Enforces [`MAX_NETWORK_SOCKETS`]
//! bounds, rejects ASCII control characters, and differentiates empty tables from
//! failed sources truthfully.
//!
//! # Privacy Boundary
//! Socket inspection is strictly read-only and metadata-only. It does not inspect
//! `/proc/<pid>` command lines, environments, or map socket inodes to process IDs.

use crate::{read, MAX_SOURCE_BYTES};
use lidb_core::network::{
    AddressFamily, NetworkError, NetworkReport, SocketProtocol, SocketRecord, SocketState,
    SourceAvailability, MAX_NETWORK_SOCKETS,
};
use lidb_core::{MetricState, MetricValue, SnapshotMode};
use serde::Serialize;
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::time::Instant;

/// Pre-stable schema for the socket-only report, separate from snapshot JSON.
pub const SOCKET_REPORT_SCHEMA_VERSION: &str = "0.1";

/// Outcome of reading host socket tables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SocketReport {
    /// Socket-report schema version.
    pub schema_version: &'static str,
    /// Origin of the batch timestamp; comparable only within one collector lifetime.
    pub clock_source: &'static str,
    /// Batch start in nanoseconds since construction of the owning collector.
    pub monotonic_ns: u64,
    /// Duration of the sequential socket table reads, in nanoseconds.
    pub collection_duration_ns: u64,
    /// Live process procfs view or explicitly selected fixture input.
    pub mode: SnapshotMode,
    /// TCP IPv4 source identifier, without a private fixture path.
    pub tcp_v4_source: &'static str,
    /// TCP IPv6 source identifier, without a private fixture path.
    pub tcp_v6_source: &'static str,
    /// UDP IPv4 source identifier, without a private fixture path.
    pub udp_v4_source: &'static str,
    /// UDP IPv6 source identifier, without a private fixture path.
    pub udp_v6_source: &'static str,
    /// Availability status of `/proc/net/tcp`.
    pub tcp_v4_status: SourceAvailability,
    /// Availability status of `/proc/net/tcp6`.
    pub tcp_v6_status: SourceAvailability,
    /// Availability status of `/proc/net/udp`.
    pub udp_v4_status: SourceAvailability,
    /// Availability status of `/proc/net/udp6`.
    pub udp_v6_status: SourceAvailability,
    /// Parsed socket records, bounded by [`MAX_NETWORK_SOCKETS`].
    pub sockets: Vec<SocketRecord>,
    /// Number of valid socket entries dropped because [`MAX_NETWORK_SOCKETS`] capacity was reached.
    pub dropped_sockets: u64,
}

impl SocketReport {
    /// Creates a report with default uncollected status.
    #[must_use]
    pub fn new(mode: SnapshotMode, monotonic_ns: u64) -> Self {
        Self {
            schema_version: SOCKET_REPORT_SCHEMA_VERSION,
            clock_source: "collector_monotonic",
            monotonic_ns,
            collection_duration_ns: 0,
            mode,
            tcp_v4_source: "procfs/net/tcp",
            tcp_v6_source: "procfs/net/tcp6",
            udp_v4_source: "procfs/net/udp",
            udp_v6_source: "procfs/net/udp6",
            tcp_v4_status: SourceAvailability::Disabled("source not collected".into()),
            tcp_v6_status: SourceAvailability::Disabled("source not collected".into()),
            udp_v4_status: SourceAvailability::Disabled("source not collected".into()),
            udp_v6_status: SourceAvailability::Disabled("source not collected".into()),
            sockets: Vec::new(),
            dropped_sockets: 0,
        }
    }

    /// Evaluates the consolidated socket availability status.
    ///
    /// If at least one table succeeded, returns [`SourceAvailability::Available`].
    /// Otherwise, reports the underlying failure (e.g. `PermissionDenied` or `Unsupported`).
    #[must_use]
    pub fn consolidated_status(&self) -> SourceAvailability {
        let statuses = [
            &self.tcp_v4_status,
            &self.tcp_v6_status,
            &self.udp_v4_status,
            &self.udp_v6_status,
        ];
        if statuses
            .iter()
            .any(|s| matches!(s, SourceAvailability::Available))
        {
            return SourceAvailability::Available;
        }
        if let Some(s) = statuses
            .iter()
            .find(|s| matches!(s, SourceAvailability::PermissionDenied(_)))
        {
            return (*s).clone();
        }
        if let Some(s) = statuses
            .iter()
            .find(|s| matches!(s, SourceAvailability::Error(_)))
        {
            return (*s).clone();
        }
        if let Some(s) = statuses
            .iter()
            .find(|s| matches!(s, SourceAvailability::TemporarilyUnavailable(_)))
        {
            return (*s).clone();
        }
        if let Some(s) = statuses
            .iter()
            .find(|s| matches!(s, SourceAvailability::Disabled(_)))
        {
            return (*s).clone();
        }
        self.tcp_v4_status.clone()
    }

    /// Appends sockets and omission counts into an existing network report.
    ///
    /// Rejects known mode, clock source, monotonic timestamp, or duration mismatch.
    /// Rejection is atomic without mutating the target.
    pub fn populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError> {
        if report.mode() != self.mode
            || report.monotonic_ns() != self.monotonic_ns
            || report.collection_duration_ns() < self.collection_duration_ns
            || report.clock_source() != self.clock_source
        {
            return Err(NetworkError::ReportContextMismatch);
        }
        report.set_sockets_status(self.consolidated_status());
        report.record_dropped_sockets(self.dropped_sockets);
        for socket in &self.sockets {
            let _ = report.push_socket(socket.clone());
        }
        Ok(())
    }
}

/// Parsed sockets from a socket table source, containing retained records and omission count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSockets {
    /// Parsed socket records, bounded by [`MAX_NETWORK_SOCKETS`].
    pub sockets: Vec<SocketRecord>,
    /// Number of valid socket entries dropped because [`MAX_NETWORK_SOCKETS`] capacity was reached.
    pub dropped_sockets: u64,
}

impl ParsedSockets {
    /// Creates a new `ParsedSockets` container.
    #[must_use]
    pub const fn new(sockets: Vec<SocketRecord>, dropped_sockets: u64) -> Self {
        Self {
            sockets,
            dropped_sockets,
        }
    }
}

/// Converts internal collector `MetricState` into `SourceAvailability`.
fn metric_state_to_availability(state: MetricState<MetricValue>) -> SourceAvailability {
    match state {
        MetricState::Available(_) => SourceAvailability::Available,
        MetricState::Unsupported(reason) => SourceAvailability::Unsupported(reason),
        MetricState::Disabled(reason) => SourceAvailability::Disabled(reason),
        MetricState::PermissionDenied(reason) => SourceAvailability::PermissionDenied(reason),
        MetricState::TemporarilyUnavailable(reason) | MetricState::Stale(reason) => {
            SourceAvailability::TemporarilyUnavailable(reason)
        }
        MetricState::Error(reason) => SourceAvailability::Error(reason),
    }
}

/// Parses a 32-hex IPv6 procfs address and 4-hex port into `(IpAddr, u16)`.
///
/// Linux formats IPv6 addresses in procfs using four 32-bit words printed in host byte order.
/// Using `.to_le_bytes()` decodes each 32-bit word back into original network octets.
fn parse_v6_address_port(s: &str) -> Result<(IpAddr, u16), SourceAvailability> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(SourceAvailability::Error(
            "malformed address:port field".into(),
        ));
    }
    let ip_hex = parts[0];
    let port_hex = parts[1];

    if ip_hex.len() != 32 {
        return Err(SourceAvailability::Error(
            "ipv6 address must be exactly 32 hex digits".into(),
        ));
    }
    if port_hex.len() != 4 {
        return Err(SourceAvailability::Error(
            "port must be exactly 4 hex digits".into(),
        ));
    }

    let mut octets = [0u8; 16];
    for (i, chunk) in ip_hex.as_bytes().chunks_exact(8).enumerate() {
        let chunk_str = std::str::from_utf8(chunk)
            .map_err(|_| SourceAvailability::Error("invalid utf-8 in ipv6 address".into()))?;
        let word = u32::from_str_radix(chunk_str, 16)
            .map_err(|_| SourceAvailability::Error("invalid hex in ipv6 address".into()))?;
        let bytes = word.to_le_bytes();
        octets[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }
    let ip = IpAddr::V6(Ipv6Addr::from(octets));

    let port = u16::from_str_radix(port_hex, 16)
        .map_err(|_| SourceAvailability::Error("invalid hex in port".into()))?;

    Ok((ip, port))
}

/// Parses an 8-hex IPv4 procfs address and 4-hex port into `(IpAddr, u16)`.
fn parse_v4_address_port(s: &str) -> Result<(IpAddr, u16), SourceAvailability> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(SourceAvailability::Error(
            "malformed address:port field".into(),
        ));
    }
    let ip_hex = parts[0];
    let port_hex = parts[1];

    if ip_hex.len() != 8 {
        return Err(SourceAvailability::Error(
            "ipv4 address must be exactly 8 hex digits".into(),
        ));
    }
    if port_hex.len() != 4 {
        return Err(SourceAvailability::Error(
            "port must be exactly 4 hex digits".into(),
        ));
    }

    let ip_u32 = u32::from_str_radix(ip_hex, 16)
        .map_err(|_| SourceAvailability::Error("invalid hex in ipv4 address".into()))?;
    let ip = IpAddr::V4(Ipv4Addr::from(ip_u32.to_le_bytes()));

    let port = u16::from_str_radix(port_hex, 16)
        .map_err(|_| SourceAvailability::Error("invalid hex in port".into()))?;

    Ok((ip, port))
}

/// Common socket table parser logic for IPv4 and IPv6 procfs sources.
fn parse_sockets_internal(
    text: &str,
    protocol: SocketProtocol,
    family: AddressFamily,
) -> Result<ParsedSockets, SourceAvailability> {
    if text.len() > MAX_SOURCE_BYTES {
        return Err(SourceAvailability::Error(
            "socket source exceeds maximum byte limit".into(),
        ));
    }
    if text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(SourceAvailability::Error(
            "socket source contains unexpected control characters".into(),
        ));
    }

    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first_line) = lines.next() else {
        return Ok(ParsedSockets::new(Vec::new(), 0));
    };

    let first_is_header = first_line.starts_with("sl") || first_line.contains("local_address");
    let first_data_line = if first_is_header {
        if !first_line.contains("local_address")
            || !first_line.contains("rem_address")
            || !first_line.contains("st")
        {
            return Err(SourceAvailability::Error(
                "socket header is malformed".into(),
            ));
        }
        None
    } else {
        Some(first_line)
    };

    let data_lines = first_data_line.into_iter().chain(lines);

    let mut sockets = Vec::with_capacity(MAX_NETWORK_SOCKETS.min(64));
    let mut seen = BTreeSet::new();
    let mut dropped_sockets = 0u64;
    let mut last_overflow: Option<SocketRecord> = None;

    for line in data_lines {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            return Err(SourceAvailability::Error(
                "socket line lacks required fields".into(),
            ));
        }

        let slot_str = cols[0]
            .strip_suffix(':')
            .ok_or_else(|| SourceAvailability::Error("slot missing colon delimiter".into()))?;
        slot_str
            .parse::<u64>()
            .map_err(|_| SourceAvailability::Error("invalid slot number in socket line".into()))?;

        let (local_ip, local_port) = match family {
            AddressFamily::Ipv4 => parse_v4_address_port(cols[1])?,
            AddressFamily::Ipv6 => parse_v6_address_port(cols[1])?,
        };

        let (remote_ip, remote_port) = match family {
            AddressFamily::Ipv4 => parse_v4_address_port(cols[2])?,
            AddressFamily::Ipv6 => parse_v6_address_port(cols[2])?,
        };

        if cols[3].len() != 2 {
            return Err(SourceAvailability::Error(
                "socket state must be exactly 2 hex digits".into(),
            ));
        }
        let st = u8::from_str_radix(cols[3], 16)
            .map_err(|_| SourceAvailability::Error("invalid hex in socket state".into()))?;
        let state = SocketState::from_linux_state(st);

        let inode = cols[9]
            .parse::<u64>()
            .map_err(|_| SourceAvailability::Error("invalid inode in socket line".into()))?;

        let mut record = SocketRecord::new(protocol, local_ip, local_port, state);
        if !(remote_ip.is_unspecified() && remote_port == 0) {
            record = record
                .with_remote(remote_ip, remote_port)
                .map_err(|err| SourceAvailability::Error(err.to_string()))?;
        }
        if inode > 0 {
            record = record.with_inode(inode);
        }

        if seen.contains(&record) {
            return Err(SourceAvailability::Error(
                "source contains duplicate socket record".into(),
            ));
        }

        if sockets.len() < MAX_NETWORK_SOCKETS {
            seen.insert(record.clone());
            sockets.push(record);
        } else {
            if last_overflow.as_ref() == Some(&record) {
                return Err(SourceAvailability::Error(
                    "source contains duplicate socket record".into(),
                ));
            }
            last_overflow = Some(record);
            dropped_sockets = dropped_sockets.saturating_add(1);
        }
    }

    Ok(ParsedSockets::new(sockets, dropped_sockets))
}

/// Parses IPv4 socket table content from Linux `/proc/net/tcp` or `/proc/net/udp`.
pub fn parse_v4_sockets(
    text: &str,
    protocol: SocketProtocol,
) -> Result<ParsedSockets, SourceAvailability> {
    parse_sockets_internal(text, protocol, AddressFamily::Ipv4)
}

/// Parses IPv6 socket table content from Linux `/proc/net/tcp6` or `/proc/net/udp6`.
pub fn parse_v6_sockets(
    text: &str,
    protocol: SocketProtocol,
) -> Result<ParsedSockets, SourceAvailability> {
    parse_sockets_internal(text, protocol, AddressFamily::Ipv6)
}

/// Collects TCP and UDP socket endpoint observations from the given procfs root.
#[must_use]
pub fn collect_sockets(proc_root: &Path, mode: SnapshotMode) -> SocketReport {
    collect_sockets_at(proc_root, mode, 0, Instant::now())
}

pub(crate) fn collect_sockets_at(
    proc_root: &Path,
    mode: SnapshotMode,
    monotonic_ns: u64,
    began: Instant,
) -> SocketReport {
    let mut sockets = Vec::new();
    let mut dropped_sockets = 0u64;

    // 1. TCP IPv4
    let mut tcp_v4_parsed: Option<ParsedSockets> = None;
    let tcp_v4_status = match read::source(proc_root, "net/tcp", mode) {
        Ok(text) => match parse_v4_sockets(&text, SocketProtocol::Tcp) {
            Ok(parsed) => {
                tcp_v4_parsed = Some(parsed);
                SourceAvailability::Available
            }
            Err(status) => status,
        },
        Err(state) => metric_state_to_availability(state),
    };

    // 2. TCP IPv6
    let mut tcp_v6_parsed: Option<ParsedSockets> = None;
    let tcp_v6_status = match read::source(proc_root, "net/tcp6", mode) {
        Ok(text) => match parse_v6_sockets(&text, SocketProtocol::Tcp) {
            Ok(parsed) => {
                tcp_v6_parsed = Some(parsed);
                SourceAvailability::Available
            }
            Err(status) => status,
        },
        Err(state) => metric_state_to_availability(state),
    };

    // 3. UDP IPv4
    let mut udp_v4_parsed: Option<ParsedSockets> = None;
    let udp_v4_status = match read::source(proc_root, "net/udp", mode) {
        Ok(text) => match parse_v4_sockets(&text, SocketProtocol::Udp) {
            Ok(parsed) => {
                udp_v4_parsed = Some(parsed);
                SourceAvailability::Available
            }
            Err(status) => status,
        },
        Err(state) => metric_state_to_availability(state),
    };

    // 4. UDP IPv6
    let mut udp_v6_parsed: Option<ParsedSockets> = None;
    let udp_v6_status = match read::source(proc_root, "net/udp6", mode) {
        Ok(text) => match parse_v6_sockets(&text, SocketProtocol::Udp) {
            Ok(parsed) => {
                udp_v6_parsed = Some(parsed);
                SourceAvailability::Available
            }
            Err(status) => status,
        },
        Err(state) => metric_state_to_availability(state),
    };

    for parsed in [tcp_v4_parsed, tcp_v6_parsed, udp_v4_parsed, udp_v6_parsed]
        .into_iter()
        .flatten()
    {
        dropped_sockets = dropped_sockets.saturating_add(parsed.dropped_sockets);
        for record in parsed.sockets {
            if sockets.len() < MAX_NETWORK_SOCKETS {
                sockets.push(record);
            } else {
                dropped_sockets = dropped_sockets.saturating_add(1);
            }
        }
    }

    SocketReport {
        tcp_v4_status,
        tcp_v6_status,
        udp_v4_status,
        udp_v6_status,
        sockets,
        dropped_sockets,
        collection_duration_ns: crate::duration_ns(began.elapsed()),
        ..SocketReport::new(mode, monotonic_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn parse_v4_tcp_sockets_listening_and_connected() {
        let fixture = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 00000000:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0\n\
   1: 0100007F:1F90 0200000A:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 67890 1 0000000000000000 100 0 0 10 0\n\
";
        let parsed = parse_v4_sockets(fixture, SocketProtocol::Tcp).expect("valid tcp table");
        assert_eq!(parsed.sockets.len(), 2);
        assert_eq!(parsed.dropped_sockets, 0);

        // Row 0: 0.0.0.0:80 LISTEN, inode 12345
        let s0 = &parsed.sockets[0];
        assert_eq!(s0.protocol(), SocketProtocol::Tcp);
        assert_eq!(s0.family(), AddressFamily::Ipv4);
        assert_eq!(s0.local_address(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        assert_eq!(s0.local_port(), 80);
        assert!(s0.remote_address().is_none());
        assert!(s0.remote_port().is_none());
        assert_eq!(s0.state(), SocketState::Listen);
        assert_eq!(s0.inode(), Some(12345));

        // Row 1: 127.0.0.1:8080 -> 10.0.0.2:443 ESTABLISHED, inode 67890
        let s1 = &parsed.sockets[1];
        assert_eq!(s1.protocol(), SocketProtocol::Tcp);
        assert_eq!(s1.family(), AddressFamily::Ipv4);
        assert_eq!(s1.local_address(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(s1.local_port(), 8080);
        assert_eq!(
            s1.remote_address(),
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)))
        );
        assert_eq!(s1.remote_port(), Some(443));
        assert_eq!(s1.state(), SocketState::Established);
        assert_eq!(s1.inode(), Some(67890));
    }

    #[test]
    fn parse_v6_tcp_sockets_loopback_and_established() {
        let fixture = "\
  sl  local_address                         rem_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 00000000000000000000000000000000:0016 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 23456 1 0000000000000000 100 0 0 10 0\n\
   1: 00000000000000000000000001000000:1F90 00000000000000000000000001000000:0050 01 00000000:00000000 00:00000000 00000000  1000        0 34567 1 0000000000000000 100 0 0 10 0\n\
";
        let parsed = parse_v6_sockets(fixture, SocketProtocol::Tcp).expect("valid tcp6 table");
        assert_eq!(parsed.sockets.len(), 2);

        // Row 0: [::]:22 LISTEN
        let s0 = &parsed.sockets[0];
        assert_eq!(s0.protocol(), SocketProtocol::Tcp);
        assert_eq!(s0.family(), AddressFamily::Ipv6);
        assert_eq!(s0.local_address(), IpAddr::V6(Ipv6Addr::UNSPECIFIED));
        assert_eq!(s0.local_port(), 22);
        assert!(s0.remote_address().is_none());
        assert_eq!(s0.state(), SocketState::Listen);
        assert_eq!(s0.inode(), Some(23456));

        // Row 1: [::1]:8080 -> [::1]:80 ESTABLISHED
        let s1 = &parsed.sockets[1];
        assert_eq!(s1.protocol(), SocketProtocol::Tcp);
        assert_eq!(s1.family(), AddressFamily::Ipv6);
        assert_eq!(s1.local_address(), IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_eq!(s1.local_port(), 8080);
        assert_eq!(s1.remote_address(), Some(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert_eq!(s1.remote_port(), Some(80));
        assert_eq!(s1.state(), SocketState::Established);
        assert_eq!(s1.inode(), Some(34567));
    }

    #[test]
    fn parse_v4_udp_sockets() {
        let fixture = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode ref pointer drops\n\
   0: 00000000:0035 00000000:0000 07 00000000:00000000 00:00000000 00000000     0        0 45678 2 0000000000000000 0\n\
";
        let parsed = parse_v4_sockets(fixture, SocketProtocol::Udp).expect("valid udp table");
        assert_eq!(parsed.sockets.len(), 1);
        let s0 = &parsed.sockets[0];
        assert_eq!(s0.protocol(), SocketProtocol::Udp);
        assert_eq!(s0.family(), AddressFamily::Ipv4);
        assert_eq!(s0.local_address(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        assert_eq!(s0.local_port(), 53);
        assert!(s0.remote_address().is_none());
        assert_eq!(s0.state(), SocketState::Closed);
        assert_eq!(s0.inode(), Some(45678));
    }

    #[test]
    fn empty_table_returns_zero_sockets() {
        let empty_with_header = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n";
        let parsed = parse_v4_sockets(empty_with_header, SocketProtocol::Tcp).expect("empty table");
        assert_eq!(parsed.sockets.len(), 0);
        assert_eq!(parsed.dropped_sockets, 0);

        let pure_empty = "";
        let parsed2 = parse_v4_sockets(pure_empty, SocketProtocol::Udp).expect("empty string");
        assert_eq!(parsed2.sockets.len(), 0);
    }

    #[test]
    fn malformed_headers_and_inputs_rejected() {
        // Bad header
        let bad_header = "sl not_local not_remote st\n";
        assert!(parse_v4_sockets(bad_header, SocketProtocol::Tcp).is_err());

        // Control characters
        let ctrl = "sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n\x07 0: 00000000:0050 00000000:0000 0A\n";
        assert!(parse_v4_sockets(ctrl, SocketProtocol::Tcp).is_err());

        // Malformed line (too few fields)
        let short_line = "sl local_address rem_address st\n0: 00000000:0050 00000000:0000 0A\n";
        assert!(parse_v4_sockets(short_line, SocketProtocol::Tcp).is_err());

        // Invalid hex
        let bad_hex = "sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n0: ZZZZZZZZ:0050 00000000:0000 0A 0 0 0 0 0 100\n";
        assert!(parse_v4_sockets(bad_hex, SocketProtocol::Tcp).is_err());
    }

    #[test]
    fn duplicate_socket_records_rejected() {
        let fixture = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 00000000:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0\n\
   1: 00000000:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0\n\
";
        let err = parse_v4_sockets(fixture, SocketProtocol::Tcp).unwrap_err();
        assert!(matches!(err, SourceAvailability::Error(_)));
    }

    #[test]
    fn consolidated_status_priorities() {
        let mut report = SocketReport::new(SnapshotMode::Live, 100);
        assert_eq!(
            report.consolidated_status(),
            SourceAvailability::Disabled("source not collected".into())
        );

        report.tcp_v4_status = SourceAvailability::Unsupported("no ipv4 tcp".into());
        report.tcp_v6_status = SourceAvailability::TemporarilyUnavailable("busy".into());
        assert_eq!(
            report.consolidated_status(),
            SourceAvailability::TemporarilyUnavailable("busy".into())
        );

        report.udp_v4_status = SourceAvailability::PermissionDenied("denied".into());
        assert_eq!(
            report.consolidated_status(),
            SourceAvailability::PermissionDenied("denied".into())
        );

        report.udp_v6_status = SourceAvailability::Available;
        assert_eq!(report.consolidated_status(), SourceAvailability::Available);
    }

    #[test]
    fn socket_report_populate_into_network_report() {
        let mut report = SocketReport::new(SnapshotMode::Fixture, 500);
        report.tcp_v4_status = SourceAvailability::Available;
        report.tcp_v6_status = SourceAvailability::Available;
        report.udp_v4_status = SourceAvailability::Available;
        report.udp_v6_status = SourceAvailability::Available;
        report.dropped_sockets = 3;
        report.sockets.push(
            SocketRecord::new(
                SocketProtocol::Tcp,
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                8080,
                SocketState::Listen,
            )
            .with_inode(999),
        );

        let mut net_report = NetworkReport::new(500, 1000, SnapshotMode::Fixture);
        report.populate_into(&mut net_report).unwrap();

        assert_eq!(net_report.sockets_status(), &SourceAvailability::Available);
        assert_eq!(net_report.dropped_sockets(), 3);
        assert_eq!(net_report.sockets().len(), 1);
        assert_eq!(net_report.sockets()[0].local_port(), 8080);
    }

    #[test]
    fn socket_report_populate_mismatch_rejected() {
        let report = SocketReport::new(SnapshotMode::Fixture, 500);
        let mut net_report = NetworkReport::new(500, 1000, SnapshotMode::Live);
        assert_eq!(
            report.populate_into(&mut net_report),
            Err(NetworkError::ReportContextMismatch)
        );
    }

    #[test]
    fn parse_sockets_bounded_retention_and_overflow() {
        use std::fmt::Write;
        let mut text = String::from("  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n");
        // Generate MAX_NETWORK_SOCKETS + 50 distinct socket lines
        for i in 0..(MAX_NETWORK_SOCKETS + 50) {
            let port = (i % 60000) as u16 + 1024;
            let ip_last_octet = (i / 60000) as u8 + 1;
            let _ = writeln!(
                text,
                "   {i}: {ip_last_octet:02X}00007F:{port:04X} 00000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 {} 1 0000000000000000 100 0 0 10 0",
                i + 1
            );
        }

        let parsed = parse_v4_sockets(&text, SocketProtocol::Tcp).expect("valid large table");
        assert_eq!(parsed.sockets.len(), MAX_NETWORK_SOCKETS);
        assert_eq!(parsed.dropped_sockets, 50);
    }

    #[test]
    fn parse_sockets_overflow_with_duplicate() {
        use std::fmt::Write;
        let mut text = String::from("  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n");
        for i in 0..MAX_NETWORK_SOCKETS {
            let port = (i % 60000) as u16 + 1024;
            let _ = writeln!(
                text,
                "   {i}: 0100007F:{port:04X} 00000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 {} 1 0000000000000000 100 0 0 10 0",
                i + 1
            );
        }
        // Add two identical overflow records
        let dup = "   9999: 0200007F:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 99999 1 0000000000000000 100 0 0 10 0\n";
        text.push_str(dup);
        text.push_str(dup);

        let err = parse_v4_sockets(&text, SocketProtocol::Tcp).unwrap_err();
        assert!(matches!(err, SourceAvailability::Error(_)));
    }

    #[test]
    fn parse_sockets_overflow_with_malformed_tail() {
        use std::fmt::Write;
        let mut text = String::from("  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n");
        for i in 0..MAX_NETWORK_SOCKETS {
            let port = (i % 60000) as u16 + 1024;
            let _ = writeln!(
                text,
                "   {i}: 0100007F:{port:04X} 00000000:0000 0A 00000000:00000000 00:00000000 00000000 0 0 {} 1 0000000000000000 100 0 0 10 0",
                i + 1
            );
        }
        text.push_str("   malformed line lacking required columns\n");

        let err = parse_v4_sockets(&text, SocketProtocol::Tcp).unwrap_err();
        assert!(matches!(err, SourceAvailability::Error(_)));
    }

    #[test]
    fn collect_sockets_on_empty_or_nonexistent_dir() {
        let temp = std::env::temp_dir().join(format!("lidb_sock_test_{}", std::process::id()));
        let report = collect_sockets(&temp, SnapshotMode::Fixture);
        assert_eq!(report.schema_version, "0.1");
        assert_eq!(report.mode, SnapshotMode::Fixture);
        assert_eq!(report.sockets.len(), 0);
        assert_eq!(report.dropped_sockets, 0);
    }
}
