//! Bounded, read-only network interface metadata collection from Linux sysfs and procfs.
//!
//! Collects interface indices, MTU, operational state, MAC address, link speed,
//! flags, and best-effort network namespace identity into [`InterfaceRecord`] entries.
//! Enforces [`MAX_NETWORK_INTERFACES`] bounds, prevents fixture symlink escapes,
//! and differentiates empty tables from failed sources truthfully.

#[cfg(any(test, target_os = "linux"))]
use lidb_core::network::MAX_INTERFACE_NAME_CHARS;
#[cfg(target_os = "linux")]
use lidb_core::network::{InterfaceOperState, MAX_MAC_ADDRESS_CHARS, MAX_NETWORK_INTERFACES};
use lidb_core::network::{
    InterfaceRecord, NetworkError, NetworkNamespace, NetworkReport, SourceAvailability,
};
use lidb_core::SnapshotMode;
use serde::Serialize;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::time::Instant;

/// Pre-stable schema for the interface-only report, separate from snapshot JSON.
pub const INTERFACE_REPORT_SCHEMA_VERSION: &str = "0.1";

/// Maximum bytes accepted from any one interface attribute file (e.g. `mtu`, `operstate`).
pub const MAX_ATTRIBUTE_BYTES: usize = 4_096;

/// Standard Linux interface flag bits from `<linux/if.h>`.
const IFF_UP: u32 = 1 << 0;
const IFF_BROADCAST: u32 = 1 << 1;
const IFF_DEBUG: u32 = 1 << 2;
const IFF_LOOPBACK: u32 = 1 << 3;
const IFF_POINTOPOINT: u32 = 1 << 4;
const IFF_NOTRAILERS: u32 = 1 << 5;
const IFF_RUNNING: u32 = 1 << 6;
const IFF_NOARP: u32 = 1 << 7;
const IFF_PROMISC: u32 = 1 << 8;
const IFF_ALLMULTI: u32 = 1 << 9;
const IFF_MASTER: u32 = 1 << 10;
const IFF_SLAVE: u32 = 1 << 11;
const IFF_MULTICAST: u32 = 1 << 12;
const IFF_PORTSEL: u32 = 1 << 13;
const IFF_AUTOMEDIA: u32 = 1 << 14;
const IFF_DYNAMIC: u32 = 1 << 15;
const IFF_LOWER_UP: u32 = 1 << 16;
const IFF_DORMANT: u32 = 1 << 17;
const IFF_ECHO: u32 = 1 << 18;

/// Decodes standard Linux interface flags into textual names.
#[must_use]
pub fn decode_interface_flags(flags: u32) -> Vec<String> {
    let mut names = Vec::new();
    if flags & IFF_UP != 0 {
        names.push("UP".into());
    }
    if flags & IFF_BROADCAST != 0 {
        names.push("BROADCAST".into());
    }
    if flags & IFF_DEBUG != 0 {
        names.push("DEBUG".into());
    }
    if flags & IFF_LOOPBACK != 0 {
        names.push("LOOPBACK".into());
    }
    if flags & IFF_POINTOPOINT != 0 {
        names.push("POINTOPOINT".into());
    }
    if flags & IFF_NOTRAILERS != 0 {
        names.push("NOTRAILERS".into());
    }
    if flags & IFF_RUNNING != 0 {
        names.push("RUNNING".into());
    }
    if flags & IFF_NOARP != 0 {
        names.push("NOARP".into());
    }
    if flags & IFF_PROMISC != 0 {
        names.push("PROMISC".into());
    }
    if flags & IFF_ALLMULTI != 0 {
        names.push("ALLMULTI".into());
    }
    if flags & IFF_MASTER != 0 {
        names.push("MASTER".into());
    }
    if flags & IFF_SLAVE != 0 {
        names.push("SLAVE".into());
    }
    if flags & IFF_MULTICAST != 0 {
        names.push("MULTICAST".into());
    }
    if flags & IFF_PORTSEL != 0 {
        names.push("PORTSEL".into());
    }
    if flags & IFF_AUTOMEDIA != 0 {
        names.push("AUTOMEDIA".into());
    }
    if flags & IFF_DYNAMIC != 0 {
        names.push("DYNAMIC".into());
    }
    if flags & IFF_LOWER_UP != 0 {
        names.push("LOWER_UP".into());
    }
    if flags & IFF_DORMANT != 0 {
        names.push("DORMANT".into());
    }
    if flags & IFF_ECHO != 0 {
        names.push("ECHO".into());
    }
    names
}

