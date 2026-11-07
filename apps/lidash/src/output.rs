use lidb_core::{
    network::NetworkReport, MetricObservation, MetricValue, Snapshot, SnapshotMode,
    SNAPSHOT_SCHEMA_VERSION,
};
use serde::Serialize;
use std::{collections::BTreeMap, fmt::Write};

pub fn mode_name(mode: SnapshotMode) -> &'static str {
    match mode {
        SnapshotMode::Live => "live",
        SnapshotMode::Fixture => "fixture",
    }
}

pub fn value_text(metric: &MetricObservation<MetricValue>) -> String {
    match metric.state().available_value() {
        Some(MetricValue::Integer(value)) => value.to_string(),
        Some(MetricValue::Number(value)) => format!("{value:.2}"),
        None => format!(
            "{}: {}",
            metric.state().name(),
            metric.state().reason().unwrap_or("no reason reported")
        ),
    }
}

pub fn snapshot_text(snapshot: &Snapshot) -> String {
    let mut out = format!(
        "LiDB {} snapshot | collector_monotonic={}ns | collection={}ns | dropped_metrics={}\n",
        mode_name(snapshot.mode()),
        snapshot.monotonic_ns(),
        snapshot.collection_duration_ns(),
        snapshot.dropped_metrics()
    );
    for metric in snapshot.metrics() {
        let _ = writeln!(
            out,
            "{} = {} {} [{} @ {}ns]",
            metric.name(),
            value_text(metric),
            metric.unit().as_str(),
            metric.source(),
            metric.monotonic_ns()
        );
    }
    out
}

pub fn routes_text(report: &lidb_collect::RouteReport) -> String {
    let mut out = format!(
        "LiDB {} routes | collector_monotonic={}ns | collection={}ns | v4: {} | v6: {} | dropped={}\n",
        mode_name(report.mode),
        report.monotonic_ns,
        report.collection_duration_ns,
        report.v4_status.as_str(),
        report.v6_status.as_str(),
        report.dropped_routes
    );
    for (source, status) in [
        (report.v4_source, &report.v4_status),
        (report.v6_source, &report.v6_status),
    ] {
        if let Some(reason) = status.reason() {
            let _ = writeln!(out, "{source}: {} — {reason}", status.as_str());
        }
    }
    if !report.routes.is_empty() {
        for route in &report.routes {
            let gw_text = match route.gateway() {
                Some(gw) => format!(" via {gw}"),
                None => String::new(),
            };
            let metric_text = match route.metric() {
                Some(metric) => format!(" metric {metric}"),
                None => String::new(),
            };
            let src_text = match (route.source_prefix(), route.source_prefix_len()) {
                (Some(src), Some(len)) => format!(" from {src}/{len}"),
                _ => String::new(),
            };
            let _ = writeln!(
                out,
                "{}/{}{} dev {}{}{}",
                route.destination(),
                route.prefix_len(),
                src_text,
                route.interface(),
                gw_text,
                metric_text
            );
        }
    }
    for (family, label, status) in [
        (
            lidb_core::network::AddressFamily::Ipv4,
            "IPv4",
            &report.v4_status,
        ),
        (
            lidb_core::network::AddressFamily::Ipv6,
            "IPv6",
            &report.v6_status,
        ),
    ] {
        let has_records = report.routes.iter().any(|route| route.family() == family);
        if !status.is_available() {
            let _ = writeln!(out, "{label} routing table unavailable");
        } else if !has_records && report.dropped_routes == 0 {
            let _ = writeln!(out, "no {label} routing entries observed");
        } else if !has_records {
            let _ = writeln!(
                out,
                "no {label} routing entries retained; report has omissions"
            );
        }
    }
    out
}

