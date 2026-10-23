//! Bounded, read-only IPv4 and IPv6 route collection from Linux procfs.
//!
//! Parses `/proc/net/route` (IPv4) and `/proc/net/ipv6_route` (IPv6) into
//! structured, validated [`RouteRecord`] entries. Enforces [`MAX_NETWORK_ROUTES`]
//! bounds, rejects ASCII control characters, and differentiates empty tables from
//! failed sources.

use crate::read;
use lidb_core::network::{
    NetworkError, NetworkReport, RouteRecord, SourceAvailability, MAX_INTERFACE_NAME_CHARS,
    MAX_NETWORK_ROUTES,
};
use lidb_core::{MetricState, MetricValue, SnapshotMode};
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::time::Instant;

/// Pre-stable schema for the route-only report, separate from snapshot JSON.
pub const ROUTE_REPORT_SCHEMA_VERSION: &str = "0.1";

/// Outcome of reading host routing tables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RouteReport {
    /// Route-report schema version.
    pub schema_version: &'static str,
    /// Origin of the batch timestamp; comparable only within one collector lifetime.
    pub clock_source: &'static str,
    /// Batch start in nanoseconds since construction of the owning collector.
    pub monotonic_ns: u64,
    /// Duration of the sequential IPv4/IPv6 reads, in nanoseconds.
    pub collection_duration_ns: u64,
    /// Live process procfs view or explicitly selected fixture input.
    pub mode: SnapshotMode,
    /// IPv4 source identifier, without a private fixture path.
    pub v4_source: &'static str,
    /// IPv6 source identifier, without a private fixture path.
    pub v6_source: &'static str,
    /// Availability status of the IPv4 routing table (`/proc/net/route`).
    pub v4_status: SourceAvailability,
    /// Availability status of the IPv6 routing table (`/proc/net/ipv6_route`).
    pub v6_status: SourceAvailability,
    /// Parsed routing records, bounded by [`MAX_NETWORK_ROUTES`].
    pub routes: Vec<RouteRecord>,
    /// Number of valid route entries dropped because [`MAX_NETWORK_ROUTES`] capacity was reached.
    pub dropped_routes: u64,
}

impl RouteReport {
    fn new(mode: SnapshotMode, monotonic_ns: u64) -> Self {
        Self {
            schema_version: ROUTE_REPORT_SCHEMA_VERSION,
            clock_source: "collector_monotonic",
            monotonic_ns,
            collection_duration_ns: 0,
            mode,
            v4_source: "procfs/net/route",
            v6_source: "procfs/net/ipv6_route",
            v4_status: SourceAvailability::Disabled("source not collected".into()),
            v6_status: SourceAvailability::Disabled("source not collected".into()),
            routes: Vec::new(),
            dropped_routes: 0,
        }
    }

    /// Evaluates the consolidated route availability status.
    ///
    /// If at least one table succeeded, returns [`SourceAvailability::Available`].
    /// Otherwise, reports the underlying failure (e.g. `PermissionDenied` or `Unsupported`).
    #[must_use]
    pub fn consolidated_status(&self) -> SourceAvailability {
        match (&self.v4_status, &self.v6_status) {
            (SourceAvailability::Available, _) | (_, SourceAvailability::Available) => {
                SourceAvailability::Available
            }
            (SourceAvailability::PermissionDenied(r), _)
            | (_, SourceAvailability::PermissionDenied(r)) => {
                SourceAvailability::PermissionDenied(r.clone())
            }
            (SourceAvailability::Error(r), _) | (_, SourceAvailability::Error(r)) => {
                SourceAvailability::Error(r.clone())
            }
            (SourceAvailability::TemporarilyUnavailable(r), _)
            | (_, SourceAvailability::TemporarilyUnavailable(r)) => {
                SourceAvailability::TemporarilyUnavailable(r.clone())
            }
            (SourceAvailability::Disabled(r), _) | (_, SourceAvailability::Disabled(r)) => {
                SourceAvailability::Disabled(r.clone())
            }
            (SourceAvailability::Unsupported(r), _) => SourceAvailability::Unsupported(r.clone()),
        }
    }