/// Outcome of reading host network interfaces and namespace identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InterfaceReport {
    /// Interface-report schema version.
    pub schema_version: &'static str,
    /// Origin of the batch timestamp; comparable only within one collector lifetime.
    pub clock_source: &'static str,
    /// Batch start in nanoseconds since construction of the owning collector.
    pub monotonic_ns: u64,
    /// Duration of the sequential attribute reads, in nanoseconds.
    pub collection_duration_ns: u64,
    /// Live process view or explicitly selected fixture input.
    pub mode: SnapshotMode,
    /// Source identifier, without a private fixture path.
    pub source: &'static str,
    /// Availability status of the interfaces source.
    pub status: SourceAvailability,
    /// Best-effort captured network namespace context.
    pub namespace: Option<NetworkNamespace>,
    /// Parsed interface records, bounded by [`MAX_NETWORK_INTERFACES`].
    pub interfaces: Vec<InterfaceRecord>,
    /// Number of valid interface entries dropped because [`MAX_NETWORK_INTERFACES`] capacity was reached.
    pub dropped_interfaces: u64,
}

impl InterfaceReport {
    /// Creates a report with default uncollected status.
    #[must_use]
    pub fn new(mode: SnapshotMode, monotonic_ns: u64) -> Self {
        Self {
            schema_version: INTERFACE_REPORT_SCHEMA_VERSION,
            clock_source: "collector_monotonic",
            monotonic_ns,
            collection_duration_ns: 0,
            mode,
            source: "sysfs/class/net",
            status: SourceAvailability::Disabled("source not collected".into()),
            namespace: None,
            interfaces: Vec::new(),
            dropped_interfaces: 0,
        }
    }

    /// Appends interfaces and omission counts into an existing network report.
    ///
    /// Rejects known mode, clock source, monotonic timestamp, or duration mismatch.
    /// If the destination report already has an incompatible namespace, returns an error.
    /// Rejection is atomic without mutating the target.
    pub fn populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError> {
        if report.mode() != self.mode
            || report.monotonic_ns() != self.monotonic_ns
            || report.collection_duration_ns() < self.collection_duration_ns
            || report.clock_source() != self.clock_source
        {
            return Err(NetworkError::ReportContextMismatch);
        }
        if let Some(target_ns) = report.namespace() {
            if let Some(src_ns) = &self.namespace {
                if target_ns != src_ns {
                    return Err(NetworkError::ReportContextMismatch);
                }
            } else {
                return Err(NetworkError::ReportContextMismatch);
            }
        } else if let Some(src_ns) = &self.namespace {
            report.set_namespace(Some(src_ns.clone()));
        }
        report.set_interfaces_status(self.status.clone());
        report.record_dropped_interfaces(self.dropped_interfaces);
        for iface in &self.interfaces {
            let _ = report.push_interface(iface.clone());
        }
        Ok(())
    }
}