pub fn interfaces_text(report: &lidb_collect::InterfaceReport) -> String {
    let ns_text = match &report.namespace {
        Some(ns) => match ns.inode() {
            Some(ino) => format!(" | ns: {}[{ino}]", ns.name()),
            None => format!(" | ns: {}", ns.name()),
        },
        None => String::new(),
    };
    let mut out = format!(
        "LiDB {} interfaces | collector_monotonic={}ns | collection={}ns | status: {}{} | dropped={}\n",
        mode_name(report.mode),
        report.monotonic_ns,
        report.collection_duration_ns,
        report.status.as_str(),
        ns_text,
        report.dropped_interfaces
    );
    if let Some(reason) = report.status.reason() {
        let _ = writeln!(
            out,
            "{}: {} — {reason}",
            report.source,
            report.status.as_str()
        );
    }
    if !report.interfaces.is_empty() {
        for iface in &report.interfaces {
            let idx_text = match iface.ifindex() {
                Some(idx) => format!("{idx}: "),
                None => String::new(),
            };
            let mtu_text = match iface.mtu() {
                Some(mtu) => format!(" mtu {mtu}"),
                None => String::new(),
            };
            let speed_text = match iface.speed_mbps() {
                Some(speed) => format!(" speed {speed}Mbps"),
                None => String::new(),
            };
            let mac_text = match iface.mac_address() {
                Some(mac) => format!(" link/ether {mac}"),
                None => String::new(),
            };
            let flags_text = if !iface.flags().is_empty() {
                format!(" <{}>", iface.flags().join(","))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "{}{}: state {}{}{}{}{}",
                idx_text,
                iface.name(),
                iface.oper_state().as_str(),
                flags_text,
                mtu_text,
                speed_text,
                mac_text
            );
        }
    } else if !report.status.is_available() {
        let _ = writeln!(out, "interfaces table unavailable");
    } else if report.dropped_interfaces == 0 {
        let _ = writeln!(out, "no network interfaces observed");
    } else {
        let _ = writeln!(out, "no network interfaces retained; report has omissions");
    }
    out
}

pub(crate) fn format_socket_endpoint(addr: std::net::IpAddr, port: u16) -> String {
    match addr {
        std::net::IpAddr::V4(v4) => format!("{v4}:{port}"),
        std::net::IpAddr::V6(v6) => format!("[{v6}]:{port}"),
    }
}

pub fn sockets_text(report: &lidb_collect::SocketReport) -> String {
    let mut out = format!(
        "LiDB {} sockets | collector_monotonic={}ns | collection={}ns | tcp4: {} | tcp6: {} | udp4: {} | udp6: {} | dropped={}\n",
        mode_name(report.mode),
        report.monotonic_ns,
        report.collection_duration_ns,
        report.tcp_v4_status.as_str(),
        report.tcp_v6_status.as_str(),
        report.udp_v4_status.as_str(),
        report.udp_v6_status.as_str(),
        report.dropped_sockets
    );
    for (source, status) in [
        (report.tcp_v4_source, &report.tcp_v4_status),
        (report.tcp_v6_source, &report.tcp_v6_status),
        (report.udp_v4_source, &report.udp_v4_status),
        (report.udp_v6_source, &report.udp_v6_status),
    ] {
        if let Some(reason) = status.reason() {
            let _ = writeln!(out, "{source}: {} — {reason}", status.as_str());
        }
    }
    if !report.sockets.is_empty() {
        for socket in &report.sockets {
            let remote_text = match (socket.remote_address(), socket.remote_port()) {
                (Some(addr), Some(port)) => format!(" -> {}", format_socket_endpoint(addr, port)),
                _ => String::new(),
            };
            let ino_text = match socket.inode() {
                Some(ino) => format!(" ino:{ino}"),
                None => String::new(),
            };
            let _ = writeln!(
                out,
                "{:<4} {}{:<16} state:{}{}",
                socket.protocol().as_str(),
                format_socket_endpoint(socket.local_address(), socket.local_port()),
                remote_text,
                socket.state().as_str(),
                ino_text
            );
        }
    } else if !report.consolidated_status().is_available() {
        let _ = writeln!(out, "sockets table unavailable");
    } else if report.dropped_sockets == 0 {
        let _ = writeln!(out, "no sockets observed");
    } else {
        let _ = writeln!(out, "no sockets retained; report has omissions");
    }
    out
}