    /// Appends routes and omission counts without relabeling a known context.
    ///
    /// Callers must already know the reports share one collector clock domain.
    /// Matching timestamps alone cannot prove that. The destination must cover
    /// this exact batch and have no asserted namespace (not captured here).
    /// Unequal family states cannot be collapsed losslessly into the target's
    /// single routes status; retain this report instead. Rejection is atomic.
    pub fn populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError> {
        if report.mode() != self.mode
            || report.monotonic_ns() != self.monotonic_ns
            || report.collection_duration_ns() < self.collection_duration_ns
            || report.clock_source() != self.clock_source
            || report.namespace().is_some()
        {
            return Err(NetworkError::ReportContextMismatch);
        }
        if self.v4_status != self.v6_status {
            return Err(NetworkError::RouteSourceStatusMismatch);
        }
        report.set_routes_status(self.consolidated_status());
        report.record_dropped_routes(self.dropped_routes);
        for route in &self.routes {
            let _ = report.push_route(route.clone());
        }
        Ok(())
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

/// Parses a 32-hex-character string into an [`Ipv6Addr`].
fn parse_ipv6_hex(hex: &str) -> Result<Ipv6Addr, &'static str> {
    if hex.len() != 32 {
        return Err("ipv6 address must be exactly 32 hex characters");
    }
    let mut octets = [0u8; 16];
    for (i, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
        let chunk_str = std::str::from_utf8(chunk).map_err(|_| "invalid utf8")?;
        octets[i] = u8::from_str_radix(chunk_str, 16).map_err(|_| "invalid hex in ipv6 address")?;
    }
    Ok(Ipv6Addr::from(octets))
}

/// Parses IPv4 routing table content from Linux `/proc/net/route`.
///
/// Columns expected:
/// `Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT`
pub fn parse_v4_routes(text: &str) -> Result<Vec<RouteRecord>, SourceAvailability> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first_line) = lines.next() else {
        // Empty file is a legitimate empty table
        return Ok(Vec::new());
    };

    let first_is_header = first_line.starts_with("Iface") || first_line.starts_with("iface");
    let data_lines: Vec<&str> = if first_is_header {
        if !first_line.contains("Destination") || !first_line.contains("Mask") {
            return Err(SourceAvailability::Error(
                "route header is malformed".into(),
            ));
        }
        lines.collect()
    } else {
        std::iter::once(first_line).chain(lines).collect()
    };

    let mut routes = Vec::new();
    for line in data_lines {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 8 {
            return Err(SourceAvailability::Error(
                "route line lacks required fields".into(),
            ));
        }

        let iface = cols[0];
        if iface.is_empty()
            || iface.len() > MAX_INTERFACE_NAME_CHARS
            || iface.chars().any(char::is_control)
        {
            return Err(SourceAvailability::Error(
                "invalid interface name in route".into(),
            ));
        }

        let dest_hex = cols[1];
        if dest_hex.len() != 8 {
            return Err(SourceAvailability::Error(
                "destination must be 8 hex digits".into(),
            ));
        }
        let dest_u32 = u32::from_str_radix(dest_hex, 16)
            .map_err(|_| SourceAvailability::Error("invalid hex in destination address".into()))?;
        let dest_ip = IpAddr::V4(Ipv4Addr::from(dest_u32.to_le_bytes()));

        let gw_hex = cols[2];
        if gw_hex.len() != 8 {
            return Err(SourceAvailability::Error(
                "gateway must be 8 hex digits".into(),
            ));
        }
        let gw_u32 = u32::from_str_radix(gw_hex, 16)
            .map_err(|_| SourceAvailability::Error("invalid hex in gateway address".into()))?;
        let gateway = if gw_u32 != 0 {
            Some(IpAddr::V4(Ipv4Addr::from(gw_u32.to_le_bytes())))
        } else {
            None
        };

        let metric = cols[6]
            .parse::<u32>()
            .map_err(|_| SourceAvailability::Error("invalid metric in route".into()))?;

        let mask_hex = cols[7];
        if mask_hex.len() != 8 {
            return Err(SourceAvailability::Error(
                "mask must be 8 hex digits".into(),
            ));
        }
        let mask_u32 = u32::from_str_radix(mask_hex, 16)
            .map_err(|_| SourceAvailability::Error("invalid hex in route mask".into()))?;
        let mask_be = u32::from_be_bytes(mask_u32.to_le_bytes());
        let prefix_len = mask_be.leading_ones() as u8;

        if prefix_len > 32 {
            return Err(SourceAvailability::Error(
                "prefix length exceeds 32 bits".into(),
            ));
        }
        if (mask_be.checked_shl(u32::from(prefix_len)).unwrap_or(0)) != 0 {
            return Err(SourceAvailability::Error(
                "non-contiguous netmask in route".into(),
            ));
        }

        let mut record = RouteRecord::new(dest_ip, prefix_len, iface)
            .map_err(|err| SourceAvailability::Error(err.to_string()))?
            .with_metric(metric);

        if let Some(gw) = gateway {
            record = record
                .with_gateway(gw)
                .map_err(|err| SourceAvailability::Error(err.to_string()))?;
        }

        if routes.iter().any(|existing| existing == &record) {
            return Err(SourceAvailability::Error(
                "source contains duplicate route".into(),
            ));
        }
        routes.push(record);
    }

    Ok(routes)
}