/// Validates an interface name for path safety and format boundaries.
#[cfg(any(test, target_os = "linux"))]
fn validate_interface_name(name: &str) -> Result<(), SourceAvailability> {
    if name.is_empty() {
        return Err(SourceAvailability::Error("interface name is empty".into()));
    }
    if name.len() > MAX_INTERFACE_NAME_CHARS {
        return Err(SourceAvailability::Error(format!(
            "interface name '{name}' exceeds limit of {MAX_INTERFACE_NAME_CHARS} characters"
        )));
    }
    if name == "." || name == ".." {
        return Err(SourceAvailability::Error(
            "interface name must not be a relative directory component".into(),
        ));
    }
    if name
        .chars()
        .any(|c| c.is_control() || c == '/' || c == '\\' || c == '\0')
    {
        return Err(SourceAvailability::Error(
            "interface name contains forbidden characters".into(),
        ));
    }
    Ok(())
}

/// Collects network interfaces from the specified procfs and sysfs roots.
#[must_use]
pub fn collect_interfaces(
    proc_root: &Path,
    sys_root: &Path,
    mode: SnapshotMode,
) -> InterfaceReport {
    collect_interfaces_at(proc_root, sys_root, mode, 0, Instant::now())
}

pub(crate) fn collect_interfaces_at(
    proc_root: &Path,
    sys_root: &Path,
    mode: SnapshotMode,
    monotonic_ns: u64,
    began: Instant,
) -> InterfaceReport {
    #[cfg(target_os = "linux")]
    {
        if !cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
            return unsupported_report(
                mode,
                monotonic_ns,
                began,
                "host collection requires Linux x86_64 or aarch64",
            );
        }
        linux_collect_interfaces(proc_root, sys_root, mode, monotonic_ns, began)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (proc_root, sys_root);
        unsupported_report(
            mode,
            monotonic_ns,
            began,
            "host collection requires Linux x86_64 or aarch64",
        )
    }
}

#[cfg(not(target_os = "linux"))]
fn unsupported_report(
    mode: SnapshotMode,
    monotonic_ns: u64,
    began: Instant,
    reason: &str,
) -> InterfaceReport {
    InterfaceReport {
        status: SourceAvailability::Unsupported(reason.into()),
        collection_duration_ns: crate::duration_ns(began.elapsed()),
        ..InterfaceReport::new(mode, monotonic_ns)
    }
}

#[cfg(target_os = "linux")]
fn unsupported_report(
    mode: SnapshotMode,
    monotonic_ns: u64,
    began: Instant,
    reason: &str,
) -> InterfaceReport {
    InterfaceReport {
        status: SourceAvailability::Unsupported(reason.into()),
        collection_duration_ns: crate::duration_ns(began.elapsed()),
        ..InterfaceReport::new(mode, monotonic_ns)
    }
}