pub fn network_text(report: &lidb_core::network::NetworkReport) -> String {
    let ns_text = match report.namespace() {
        Some(ns) => match ns.inode() {
            Some(ino) => format!(" | ns: {}[{ino}]", ns.name()),
            None => format!(" | ns: {}", ns.name()),
        },
        None => String::new(),
    };
    let mut out = format!(
        "LiDB {} network | collector_monotonic={}ns | collection={}ns | ifaces: {} | routes: {} | sockets: {}{} | dropped={}\n",
        mode_name(report.mode()),
        report.monotonic_ns(),
        report.collection_duration_ns(),
        report.interfaces_status().as_str(),
        report.routes_status().as_str(),
        report.sockets_status().as_str(),
        ns_text,
        report.dropped_records()
    );

    for (label, status) in [
        ("interfaces", report.interfaces_status()),
        ("routes", report.routes_status()),
        ("sockets", report.sockets_status()),
    ] {
        if let Some(reason) = status.reason() {
            let _ = writeln!(out, "{label}: {} — {reason}", status.as_str());
        }
    }

    out.push_str("\n--- Interfaces ---\n");
    if !report.interfaces().is_empty() {
        for iface in report.interfaces() {
            let idx_text = match iface.ifindex() {
                Some(idx) => format!("{idx}: "),
                None => String::new(),
            };
            let mtu_text = match iface.mtu() {
                Some(mtu) => format!(" mtu {mtu}"),
                None => String::new(),
            };
            let speed_text = match iface.speed_mbps() {
                Some(speed) => format!(" speed {speed}Mbps"),
                None => String::new(),
            };
            let mac_text = match iface.mac_address() {
                Some(mac) => format!(" link/ether {mac}"),
                None => String::new(),
            };
            let flags_text = if !iface.flags().is_empty() {
                format!(" <{}>", iface.flags().join(","))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "{}{}: state {}{}{}{}{}",
                idx_text,
                iface.name(),
                iface.oper_state().as_str(),
                flags_text,
                mtu_text,
                speed_text,
                mac_text
            );
        }
    } else if !report.interfaces_status().is_available() {
        let _ = writeln!(out, "interfaces table unavailable");
    } else if report.dropped_interfaces() == 0 {
        let _ = writeln!(out, "no network interfaces observed");
    } else {
        let _ = writeln!(out, "no network interfaces retained; report has omissions");
    }

    out.push_str("\n--- Routes ---\n");
    if !report.routes().is_empty() {
        for route in report.routes() {
            let gw_text = match route.gateway() {
                Some(gw) => format!(" via {gw}"),
                None => String::new(),
            };
            let metric_text = match route.metric() {
                Some(metric) => format!(" metric {metric}"),
                None => String::new(),
            };
            let src_text = match (route.source_prefix(), route.source_prefix_len()) {
                (Some(src), Some(len)) => format!(" from {src}/{len}"),
                _ => String::new(),
            };
            let _ = writeln!(
                out,
                "{}/{}{} dev {}{}{}",
                route.destination(),
                route.prefix_len(),
                src_text,
                route.interface(),
                gw_text,
                metric_text
            );
        }
    } else if !report.routes_status().is_available() {
        let _ = writeln!(out, "routing table unavailable");
    } else if report.dropped_routes() == 0 {
        let _ = writeln!(out, "no routing entries observed");
    } else {
        let _ = writeln!(out, "no routing entries retained; report has omissions");
    }

    out.push_str("\n--- Sockets ---\n");
    if !report.sockets().is_empty() {
        for socket in report.sockets() {
            let remote_text = match (socket.remote_address(), socket.remote_port()) {
                (Some(addr), Some(port)) => format!(" -> {}", format_socket_endpoint(addr, port)),
                _ => String::new(),
            };
            let ino_text = match socket.inode() {
                Some(ino) => format!(" ino:{ino}"),
                None => String::new(),
            };
            let _ = writeln!(
                out,
                "{:<4} {}{:<16} state:{}{}",
                socket.protocol().as_str(),
                format_socket_endpoint(socket.local_address(), socket.local_port()),
                remote_text,
                socket.state().as_str(),
                ino_text
            );
        }
    } else if !report.sockets_status().is_available() {
        let _ = writeln!(out, "sockets table unavailable");
    } else if report.dropped_sockets() == 0 {
        let _ = writeln!(out, "no sockets observed");
    } else {
        let _ = writeln!(out, "no sockets retained; report has omissions");
    }

    out
}

#[derive(Clone, Debug, Serialize)]
pub struct Failure<'a> {
    pub metric: &'a str,
    pub status: &'a str,
    pub reason: &'a str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceCapability<'a> {
    pub source: &'a str,
    pub status: &'a str,
    pub available_metrics: usize,
    pub unavailable_metrics: usize,
    pub failures: Vec<Failure<'a>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport<'a> {
    pub schema_version: &'static str,
    pub mode: &'static str,
    pub collection_status: &'static str,
    pub usable: bool,
    pub monotonic_ns: u64,
    pub collection_duration_ns: u64,
    pub dropped_metrics: u64,
    pub capabilities: Vec<SourceCapability<'a>>,
}

pub const EXPORT_SCHEMA_VERSION: &str = "0.1";
pub const EXPORT_TYPE: &str = "offline_diagnostic_evidence";
pub const EXPORT_CLOCK_SOURCE: &str = "collector_monotonic";
pub const EXPORT_PRIVACY_DECLARATION: &str =
    "LiDB offline diagnostic evidence bundle: non-root, read-only, metadata-only. Zero packet payloads, zero decrypted TLS, zero environment variables, zero process arguments, zero secrets.";

#[derive(Clone, Debug, Serialize)]
pub struct EvidenceExport<'a> {
    pub schema_version: &'static str,
    pub export_type: &'static str,
    pub clock_source: &'static str,
    pub privacy_declaration: &'static str,
    pub mode: &'static str,
    pub monotonic_ns: u64,
    pub collection_duration_ns: u64,
    pub snapshot: &'a Snapshot,
    pub doctor: DoctorReport<'a>,
    pub network: &'a NetworkReport,
}

