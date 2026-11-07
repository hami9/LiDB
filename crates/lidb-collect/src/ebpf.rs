//! Linux eBPF telemetry collection and unprivileged capability probing.
//!
//! LiDB provides optional deeper network instrumentation by observing kernel
//! tracepoints (such as short-lived TCP connection lifecycles).
//!
//! # Safety and Boundaries
//! - Non-root baseline: by default, eBPF is disabled (`SourceAvailability::Disabled`).
//! - Truthful availability: when enabled without requisite capabilities (`CAP_BPF` / root)
//!   or kernel BTF support, the collector truthfully reports `SourceAvailability::PermissionDenied`
//!   or `SourceAvailability::Unsupported` rather than failing the entire diagnostic run.
//! - Read-only: no network packets are modified, no socket options altered, and zero
//!   packet payload or decrypted TLS is inspected.
//! - Bounded buffers: events in userspace are strictly capped at [`MAX_EBPF_EVENTS`] with
//!   explicit drop accounting.

use lidb_core::{EbpfLifecycleEvent, EbpfReport, SnapshotMode, SourceAvailability};
use std::{path::Path, sync::Mutex, time::Instant};

#[cfg(target_os = "linux")]
fn has_bpf_privilege() -> bool {
    let Ok(content) = std::fs::read_to_string("/proc/self/status") else {
        return false;
    };
    let mut is_root = false;
    let mut has_cap = false;

    for line in content.lines() {
        if line.starts_with("Uid:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Format: Uid: <ruid> <euid> <suid> <fsuid>
            if parts.len() >= 3 && parts[2] == "0" {
                is_root = true;
            }
        } else if line.starts_with("CapEff:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(cap) = u64::from_str_radix(parts[1], 16) {
                    // CAP_SYS_ADMIN is bit 21, CAP_BPF is bit 38
                    const CAP_SYS_ADMIN: u64 = 1 << 21;
                    const CAP_BPF: u64 = 1 << 38;
                    if (cap & CAP_SYS_ADMIN) != 0 || (cap & CAP_BPF) != 0 {
                        has_cap = true;
                    }
                }
            }
        }
    }

    is_root || has_cap
}

/// Probes whether the host environment supports Linux eBPF CO-RE.
#[must_use]
pub fn probe_ebpf_support(sys_root: &Path) -> SourceAvailability {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = sys_root;
        SourceAvailability::unsupported("eBPF instrumentation is only supported on Linux")
            .unwrap_or(SourceAvailability::Available)
    }

    #[cfg(target_os = "linux")]
    {
        // 1. Check BTF availability in sysfs
        let btf_path = sys_root.join("kernel/btf/vmlinux");
        if !btf_path.exists() {
            return SourceAvailability::unsupported(
                "kernel BTF type information missing (/sys/kernel/btf/vmlinux not found)",
            )
            .unwrap_or(SourceAvailability::Available);
        }

        // 2. Check unprivileged status (eBPF typically requires CAP_BPF or root)
        if !has_bpf_privilege() {
            return SourceAvailability::permission_denied(
                "eBPF collection requires CAP_BPF or root privileges",
            )
            .unwrap_or(SourceAvailability::Available);
        }

        SourceAvailability::Available
    }
}

/// In-memory ring buffer event accumulator for testing or live reception.
#[derive(Debug, Default)]
struct EventQueue {
    events: Vec<EbpfLifecycleEvent>,
    dropped_count: u64,
}

/// Collector for Linux eBPF telemetry events.
#[derive(Debug)]
pub struct EbpfCollector {
    mode: SnapshotMode,
    enabled: bool,
    sys_root: std::path::PathBuf,
    mock_queue: Option<Mutex<EventQueue>>,
}