#[cfg(target_os = "linux")]
fn linux_collect_interfaces(
    proc_root: &Path,
    sys_root: &Path,
    mode: SnapshotMode,
    monotonic_ns: u64,
    began: Instant,
) -> InterfaceReport {
    use std::fs;

    // Detect network namespace identity best-effort from procfs.
    let namespace = detect_namespace(proc_root);

    // Locate the net class directory: either <sys_root>/class/net or <sys_root>/net or <sys_root>.
    let class_net = resolve_class_net(sys_root);
    let class_net = match class_net {
        Ok(dir) => dir,
        Err(status) => {
            return InterfaceReport {
                status,
                namespace,
                collection_duration_ns: crate::duration_ns(began.elapsed()),
                ..InterfaceReport::new(mode, monotonic_ns)
            };
        }
    };

    let read_dir_result = fs::read_dir(&class_net);
    let dir_entries = match read_dir_result {
        Ok(entries) => entries,
        Err(err) => {
            return InterfaceReport {
                status: io_error_to_availability(err),
                namespace,
                collection_duration_ns: crate::duration_ns(began.elapsed()),
                ..InterfaceReport::new(mode, monotonic_ns)
            };
        }
    };

    let mut iface_names = Vec::new();
    for entry in dir_entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // Ignore individual transient directory read failures
        };
        let file_name = entry.file_name();
        let name_str = match file_name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if name_str.starts_with('.') {
            continue;
        }
        if let Err(status) = validate_interface_name(name_str) {
            return InterfaceReport {
                status,
                namespace,
                collection_duration_ns: crate::duration_ns(began.elapsed()),
                ..InterfaceReport::new(mode, monotonic_ns)
            };
        }
        iface_names.push(name_str.to_string());
    }

    // Sort interface names for deterministic evaluation
    iface_names.sort();

    let mut interfaces = Vec::new();
    let mut dropped_interfaces = 0u64;

    for name in iface_names {
        let iface_path = class_net.join(&name);

        // Path and symlink security validation
        if let Err(status) = validate_iface_path(&iface_path, sys_root, mode) {
            return InterfaceReport {
                status,
                namespace,
                collection_duration_ns: crate::duration_ns(began.elapsed()),
                ..InterfaceReport::new(mode, monotonic_ns)
            };
        }

        match read_interface_record(&iface_path, &name, mode) {
            Ok(Some(record)) => {
                if interfaces.len() < MAX_NETWORK_INTERFACES {
                    interfaces.push(record);
                } else {
                    dropped_interfaces = dropped_interfaces.saturating_add(1);
                }
            }
            Ok(None) => {
                // Interface disappeared during collection; handle gracefully without aborting
                continue;
            }
            Err(status) => {
                return InterfaceReport {
                    status,
                    namespace,
                    collection_duration_ns: crate::duration_ns(began.elapsed()),
                    ..InterfaceReport::new(mode, monotonic_ns)
                };
            }
        }
    }

    InterfaceReport {
        status: SourceAvailability::Available,
        namespace,
        interfaces,
        dropped_interfaces,
        collection_duration_ns: crate::duration_ns(began.elapsed()),
        ..InterfaceReport::new(mode, monotonic_ns)
    }
}

/// Resolves the network class directory across standard and fixture structures.
#[cfg(target_os = "linux")]
fn resolve_class_net(sys_root: &Path) -> Result<PathBuf, SourceAvailability> {
    use std::fs;

    let candidates = [
        sys_root.join("class/net"),
        sys_root.join("net"),
        sys_root.to_path_buf(),
    ];
    for candidate in &candidates {
        if let Ok(meta) = fs::symlink_metadata(candidate) {
            if meta.is_dir() {
                return Ok(candidate.clone());
            }
        }
    }

    // Check if sys_root exists at all to provide accurate error reasons
    match fs::symlink_metadata(sys_root) {
        Ok(_) => Err(SourceAvailability::Unsupported(
            "sysfs network class directory is not present".into(),
        )),
        Err(err) => Err(io_error_to_availability(err)),
    }
}

/// Validates that an interface directory does not escape the sysfs fixture root.
#[cfg(target_os = "linux")]
fn validate_iface_path(
    iface_path: &Path,
    sys_root: &Path,
    mode: SnapshotMode,
) -> Result<(), SourceAvailability> {
    use std::fs;

    let meta = match fs::symlink_metadata(iface_path) {
        Ok(m) => m,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // Disappearing interface
            return Ok(());
        }
        Err(err) => return Err(io_error_to_availability(err)),
    };

    if mode == SnapshotMode::Fixture && meta.file_type().is_symlink() {
        // In fixture mode, symlinks must resolve inside sys_root
        let canonical_target = match fs::canonicalize(iface_path) {
            Ok(p) => p,
            Err(_) => {
                return Err(SourceAvailability::Error(
                    "fixture interface symlink target cannot be resolved".into(),
                ));
            }
        };
        let canonical_root = match fs::canonicalize(sys_root) {
            Ok(p) => p,
            Err(_) => {
                return Err(SourceAvailability::Error(
                    "fixture sys root cannot be resolved".into(),
                ));
            }
        };
        if !canonical_target.starts_with(&canonical_root) {
            return Err(SourceAvailability::Error(
                "fixture interface symlink escapes fixture root".into(),
            ));
        }
    }
    Ok(())
}