impl<'a> EvidenceExport<'a> {
    pub fn new(
        snapshot: &'a Snapshot,
        doctor: DoctorReport<'a>,
        network: &'a NetworkReport,
        monotonic_ns: u64,
        collection_duration_ns: u64,
    ) -> Self {
        Self {
            schema_version: EXPORT_SCHEMA_VERSION,
            export_type: EXPORT_TYPE,
            clock_source: EXPORT_CLOCK_SOURCE,
            privacy_declaration: EXPORT_PRIVACY_DECLARATION,
            mode: mode_name(snapshot.mode()),
            monotonic_ns,
            collection_duration_ns,
            snapshot,
            doctor,
            network,
        }
    }
}

pub fn export_text(export: &EvidenceExport<'_>) -> String {
    format!(
        "LiDB Diagnostic Evidence Bundle v{}\n\
         ========================================\n\
         Export Type: {}\n\
         Mode: {}\n\
         Collector Monotonic: {} ns\n\
         Collection Duration: {} ns\n\
         Privacy: {}\n\n\
         --- [1/3] Host Metrics Snapshot ---\n\
         {}\n\
         --- [2/3] Source Doctor Report ---\n\
         {}\n\
         --- [3/3] Network Telemetry Report ---\n\
         {}",
        export.schema_version,
        export.export_type,
        export.mode,
        export.monotonic_ns,
        export.collection_duration_ns,
        export.privacy_declaration,
        snapshot_text(export.snapshot),
        export.doctor.text(),
        network_text(export.network),
    )
}

impl<'a> DoctorReport<'a> {
    pub fn from_snapshot(snapshot: &'a Snapshot) -> Self {
        let mut sources = BTreeMap::<&str, SourceCapability<'_>>::new();
        let mut available = 0;
        let mut missing = 0;
        for metric in snapshot.metrics() {
            let source = sources.entry(metric.source()).or_insert(SourceCapability {
                source: metric.source(),
                status: "unavailable",
                available_metrics: 0,
                unavailable_metrics: 0,
                failures: Vec::new(),
            });
            if metric.state().available_value().is_some() {
                source.available_metrics += 1;
                available += 1;
            } else {
                source.unavailable_metrics += 1;
                missing += 1;
                source.failures.push(Failure {
                    metric: metric.name(),
                    status: metric.state().name(),
                    reason: metric.state().reason().unwrap_or("no reason reported"),
                });
            }
        }
        for source in sources.values_mut() {
            source.status = if source.available_metrics > 0 {
                if source.unavailable_metrics > 0 {
                    "degraded"
                } else {
                    "available"
                }
            } else if source
                .failures
                .iter()
                .all(|failure| failure.status == source.failures[0].status)
            {
                source.failures[0].status
            } else {
                "unavailable"
            };
        }
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            mode: mode_name(snapshot.mode()),
            collection_status: if available == 0 {
                "unavailable"
            } else if missing > 0 || snapshot.dropped_metrics() > 0 {
                "degraded"
            } else {
                "available"
            },
            usable: available > 0,
            monotonic_ns: snapshot.monotonic_ns(),
            collection_duration_ns: snapshot.collection_duration_ns(),
            dropped_metrics: snapshot.dropped_metrics(),
            capabilities: sources.into_values().collect(),
        }
    }

    pub fn exit_code(&self) -> u8 {
        u8::from(!self.usable)
    }

    pub fn text(&self) -> String {
        let mut out = format!(
            "LiDB {} doctor: {} (source collection only; read-only; no active probes)\ncollection={}ns | dropped_metrics={}\n",
            self.mode, self.collection_status, self.collection_duration_ns, self.dropped_metrics
        );
        for source in &self.capabilities {
            let _ = writeln!(
                out,
                "{}: {} ({} available, {} unavailable)",
                source.source, source.status, source.available_metrics, source.unavailable_metrics
            );
            for failure in &source.failures {
                let _ = writeln!(
                    out,
                    "  {}: {} — {}",
                    failure.metric, failure.status, failure.reason
                );
            }
        }
        out
    }
}