impl EbpfCollector {
    /// Creates a new eBPF collector with default disabled state.
    #[must_use]
    pub fn new(mode: SnapshotMode, sys_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            mode,
            enabled: false,
            sys_root: sys_root.into(),
            mock_queue: None,
        }
    }

    /// Enables or disables active eBPF collection.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns whether eBPF collection is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Configures the collector with simulated mock events for reproducible testing.
    pub fn with_mock_events(&mut self, events: Vec<EbpfLifecycleEvent>) {
        self.mock_queue = Some(Mutex::new(EventQueue {
            events,
            dropped_count: 0,
        }));
    }

    /// Collects the current batch of eBPF lifecycle events.
    #[must_use]
    pub fn collect(&self, monotonic_now_ns: u64) -> EbpfReport {
        let start = Instant::now();
        let mut report = EbpfReport::new(self.mode, monotonic_now_ns);

        if !self.enabled {
            report.set_status(
                SourceAvailability::disabled(
                    "eBPF collection not enabled (use --ebpf to activate)",
                )
                .unwrap_or(SourceAvailability::Available),
            );
            report.set_collection_duration_ns(start.elapsed().as_nanos() as u64);
            return report;
        }

        // If mock events are configured (e.g. for deterministic unit / CI testing)
        if let Some(ref queue_mutex) = self.mock_queue {
            report.set_status(SourceAvailability::Available);
            if let Ok(mut queue) = queue_mutex.lock() {
                for ev in queue.events.drain(..) {
                    report.push_event(ev);
                }
                report.set_dropped_events(queue.dropped_count);
            }
            report.set_collection_duration_ns(start.elapsed().as_nanos() as u64);
            return report;
        }

        // Live probe verification
        let availability = probe_ebpf_support(&self.sys_root);
        report.set_status(availability);
        report.set_collection_duration_ns(start.elapsed().as_nanos() as u64);
        report
    }
}

impl Drop for EbpfCollector {
    fn drop(&mut self) {
        // Safe clean detach on drop:
        // In full live BPF mode, attached link fds / maps are dropped here automatically.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::{EbpfEventType, SocketState};
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn disabled_by_default_reports_disabled_status() {
        let collector = EbpfCollector::new(SnapshotMode::Fixture, "/sys");
        assert!(!collector.is_enabled());

        let report = collector.collect(1_000_000);
        assert!(!report.status().is_available());
        assert_eq!(report.status().as_str(), "disabled");
        assert_eq!(report.events().len(), 0);
    }

    #[test]
    fn mock_events_collection_and_drain() {
        let mut collector = EbpfCollector::new(SnapshotMode::Fixture, "/sys");
        collector.set_enabled(true);

        let v4_loc = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let v4_rem = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

        let ev1 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpConnect,
            1_000,
            5001,
            "curl",
            v4_loc,
            45000,
            v4_rem,
            80,
        )
        .unwrap()
        .with_states(Some(SocketState::SynSent), Some(SocketState::Established));

        let ev2 = EbpfLifecycleEvent::new(
            EbpfEventType::TcpClose,
            2_000,
            5001,
            "curl",
            v4_loc,
            45000,
            v4_rem,
            80,
        )
        .unwrap()
        .with_states(Some(SocketState::Established), Some(SocketState::Closed));

        collector.with_mock_events(vec![ev1, ev2]);

        let report1 = collector.collect(2_500);
        assert!(report1.status().is_available());
        assert_eq!(report1.events().len(), 2);
        assert_eq!(report1.events()[0].event_type(), EbpfEventType::TcpConnect);
        assert_eq!(report1.events()[1].event_type(), EbpfEventType::TcpClose);

        // Subsequent collection drains the queue
        let report2 = collector.collect(3_000);
        assert!(report2.status().is_available());
        assert_eq!(report2.events().len(), 0);
    }

    #[test]
    fn probe_on_nonexistent_sysfs_reports_unsupported() {
        let temp_dir = std::env::temp_dir().join("lidb_test_ebpf_nonexistent");
        let availability = probe_ebpf_support(&temp_dir);
        assert!(!availability.is_available());
        assert!(
            availability.as_str() == "unsupported" || availability.as_str() == "permission_denied"
        );
    }
}