/// Reads attributes for an individual network interface into an [`InterfaceRecord`].
#[cfg(target_os = "linux")]
fn read_interface_record(
    iface_path: &Path,
    name: &str,
    mode: SnapshotMode,
) -> Result<Option<InterfaceRecord>, SourceAvailability> {
    use std::fs;

    // Check if directory is accessible
    if let Err(err) = fs::metadata(iface_path) {
        if err.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(io_error_to_availability(err));
    }

    // Read operstate (defaults to Unknown if missing/unreadable)
    let oper_state = match read_attribute_string(iface_path, "operstate", mode) {
        Ok(Some(text)) => {
            InterfaceOperState::parse_operstate(&text).unwrap_or(InterfaceOperState::Unknown)
        }
        Ok(None) => InterfaceOperState::Unknown,
        Err(status) => return Err(status),
    };

    let mut record = InterfaceRecord::new(name, oper_state)
        .map_err(|e| SourceAvailability::Error(e.to_string()))?;

    // Read ifindex
    match read_attribute_string(iface_path, "ifindex", mode) {
        Ok(Some(text)) => {
            if let Ok(idx) = text.trim().parse::<u32>() {
                record = record.with_ifindex(idx);
            } else {
                return Err(SourceAvailability::Error(
                    "invalid ifindex integer in sysfs".into(),
                ));
            }
        }
        Ok(None) => {}
        Err(status) => return Err(status),
    }

    // Read mtu
    match read_attribute_string(iface_path, "mtu", mode) {
        Ok(Some(text)) => {
            if let Ok(mtu) = text.trim().parse::<u32>() {
                record = record.with_mtu(mtu);
            } else {
                return Err(SourceAvailability::Error(
                    "invalid mtu integer in sysfs".into(),
                ));
            }
        }
        Ok(None) => {}
        Err(status) => return Err(status),
    }

    // Read MAC address
    match read_attribute_string(iface_path, "address", mode) {
        Ok(Some(text)) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                if trimmed.len() <= MAX_MAC_ADDRESS_CHARS && !trimmed.chars().any(char::is_control)
                {
                    record = record
                        .with_mac_address(trimmed)
                        .map_err(|e| SourceAvailability::Error(e.to_string()))?;
                } else {
                    return Err(SourceAvailability::Error(
                        "invalid mac address in sysfs".into(),
                    ));
                }
            }
        }
        Ok(None) => {}
        Err(status) => return Err(status),
    }

    // Read link speed in Mbps (Option<u64>, negative or missing means None)
    match read_attribute_string(iface_path, "speed", mode) {
        Ok(Some(text)) => {
            let trimmed = text.trim();
            // In Linux, virtual interfaces (like lo) or disconnected cables return -1 (EINVAL)
            if !trimmed.starts_with('-') {
                if let Ok(speed) = trimmed.parse::<u64>() {
                    record = record.with_speed_mbps(speed);
                }
            }
        }
        Ok(None) => {}
        Err(_) => {
            // Sysfs read failure on speed (e.g. EINVAL on virtual device) is safely ignored
        }
    }

    // Read interface flags (hex integer like 0x1003)
    match read_attribute_string(iface_path, "flags", mode) {
        Ok(Some(text)) => {
            let trimmed = text.trim();
            let hex_str = trimmed.strip_prefix("0x").unwrap_or(trimmed);
            if let Ok(raw_flags) = u32::from_str_radix(hex_str, 16) {
                let decoded = decode_interface_flags(raw_flags);
                record = record
                    .with_flags(decoded)
                    .map_err(|e| SourceAvailability::Error(e.to_string()))?;
            } else {
                return Err(SourceAvailability::Error(
                    "invalid flags hex in sysfs".into(),
                ));
            }
        }
        Ok(None) => {}
        Err(status) => return Err(status),
    }

    Ok(Some(record))
}