/// Parses IPv6 routing table content from Linux `/proc/net/ipv6_route`.
///
/// Columns expected (space-separated, 10 fields):
/// `dest_ip dest_prefix src_ip src_prefix next_hop metric refcnt use flags iface`
pub fn parse_v6_routes(text: &str) -> Result<Vec<RouteRecord>, SourceAvailability> {
    let mut routes = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            return Err(SourceAvailability::Error(
                "ipv6 route line lacks required fields".into(),
            ));
        }

        let dest_ip = parse_ipv6_hex(cols[0])
            .map_err(|_| SourceAvailability::Error("invalid ipv6 destination hex".into()))?;

        let prefix_len = u8::from_str_radix(cols[1], 16)
            .map_err(|_| SourceAvailability::Error("invalid ipv6 prefix hex".into()))?;
        if prefix_len > 128 {
            return Err(SourceAvailability::Error(
                "ipv6 prefix length exceeds 128 bits".into(),
            ));
        }

        let next_hop = parse_ipv6_hex(cols[4])
            .map_err(|_| SourceAvailability::Error("invalid ipv6 next hop hex".into()))?;
        let gateway = if next_hop.is_unspecified() {
            None
        } else {
            Some(IpAddr::V6(next_hop))
        };

        let metric = u32::from_str_radix(cols[5], 16)
            .map_err(|_| SourceAvailability::Error("invalid ipv6 metric hex".into()))?;

        let iface = cols[9];
        if iface.is_empty()
            || iface.len() > MAX_INTERFACE_NAME_CHARS
            || iface.chars().any(char::is_control)
        {
            return Err(SourceAvailability::Error(
                "invalid interface name in ipv6 route".into(),
            ));
        }

        let mut record = RouteRecord::new(IpAddr::V6(dest_ip), prefix_len, iface)
            .map_err(|err| SourceAvailability::Error(err.to_string()))?
            .with_metric(metric);

        if let Some(gw) = gateway {
            record = record
                .with_gateway(gw)
                .map_err(|err| SourceAvailability::Error(err.to_string()))?;
        }

        if routes.iter().any(|existing| existing == &record) {
            return Err(SourceAvailability::Error(
                "source contains duplicate route".into(),
            ));
        }
        routes.push(record);
    }

    Ok(routes)
}

/// Collects all IPv4 and IPv6 routing table entries from the specified procfs root.
///
/// Enforces [`MAX_NETWORK_ROUTES`] total capacity, counting any excess as dropped routes.
/// This standalone call starts its own one-call clock domain at zero; use
/// [`crate::LinuxCollector::routes`] for timestamps shared with snapshots.
#[must_use]
pub fn collect_routes(proc_root: &Path, mode: SnapshotMode) -> RouteReport {
    // Standalone calls have a single-call clock domain starting at zero.
    collect_routes_at(proc_root, mode, 0, Instant::now())
}