pub fn ebpf_text(report: &lidb_core::ebpf::EbpfReport) -> String {
    let mut out = format!(
        "LiDB {} ebpf | schema={} | collector_monotonic={}ns | collection={}ns | status={} | events={} | dropped={}\n",
        mode_name(report.mode()),
        report.schema(),
        report.collection_time_ns(),
        report.collection_duration_ns(),
        report.status().as_str(),
        report.events().len(),
        report.dropped_events()
    );
    if let Some(reason) = report.status().reason() {
        let _ = writeln!(out, "status reason: {reason}");
    }
    if !report.events().is_empty() {
        for ev in report.events() {
            let rtt_text = match ev.round_trip_us() {
                Some(rtt) => format!(" rtt:{rtt}us"),
                None => String::new(),
            };
            let state_text = match (ev.old_state(), ev.new_state()) {
                (Some(old_st), Some(new_st)) => {
                    format!(" state:{}->{}", old_st.as_str(), new_st.as_str())
                }
                (None, Some(new_st)) => format!(" state:{}", new_st.as_str()),
                _ => String::new(),
            };
            let _ = writeln!(
                out,
                "{:<14} pid:{:<6} comm:{:<12} {}:{} -> {}:{}{}{}",
                ev.event_type().as_str(),
                ev.pid(),
                ev.comm(),
                ev.local_address(),
                ev.local_port(),
                ev.remote_address(),
                ev.remote_port(),
                state_text,
                rtt_text
            );
        }
    } else if !report.status().is_available() {
        let _ = writeln!(out, "eBPF subsystem unavailable");
    } else if report.dropped_events() == 0 {
        let _ = writeln!(out, "no eBPF events observed");
    } else {
        let _ = writeln!(out, "no events retained; buffer has omissions");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::{MetricState, Unit};

    #[test]
    fn doctor_reports_partial_sources_and_real_zero_as_usable() {
        let mut snapshot = Snapshot::new(0, 0, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "zero",
                    "procfs/stat",
                    Unit::Count,
                    0,
                    MetricState::Available(MetricValue::Integer(0)),
                )
                .unwrap(),
            )
            .unwrap();
        snapshot
            .push(
                MetricObservation::new(
                    "rate",
                    "procfs/stat",
                    Unit::Percent,
                    0,
                    MetricState::TemporarilyUnavailable("need two samples".into()),
                )
                .unwrap(),
            )
            .unwrap();
        let report = DoctorReport::from_snapshot(&snapshot);
        assert_eq!(report.exit_code(), 0);
        assert_eq!(report.collection_status, "degraded");
        assert_eq!(report.capabilities[0].status, "degraded");
        assert!(report.text().contains("need two samples"));
        assert!(snapshot_text(&snapshot).contains("zero = 0 count"));
        let empty = Snapshot::new(0, 0, SnapshotMode::Live);
        assert_eq!(DoctorReport::from_snapshot(&empty).exit_code(), 1);
    }

    #[test]
    fn evidence_export_formats_cleanly_with_privacy_declaration() {
        let mut snapshot = Snapshot::new(100, 50, SnapshotMode::Fixture);
        snapshot
            .push(
                MetricObservation::new(
                    "cpu.user",
                    "procfs/stat",
                    Unit::Percent,
                    100,
                    MetricState::Available(MetricValue::Number(12.5)),
                )
                .unwrap(),
            )
            .unwrap();
        let doctor = DoctorReport::from_snapshot(&snapshot);
        let network = NetworkReport::new(100, 25, SnapshotMode::Fixture);
        let export = EvidenceExport::new(&snapshot, doctor, &network, 100, 75);

        assert_eq!(export.schema_version, "0.1");
        assert_eq!(export.export_type, "offline_diagnostic_evidence");
        assert_eq!(export.clock_source, "collector_monotonic");
        assert!(export.privacy_declaration.contains("Zero packet payloads"));
        assert_eq!(export.mode, "fixture");
        assert_eq!(export.monotonic_ns, 100);
        assert_eq!(export.collection_duration_ns, 75);

        let text = export_text(&export);
        assert!(text.contains("LiDB Diagnostic Evidence Bundle v0.1"));
        assert!(text.contains("Export Type: offline_diagnostic_evidence"));
        assert!(text.contains("--- [1/3] Host Metrics Snapshot ---"));
        assert!(text.contains("--- [2/3] Source Doctor Report ---"));
        assert!(text.contains("--- [3/3] Network Telemetry Report ---"));
        assert!(text.contains("cpu.user = 12.50 percent"));
        assert!(text.contains("Zero packet payloads"));

        let serialized = serde_json::to_string(&export).unwrap();
        assert!(serialized.contains("\"schema_version\":\"0.1\""));
        assert!(serialized.contains("\"export_type\":\"offline_diagnostic_evidence\""));
        assert!(serialized.contains("Zero packet payloads"));
    }
}