/// Reads a bounded string from an interface attribute file.
#[cfg(target_os = "linux")]
fn read_attribute_string(
    iface_path: &Path,
    attribute: &str,
    _mode: SnapshotMode,
) -> Result<Option<String>, SourceAvailability> {
    use std::fs::File;
    use std::io::Read;

    let path = iface_path.join(attribute);
    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(io_error_to_availability(err)),
    };

    let mut buffer = Vec::new();
    let mut take = (&mut file).take((MAX_ATTRIBUTE_BYTES + 1) as u64);
    if let Err(err) = take.read_to_end(&mut buffer) {
        return Err(io_error_to_availability(err));
    }
    if buffer.len() > MAX_ATTRIBUTE_BYTES {
        return Err(SourceAvailability::Error(format!(
            "attribute '{attribute}' exceeds byte limit"
        )));
    }
    let text = String::from_utf8(buffer).map_err(|_| {
        SourceAvailability::Error(format!("attribute '{attribute}' contains invalid UTF-8"))
    })?;

    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(SourceAvailability::Error(format!(
            "attribute '{attribute}' contains forbidden control characters"
        )));
    }

    Ok(Some(text))
}

/// Detects the network namespace context best-effort from procfs.
#[cfg(target_os = "linux")]
fn detect_namespace(proc_root: &Path) -> Option<NetworkNamespace> {
    use std::fs;
    #[cfg(target_os = "linux")]
    use std::os::unix::fs::MetadataExt;

    let ns_path = proc_root.join("self/ns/net");
    if let Ok(meta) = fs::symlink_metadata(&ns_path) {
        let inode = meta.ino();
        if inode > 0 {
            return NetworkNamespace::new("current", Some(inode)).ok();
        }
    }
    if let Ok(target) = fs::read_link(&ns_path) {
        let lossy = target.to_string_lossy();
        if let Some(rest) = lossy.strip_prefix("net:[") {
            if let Some(num_str) = rest.strip_suffix(']') {
                if let Ok(inode) = num_str.parse::<u64>() {
                    return NetworkNamespace::new("current", Some(inode)).ok();
                }
            }
        }
    }
    None
}