pub(crate) fn collect_routes_at(
    proc_root: &Path,
    mode: SnapshotMode,
    monotonic_ns: u64,
    began: Instant,
) -> RouteReport {
    let mut routes = Vec::new();
    let mut dropped_routes = 0u64;

    // IPv4 routes collection
    let v4_status = match read::source(proc_root, "net/route", mode) {
        Ok(text) => match parse_v4_routes(&text) {
            Ok(v4_records) => {
                for record in v4_records {
                    if routes.len() < MAX_NETWORK_ROUTES {
                        routes.push(record);
                    } else {
                        dropped_routes = dropped_routes.saturating_add(1);
                    }
                }
                SourceAvailability::Available
            }
            Err(status) => status,
        },
        Err(state) => metric_state_to_availability(state),
    };

    // IPv6 routes collection
    let v6_status = match read::source(proc_root, "net/ipv6_route", mode) {
        Ok(text) => match parse_v6_routes(&text) {
            Ok(v6_records) => {
                for record in v6_records {
                    if routes.len() < MAX_NETWORK_ROUTES {
                        routes.push(record);
                    } else {
                        dropped_routes = dropped_routes.saturating_add(1);
                    }
                }
                SourceAvailability::Available
            }
            Err(status) => status,
        },
        Err(state) => metric_state_to_availability(state),
    };

    RouteReport {
        v4_status,
        v6_status,
        routes,
        dropped_routes,
        collection_duration_ns: crate::duration_ns(began.elapsed()),
        ..RouteReport::new(mode, monotonic_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::network::AddressFamily;

    #[test]
    fn parse_v4_routes_default_and_subnet() {
        let fixture = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n\
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n\
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0\n\
";
        let routes = parse_v4_routes(fixture).expect("valid routes");
        assert_eq!(routes.len(), 2);

        // Default route
        let default_rt = &routes[0];
        assert_eq!(default_rt.family(), AddressFamily::Ipv4);
        assert_eq!(
            default_rt.destination(),
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))
        );
        assert_eq!(default_rt.prefix_len(), 0);
        assert_eq!(
            default_rt.gateway(),
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
        );
        assert_eq!(default_rt.interface(), "eth0");
        assert_eq!(default_rt.metric(), Some(100));

        // Subnet route 192.168.1.0/24
        let subnet_rt = &routes[1];
        assert_eq!(subnet_rt.family(), AddressFamily::Ipv4);
        assert_eq!(
            subnet_rt.destination(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0))
        );
        assert_eq!(subnet_rt.prefix_len(), 24);
        assert_eq!(subnet_rt.gateway(), None);
        assert_eq!(subnet_rt.interface(), "eth0");
        assert_eq!(subnet_rt.metric(), Some(100));
    }

    #[test]
    fn parse_v4_rejects_non_contiguous_mask() {
        // Mask 0x00FEFFFF in le_bytes is [255, 255, 254, 0], contiguous.
        // But 0x0055FFFF in le_bytes is [255, 255, 85, 0], non-contiguous.
        let fixture = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n\
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t0055FFFF\t0\t0\t0\n\
";
        assert!(matches!(
            parse_v4_routes(fixture),
            Err(SourceAvailability::Error(_))
        ));
    }

    #[test]
    fn parse_v4_rejects_duplicates() {
        let fixture = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n\
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0\n\
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0\n\
";
        let res = parse_v4_routes(fixture);
        assert!(matches!(res, Err(SourceAvailability::Error(msg)) if msg.contains("duplicate")));
    }

    #[test]
    fn parse_v4_empty_table_is_available() {
        let fixture =
            "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n";
        let routes = parse_v4_routes(fixture).expect("empty table succeeds");
        assert!(routes.is_empty());

        let empty_str = "";
        let routes_empty = parse_v4_routes(empty_str).expect("empty string succeeds");
        assert!(routes_empty.is_empty());
    }

    #[test]
    fn parse_v6_routes_default_and_subnet() {
        let fixture = "\
00000000000000000000000000000000 00 00000000000000000000000000000000 00 fe80000000000000021122fffe334455 00000200 00000000 00000000 00000003 eth0\n\
20010db8000100000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000400 00000000 00000000 00000001 eth0\n\
";
        let routes = parse_v6_routes(fixture).expect("valid ipv6 routes");
        assert_eq!(routes.len(), 2);

        // IPv6 default route
        let default_rt = &routes[0];
        assert_eq!(default_rt.family(), AddressFamily::Ipv6);
        assert_eq!(
            default_rt.destination(),
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0))
        );
        assert_eq!(default_rt.prefix_len(), 0);
        assert_eq!(
            default_rt.gateway(),
            Some(IpAddr::V6(Ipv6Addr::new(
                0xfe80, 0, 0, 0, 0x0211, 0x22ff, 0xfe33, 0x4455
            )))
        );
        assert_eq!(default_rt.metric(), Some(512));
        assert_eq!(default_rt.interface(), "eth0");

        // IPv6 subnet route 2001:db8:1::/64
        let subnet_rt = &routes[1];
        assert_eq!(subnet_rt.family(), AddressFamily::Ipv6);
        assert_eq!(
            subnet_rt.destination(),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0x0001, 0, 0, 0, 0, 0))
        );
        assert_eq!(subnet_rt.prefix_len(), 64);
        assert_eq!(subnet_rt.gateway(), None);
        assert_eq!(subnet_rt.metric(), Some(1024));
        assert_eq!(subnet_rt.interface(), "eth0");
    }

    #[test]
    fn parse_v6_rejects_invalid_hex_and_prefix() {
        let bad_prefix = "00000000000000000000000000000000 81 00000000000000000000000000000000 00 00000000000000000000000000000000 00000200 0 0 0 eth0\n";
        assert!(matches!(
            parse_v6_routes(bad_prefix),
            Err(SourceAvailability::Error(_))
        ));

        let bad_hex = "0000000000000000000000000000000Z 00 00000000000000000000000000000000 00 00000000000000000000000000000000 00000200 0 0 0 eth0\n";
        assert!(matches!(
            parse_v6_routes(bad_hex),
            Err(SourceAvailability::Error(_))
        ));
    }

    #[test]
    fn parse_v6_rejects_duplicates() {
        let fixture = "\
20010db8000100000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000400 00000000 00000000 00000001 eth0\n\
20010db8000100000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000400 00000000 00000000 00000001 eth0\n\
";
        let res = parse_v6_routes(fixture);
        assert!(matches!(res, Err(SourceAvailability::Error(msg)) if msg.contains("duplicate")));
    }

    #[test]
    fn route_report_consolidated_status_logic() {
        let avail = RouteReport {
            v4_status: SourceAvailability::Available,
            v6_status: SourceAvailability::unsupported("no ipv6 on kernel").unwrap(),
            routes: Vec::new(),
            dropped_routes: 0,
            ..RouteReport::new(SnapshotMode::Fixture, 0)
        };
        assert_eq!(avail.consolidated_status(), SourceAvailability::Available);

        let denied = RouteReport {
            v4_status: SourceAvailability::permission_denied("denied v4").unwrap(),
            v6_status: SourceAvailability::unsupported("unsupported v6").unwrap(),
            routes: Vec::new(),
            dropped_routes: 0,
            ..RouteReport::new(SnapshotMode::Fixture, 0)
        };
        assert!(matches!(
            denied.consolidated_status(),
            SourceAvailability::PermissionDenied(_)
        ));
    }

    #[test]
    fn route_report_populate_into_network_report() {
        let route = RouteRecord::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8, "eth1")
            .unwrap()
            .with_metric(50);

        let report_src = RouteReport {
            v4_status: SourceAvailability::Available,
            v6_status: SourceAvailability::Available,
            routes: vec![route],
            dropped_routes: 0,
            ..RouteReport::new(SnapshotMode::Live, 100)
        };

        let mut net_report = NetworkReport::new(100, 10, SnapshotMode::Live);
        report_src.populate_into(&mut net_report).unwrap();

        assert_eq!(net_report.routes_status(), &SourceAvailability::Available);
        assert_eq!(net_report.routes().len(), 1);
        assert_eq!(net_report.routes()[0].interface(), "eth1");
    }

    #[test]
    fn collect_routes_on_empty_dir_reports_availability() {
        let temp_dir =
            std::env::temp_dir().join(format!("lidb_test_routes_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let report = collect_routes(&temp_dir, SnapshotMode::Fixture);
        // On non-Linux or without the files, it truthfully reports unsupported
        assert!(matches!(
            report.consolidated_status(),
            SourceAvailability::Unsupported(_)
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn population_preserves_upstream_and_destination_omissions() {
        let route = RouteRecord::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 32, "lo").unwrap();
        let source = RouteReport {
            v4_status: SourceAvailability::Available,
            v6_status: SourceAvailability::Available,
            routes: vec![route.clone()],
            dropped_routes: 17,
            ..RouteReport::new(SnapshotMode::Fixture, 100)
        };
        let mut target = NetworkReport::new(100, 0, SnapshotMode::Fixture);
        for _ in 0..MAX_NETWORK_ROUTES {
            target.push_route(route.clone()).unwrap();
        }
        source.populate_into(&mut target).unwrap();
        assert_eq!(target.routes().len(), MAX_NETWORK_ROUTES);
        assert_eq!(target.dropped_routes(), 18);
        assert_eq!(target.dropped_records(), 18);
        assert!(!target.interfaces_status().is_available());
        assert!(!target.sockets_status().is_available());
    }

    #[test]
    fn population_rejects_context_and_source_state_loss_before_mutation() {
        let source = RouteReport {
            v4_status: SourceAvailability::Available,
            v6_status: SourceAvailability::Available,
            collection_duration_ns: 10,
            dropped_routes: 9,
            ..RouteReport::new(SnapshotMode::Fixture, 100)
        };
        let namespace = lidb_core::network::NetworkNamespace::new("target", Some(10)).unwrap();
        for mut target in [
            NetworkReport::new(100, 10, SnapshotMode::Live),
            NetworkReport::new(101, 10, SnapshotMode::Fixture),
            NetworkReport::new(100, 9, SnapshotMode::Fixture),
            NetworkReport::new(100, 10, SnapshotMode::Fixture).with_namespace(namespace),
        ] {
            assert_eq!(
                source.populate_into(&mut target),
                Err(NetworkError::ReportContextMismatch)
            );
            assert_eq!(target.routes_status().as_str(), "disabled");
            assert_eq!(target.dropped_routes(), 0);
            assert!(target.routes().is_empty());
        }
        let mut target = NetworkReport::new(100, 10, SnapshotMode::Fixture);
        let mut partial = source;
        partial.v6_status = SourceAvailability::PermissionDenied("denied IPv6".into());
        assert_eq!(
            partial.populate_into(&mut target),
            Err(NetworkError::RouteSourceStatusMismatch)
        );
        assert_eq!(target.routes_status().as_str(), "disabled");
        assert_eq!(target.dropped_routes(), 0);
    }

    #[test]
    fn collection_window_includes_delay_after_the_shared_start() {
        let gap = std::time::Duration::from_secs(2);
        let began = Instant::now() - gap;
        let source = collect_routes_at(
            Path::new("/nonexistent-lidb-clock-fixture"),
            SnapshotMode::Fixture,
            123,
            began,
        );
        assert!(source.collection_duration_ns >= crate::duration_ns(gap));
        let mut target =
            NetworkReport::new(123, crate::duration_ns(gap) - 1, SnapshotMode::Fixture);
        assert_eq!(
            source.populate_into(&mut target),
            Err(NetworkError::ReportContextMismatch)
        );
        assert_eq!(target.routes_status().as_str(), "disabled");
        assert_eq!(target.dropped_routes(), 0);
    }
}