/// Converts a [`std::io::Error`] into a truthful [`SourceAvailability`].
#[cfg(target_os = "linux")]
fn io_error_to_availability(err: std::io::Error) -> SourceAvailability {
    match err.kind() {
        std::io::ErrorKind::NotFound => {
            SourceAvailability::Unsupported("source is not present".into())
        }
        std::io::ErrorKind::PermissionDenied => {
            SourceAvailability::PermissionDenied("source access is denied".into())
        }
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted => {
            SourceAvailability::TemporarilyUnavailable("source read did not complete".into())
        }
        _ => SourceAvailability::Error("source read failed".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lidb_core::network::InterfaceOperState;

    #[test]
    fn decode_interface_flags_standard_bits() {
        // UP | BROADCAST | MULTICAST = 0x1 | 0x2 | 0x1000 = 0x1003
        let decoded = decode_interface_flags(0x1003);
        assert_eq!(decoded, vec!["UP", "BROADCAST", "MULTICAST"]);

        // LOOPBACK | UP | RUNNING = 0x8 | 0x1 | 0x40 = 0x49
        let lo_flags = decode_interface_flags(0x49);
        assert_eq!(lo_flags, vec!["UP", "LOOPBACK", "RUNNING"]);

        let empty = decode_interface_flags(0);
        assert!(empty.is_empty());
    }

    #[test]
    fn validate_interface_name_bounds_and_controls() {
        assert!(validate_interface_name("eth0").is_ok());
        assert!(validate_interface_name("lo").is_ok());
        assert!(validate_interface_name("wlan0.100").is_ok());
        assert!(validate_interface_name("br-lan").is_ok());
        assert!(validate_interface_name("vpn+prod").is_ok());

        assert!(validate_interface_name("").is_err());
        assert!(validate_interface_name(".").is_err());
        assert!(validate_interface_name("..").is_err());
        assert!(validate_interface_name("eth/0").is_err());
        assert!(validate_interface_name("eth\\0").is_err());
        assert!(validate_interface_name("eth\x1b[2J").is_err());
        assert!(validate_interface_name("this_interface_name_is_way_too_long").is_err());
    }

    #[test]
    fn interface_report_populate_into_network_report() {
        let iface = InterfaceRecord::new("eth0", InterfaceOperState::Up)
            .unwrap()
            .with_ifindex(2)
            .with_mtu(1500)
            .with_mac_address("52:54:00:12:34:56")
            .unwrap()
            .with_speed_mbps(1000);

        let report_src = InterfaceReport {
            status: SourceAvailability::Available,
            interfaces: vec![iface],
            dropped_interfaces: 0,
            ..InterfaceReport::new(SnapshotMode::Live, 100)
        };

        let mut net_report = NetworkReport::new(100, 10, SnapshotMode::Live);
        report_src.populate_into(&mut net_report).unwrap();

        assert_eq!(
            net_report.interfaces_status(),
            &SourceAvailability::Available
        );
        assert_eq!(net_report.interfaces().len(), 1);
        assert_eq!(net_report.interfaces()[0].name(), "eth0");
        assert_eq!(net_report.interfaces()[0].ifindex(), Some(2));
        assert_eq!(net_report.interfaces()[0].mtu(), Some(1500));
        assert_eq!(net_report.interfaces()[0].speed_mbps(), Some(1000));
        assert_eq!(
            net_report.interfaces()[0].mac_address(),
            Some("52:54:00:12:34:56")
        );
    }

    #[test]
    fn interface_report_population_rejects_context_and_namespace_mismatch() {
        let iface = InterfaceRecord::new("eth0", InterfaceOperState::Up).unwrap();
        let report_src = InterfaceReport {
            status: SourceAvailability::Available,
            interfaces: vec![iface],
            dropped_interfaces: 0,
            collection_duration_ns: 20,
            namespace: Some(NetworkNamespace::new("init", Some(1234)).unwrap()),
            ..InterfaceReport::new(SnapshotMode::Fixture, 50)
        };

        let other_ns = NetworkNamespace::new("other", Some(5678)).unwrap();

        // Mode mismatch
        let mut target_mode = NetworkReport::new(50, 20, SnapshotMode::Live);
        assert_eq!(
            report_src.populate_into(&mut target_mode),
            Err(NetworkError::ReportContextMismatch)
        );

        // Timestamp mismatch
        let mut target_time = NetworkReport::new(51, 20, SnapshotMode::Fixture);
        assert_eq!(
            report_src.populate_into(&mut target_time),
            Err(NetworkError::ReportContextMismatch)
        );

        // Duration shorter than source
        let mut target_dur = NetworkReport::new(50, 10, SnapshotMode::Fixture);
        assert_eq!(
            report_src.populate_into(&mut target_dur),
            Err(NetworkError::ReportContextMismatch)
        );

        // Namespace mismatch
        let mut target_ns =
            NetworkReport::new(50, 20, SnapshotMode::Fixture).with_namespace(other_ns);
        assert_eq!(
            report_src.populate_into(&mut target_ns),
            Err(NetworkError::ReportContextMismatch)
        );
    }

    #[test]
    fn collect_interfaces_on_empty_or_nonexistent_dir() {
        let temp_dir =
            std::env::temp_dir().join(format!("lidb_test_ifaces_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let report = collect_interfaces(&temp_dir, &temp_dir, SnapshotMode::Fixture);
        // On non-Linux or without the files, it truthfully reports unsupported
        assert!(matches!(report.status, SourceAvailability::Unsupported(_)));
    }
}
