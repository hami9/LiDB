use serde_json::Value;
use std::process::{Command, Output};
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
struct Fixture(PathBuf);
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "lidash-cli-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("net")).unwrap();
        fs::create_dir(root.join("pressure")).unwrap();
        for (file, data) in [
            ("stat", "cpu 10 0 5 100 1 0 0 0 0 0\n"),
            ("meminfo", "MemTotal: 1024 kB\nMemAvailable: 512 kB\nMemFree: 100 kB\nSwapTotal: 128 kB\nSwapFree: 64 kB\n"),
            ("uptime", "123.25 99.00\n"),
            ("loadavg", "0.10 0.20 0.30 1/100 42\n"),
            ("net/dev", "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n eth0: 1000 2 3 4 0 0 0 0 2000 2 5 0 0 0 0 0\n"),
            ("diskstats", "8 0 sda 10 0 2 1 20 0 4 1 0 1 1 0 0 0 0\n"),
            ("pressure/cpu", "some avg10=1.00 avg60=0.50 avg300=0.20 total=123\n"),
            ("pressure/memory", "some avg10=2.00 avg60=1.00 avg300=0.20 total=123\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n"),
            ("pressure/io", "some avg10=3.00 avg60=1.00 avg300=0.20 total=123\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n"),
        ] {
            fs::write(root.join(file), data).unwrap();
        }
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lidash"))
            .args(args)
            .arg("--proc-root")
            .arg(&self.0)
            .output()
            .unwrap()
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn binary(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lidash"))
        .args(args)
        .output()
        .unwrap()
}

fn json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn help_version_invalid_options_and_nonterminal_tui_have_defined_exits() {
    let help = binary(&["--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--interval-ms"));
    let version = binary(&["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("lidash "));
    for args in [
        &["made-up"][..],
        &["--interval-ms", "0"],
        &["tui"],
        &["tui", "--json"],
        &["doctor", "snapshot"],
        &["snapshot", "--output", "foo.txt"],
        &["network", "--output", "foo.txt"],
    ] {
        let output = binary(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(!output.stderr.is_empty());
        assert!(output.stdout.is_empty());
    }
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn piped_default_and_json_export_use_real_fixture_values_and_rate_states() {
    let fixture = Fixture::new();
    let output = fixture.run(&[]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("fixture snapshot"));
    assert!(text.contains("memory.total.bytes = 1048576 bytes"));
    assert!(text.contains("proc.meminfo"));
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("network.eth0.rx.errors = 3 count"));
    assert!(text.contains("network.eth0.rx.drops = 4 count"));
    assert!(text.contains("network.eth0.tx.errors = 5 count"));
    assert!(text.contains("network.eth0.tx.drops = 0 count"));
    let output = fixture.run(&["snapshot", "--json", "--no-color"]);
    let data = json(&output);
    assert_eq!(data["schema_version"], "0.1");
    assert_eq!(data["mode"], "fixture");
    assert_eq!(data["clock_source"], "collector_monotonic");
    assert!(data["monotonic_ns"].as_u64().unwrap() > 0);
    let metrics = data["metrics"].as_array().unwrap();
    let memory = metrics
        .iter()
        .find(|metric| metric["name"] == "memory.total.bytes")
        .unwrap();
    assert_eq!(memory["state"]["status"], "available");
    assert_eq!(memory["state"]["value"].as_u64(), Some(1_048_576));
    let rate = metrics
        .iter()
        .find(|metric| metric["name"] == "network.eth0.rx.bytes_per_second")
        .unwrap();
    assert_eq!(rate["state"]["status"], "available");
    assert_eq!(rate["state"]["value"].as_f64(), Some(0.0));
    for (suffix, expected) in [
        ("rx.errors", 3),
        ("rx.drops", 4),
        ("tx.errors", 5),
        ("tx.drops", 0),
    ] {
        let name = format!("network.eth0.{suffix}");
        let counter = metrics
            .iter()
            .find(|metric| metric["name"] == name)
            .unwrap();
        assert_eq!(counter["state"]["status"], "available");
        assert_eq!(counter["state"]["value"].as_u64(), Some(expected));
        assert_eq!(counter["unit"], "count");
        assert_eq!(counter["source"], "proc.net.dev");
        assert_eq!(counter["monotonic_ns"], data["monotonic_ns"]);
    }
    assert!(!metrics
        .iter()
        .any(|metric| metric["name"].as_str().unwrap().contains("gpu")));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn doctor_derives_capabilities_from_reads_and_marks_complete_source_failure() {
    let fixture = Fixture::new();
    let usable = json(&fixture.run(&["doctor", "--json"]));
    assert_eq!(usable["mode"], "fixture");
    assert_eq!(usable["usable"], true);
    assert!(usable["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["available_metrics"].as_u64().unwrap() > 0));
    for entry in fs::read_dir(&fixture.0).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            fs::remove_dir_all(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
    }
    let output = fixture.run(&["capabilities", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let unavailable: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(unavailable["collection_status"], "unavailable");
    assert_eq!(unavailable["usable"], false);
    assert!(unavailable["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| source["available_metrics"] == 0));
    assert!(unavailable["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| !source["failures"].as_array().unwrap().is_empty()));
}

#[test]
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
fn non_linux_platforms_report_unsupported_cleanly() {
    let output = binary(&["snapshot", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["schema_version"], "0.1");
    let metrics = data["metrics"].as_array().unwrap();
    assert!(metrics
        .iter()
        .all(|metric| metric["state"]["status"] == "unsupported"));
}

#[test]
fn explicit_system_proc_root_remains_labeled_fixture() {
    let output = binary(&["status", "--json", "--proc-root", "/proc"]);
    assert_eq!(json(&output)["mode"], "fixture");
    let routes = json(&binary(&["routes", "--json", "--proc-root", "/proc"]));
    assert_eq!(routes["mode"], "fixture");
}

#[test]
fn routes_command_runs_cleanly_on_all_platforms() {
    let output = binary(&["routes", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert!(data.get("v4_status").is_some());
    assert!(data.get("v6_status").is_some());
    assert!(data.get("routes").is_some());
    assert!(data.get("dropped_routes").is_some());
    assert_eq!(data["mode"], "live");
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn routes_command_reads_fixture_routes() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("net/route"),
        "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n\
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n",
    )
    .unwrap();
    let output = fixture.run(&["routes", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["v4_status"]["status"], "available");
    assert_eq!(data["mode"], "fixture");
    assert_eq!(data["schema_version"], "0.1");
    assert_eq!(data["clock_source"], "collector_monotonic");
    assert_eq!(data["v4_source"], "procfs/net/route");
    assert_eq!(data["v6_source"], "procfs/net/ipv6_route");
    assert!(data["monotonic_ns"].is_u64());
    assert!(data["collection_duration_ns"].is_u64());
    let routes = data["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["destination"], "0.0.0.0");
    assert_eq!(routes[0]["gateway"], "192.168.1.1");
    assert_eq!(routes[0]["interface"], "eth0");

    let text = String::from_utf8(fixture.run(&["routes"]).stdout).unwrap();
    assert!(text.contains("LiDB fixture routes"));
    assert!(text.contains("0.0.0.0/0 dev eth0 via 192.168.1.1 metric 100"));
    assert!(text.contains("procfs/net/ipv6_route: unsupported — source is not present"));
    assert!(text.contains("IPv6 routing table unavailable"));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn routes_empty_failed_and_partial_sources_are_distinct() {
    let fixture = Fixture::new();
    let missing = String::from_utf8(fixture.run(&["routes"]).stdout).unwrap();
    assert!(missing.contains("IPv4 routing table unavailable"));
    assert!(missing.contains("IPv6 routing table unavailable"));
    assert!(!missing.contains("entries observed"));
    assert!(missing.contains("source is not present"));

    fs::write(
        fixture.0.join("net/route"),
        "Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\n",
    )
    .unwrap();
    fs::write(fixture.0.join("net/ipv6_route"), "").unwrap();
    let empty = String::from_utf8(fixture.run(&["routes"]).stdout).unwrap();
    assert!(empty.contains("no IPv4 routing entries observed"));
    assert!(empty.contains("no IPv6 routing entries observed"));
    assert!(!empty.contains("unavailable"));

    fs::write(fixture.0.join("net/route"), "broken\n").unwrap();
    let partial = json(&fixture.run(&["routes", "--json"]));
    assert_eq!(partial["v4_status"]["status"], "error");
    assert_eq!(partial["v6_status"]["status"], "available");
    let partial = String::from_utf8(fixture.run(&["routes"]).stdout).unwrap();
    assert!(partial.contains("procfs/net/route: error — route line lacks required fields"));
    assert!(partial.contains("IPv4 routing table unavailable"));
    assert!(partial.contains("no IPv6 routing entries observed"));

    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let source = fixture.0.join("net/route");
    if fs::metadata(&source).unwrap().uid() != 0 {
        fs::set_permissions(&source, fs::Permissions::from_mode(0o0)).unwrap();
        let denied = fixture.run(&["routes", "--json"]);
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(json(&denied)["v4_status"]["status"], "permission_denied");
    }
}

#[test]
fn interfaces_command_runs_cleanly_on_all_platforms() {
    let output = binary(&["interfaces", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert!(data.get("schema_version").is_some());
    assert!(data.get("status").is_some());
    assert!(data.get("interfaces").is_some());
    assert!(data.get("dropped_interfaces").is_some());
    assert_eq!(data["mode"], "live");
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn interfaces_command_reads_fixture_interfaces() {
    let fixture = Fixture::new();
    let sys_net = fixture.0.join("sys/class/net");
    fs::create_dir_all(sys_net.join("eth0")).unwrap();
    fs::write(sys_net.join("eth0/ifindex"), "2\n").unwrap();
    fs::write(sys_net.join("eth0/mtu"), "1500\n").unwrap();
    fs::write(sys_net.join("eth0/operstate"), "up\n").unwrap();
    fs::write(sys_net.join("eth0/address"), "52:54:00:12:34:56\n").unwrap();
    fs::write(sys_net.join("eth0/speed"), "1000\n").unwrap();
    fs::write(sys_net.join("eth0/flags"), "0x1003\n").unwrap();

    let output = fixture.run(&["interfaces", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["status"]["status"], "available");
    assert_eq!(data["mode"], "fixture");
    assert_eq!(data["schema_version"], "0.1");
    let ifaces = data["interfaces"].as_array().unwrap();
    assert_eq!(ifaces.len(), 1);
    assert_eq!(ifaces[0]["name"], "eth0");
    assert_eq!(ifaces[0]["ifindex"], 2);
    assert_eq!(ifaces[0]["mtu"], 1500);
    assert_eq!(ifaces[0]["oper_state"], "up");
    assert_eq!(ifaces[0]["mac_address"], "52:54:00:12:34:56");
    assert_eq!(ifaces[0]["speed_mbps"], 1000);
    let flags = ifaces[0]["flags"].as_array().unwrap();
    assert!(flags.iter().any(|f| f == "UP"));
    assert!(flags.iter().any(|f| f == "BROADCAST"));
    assert!(flags.iter().any(|f| f == "MULTICAST"));

    let text = String::from_utf8(fixture.run(&["interfaces"]).stdout).unwrap();
    assert!(text.contains("LiDB fixture interfaces"));
    assert!(text.contains("2: eth0: state up <UP,BROADCAST,MULTICAST> mtu 1500 speed 1000Mbps link/ether 52:54:00:12:34:56"));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn interfaces_empty_missing_and_denied_sources_are_distinct() {
    let fixture = Fixture::new();
    let text = String::from_utf8(fixture.run(&["interfaces"]).stdout).unwrap();
    assert!(text.contains("interfaces table unavailable"));
    assert!(text.contains("sysfs network class directory is not present"));

    let sys_net = fixture.0.join("sys/class/net");
    fs::create_dir_all(&sys_net).unwrap();
    let empty = String::from_utf8(fixture.run(&["interfaces"]).stdout).unwrap();
    assert!(empty.contains("no network interfaces observed"));
    assert!(!empty.contains("unavailable"));

    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if fs::metadata(&sys_net).unwrap().uid() != 0 {
        fs::set_permissions(&sys_net, fs::Permissions::from_mode(0o0)).unwrap();
        let denied = fixture.run(&["interfaces", "--json"]);
        fs::set_permissions(&sys_net, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(json(&denied)["status"]["status"], "permission_denied");
    }
}

#[test]
fn sockets_command_runs_cleanly_on_all_platforms() {
    let output = binary(&["sockets", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["schema_version"], "0.1");
    assert!(data.get("tcp_v4_status").is_some());
    assert!(data.get("tcp_v6_status").is_some());
    assert!(data.get("udp_v4_status").is_some());
    assert!(data.get("udp_v6_status").is_some());
    assert!(data.get("sockets").is_some());
    assert!(data.get("dropped_sockets").is_some());
    assert_eq!(data["mode"], "live");
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn sockets_command_reads_fixture_sockets() {
    let fixture = Fixture::new();
    let net_dir = fixture.0.join("net");
    fs::create_dir_all(&net_dir).unwrap();

    let tcp_fixture = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 00000000:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0\n\
";
    let udp_fixture = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode ref pointer drops\n\
   0: 0100007F:0035 00000000:0000 07 00000000:00000000 00:00000000 00000000     0        0 23456 2 0000000000000000 0\n\
";
    let tcp6_fixture = "\
  sl  local_address                         rem_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 00000000000000000000000001000000:0050 00000000000000000000000001000000:01BB 01 00000000:00000000 00:00000000 00000000     0        0 34567 1 0000000000000000 100 0 0 10 0\n\
";
    fs::write(net_dir.join("tcp"), tcp_fixture).unwrap();
    fs::write(net_dir.join("udp"), udp_fixture).unwrap();
    fs::write(net_dir.join("tcp6"), tcp6_fixture).unwrap();

    let output = fixture.run(&["sockets", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["tcp_v4_status"]["status"], "available");
    assert_eq!(data["tcp_v6_status"]["status"], "available");
    assert_eq!(data["udp_v4_status"]["status"], "available");
    assert_eq!(data["mode"], "fixture");
    assert_eq!(data["schema_version"], "0.1");

    let sockets = data["sockets"].as_array().unwrap();
    assert_eq!(sockets.len(), 3);
    assert_eq!(sockets[0]["protocol"], "tcp");
    assert_eq!(sockets[0]["local_port"], 80);
    assert_eq!(sockets[0]["state"], "listen");
    assert_eq!(sockets[0]["inode"], 12345);

    assert_eq!(sockets[1]["protocol"], "tcp");
    assert_eq!(sockets[1]["local_address"], "::1");
    assert_eq!(sockets[1]["local_port"], 80);
    assert_eq!(sockets[1]["remote_address"], "::1");
    assert_eq!(sockets[1]["remote_port"], 443);
    assert_eq!(sockets[1]["inode"], 34567);

    assert_eq!(sockets[2]["protocol"], "udp");
    assert_eq!(sockets[2]["local_address"], "127.0.0.1");
    assert_eq!(sockets[2]["local_port"], 53);
    assert_eq!(sockets[2]["inode"], 23456);

    let text = String::from_utf8(fixture.run(&["sockets"]).stdout).unwrap();
    assert!(text.contains("LiDB fixture sockets"));
    assert!(text.contains("tcp  0.0.0.0:80"));
    assert!(text.contains("state:listen ino:12345"));
    assert!(text.contains("tcp  [::1]:80 -> [::1]:443"));
    assert!(text.contains("state:established ino:34567"));
    assert!(text.contains("udp  127.0.0.1:53"));
    assert!(text.contains("state:closed ino:23456"));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn sockets_empty_missing_and_denied_sources_are_distinct() {
    let fixture = Fixture::new();
    let text = String::from_utf8(fixture.run(&["sockets"]).stdout).unwrap();
    assert!(text.contains("sockets table unavailable"));

    let net_dir = fixture.0.join("net");
    fs::create_dir_all(&net_dir).unwrap();
    fs::write(net_dir.join("tcp"), "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n").unwrap();
    fs::write(net_dir.join("tcp6"), "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n").unwrap();
    fs::write(net_dir.join("udp"), "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode ref pointer drops\n").unwrap();
    fs::write(net_dir.join("udp6"), "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode ref pointer drops\n").unwrap();

    let empty = String::from_utf8(fixture.run(&["sockets"]).stdout).unwrap();
    assert!(empty.contains("no sockets observed"));
    assert!(!empty.contains("sockets table unavailable"));

    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let tcp_path = net_dir.join("tcp");
    if fs::metadata(&tcp_path).unwrap().uid() != 0 {
        fs::set_permissions(&tcp_path, fs::Permissions::from_mode(0o0)).unwrap();
        let denied = fixture.run(&["sockets", "--json"]);
        fs::set_permissions(&tcp_path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            json(&denied)["tcp_v4_status"]["status"],
            "permission_denied"
        );
    }
}

#[test]
fn network_command_runs_cleanly_on_all_platforms() {
    let output = binary(&["network", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["schema_version"], "0.1");
    assert_eq!(data["clock_source"], "collector_monotonic");
    assert!(data.get("interfaces_status").is_some());
    assert!(data.get("routes_status").is_some());
    assert!(data.get("sockets_status").is_some());
    assert!(data.get("interfaces").is_some());
    assert!(data.get("routes").is_some());
    assert!(data.get("sockets").is_some());
    assert!(data.get("dropped_interfaces").is_some());
    assert!(data.get("dropped_routes").is_some());
    assert!(data.get("dropped_sockets").is_some());
    assert_eq!(data["mode"], "live");

    let text_output = binary(&["network"]);
    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains("LiDB live network"));
    assert!(text.contains("--- Interfaces ---"));
    assert!(text.contains("--- Routes ---"));
    assert!(text.contains("--- Sockets ---"));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn network_command_reads_fixture_network() {
    let fixture = Fixture::new();
    let net_dir = fixture.0.join("net");
    fs::create_dir_all(&net_dir).unwrap();

    let dev_fixture = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0:    1000      10    0    0    0     0          0         0     2000      20    0    0    0     0       0          0
";
    fs::write(net_dir.join("dev"), dev_fixture).unwrap();

    let route_v4 = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n\
";
    fs::write(net_dir.join("route"), route_v4).unwrap();

    let tcp_fixture = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
   0: 00000000:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0\n\
";
    fs::write(net_dir.join("tcp"), tcp_fixture).unwrap();

    let output = fixture.run(&["network", "--json"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["mode"], "fixture");
    assert_eq!(data["schema_version"], "0.1");

    let text = String::from_utf8(fixture.run(&["network"]).stdout).unwrap();
    assert!(text.contains("LiDB fixture network"));
    assert!(text.contains("--- Interfaces ---"));
    assert!(text.contains("--- Routes ---"));
    assert!(text.contains("--- Sockets ---"));
}

static EXPORT_TEMP_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[test]
fn export_command_runs_cleanly_on_all_platforms() {
    let output = binary(&["export", "--json", "--interval-ms", "250"]);
    // Exit code 0 if metrics usable, 1 if unavailable (e.g. non-Linux without fixture)
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 1, "unexpected exit code: {code}");
    let data: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["schema_version"], "0.1");
    assert_eq!(data["export_type"], "offline_diagnostic_evidence");
    assert_eq!(data["clock_source"], "collector_monotonic");
    assert!(data["privacy_declaration"]
        .as_str()
        .unwrap()
        .contains("Zero packet payloads"));
    assert_eq!(data["snapshot"]["schema_version"], "0.1");
    assert_eq!(data["doctor"]["schema_version"], "0.1");
    assert_eq!(data["network"]["schema_version"], "0.1");

    let text_output = binary(&["export", "--interval-ms", "250"]);
    let text_code = text_output.status.code().unwrap_or(-1);
    assert!(text_code == 0 || text_code == 1);
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains("LiDB Diagnostic Evidence Bundle v0.1"));
    assert!(text.contains("Export Type: offline_diagnostic_evidence"));
    assert!(text.contains("Zero packet payloads"));
    assert!(text.contains("--- [1/3] Host Metrics Snapshot ---"));
    assert!(text.contains("--- [2/3] Source Doctor Report ---"));
    assert!(text.contains("--- [3/3] Network Telemetry Report ---"));
}

#[test]
fn export_command_writes_to_output_file() {
    let temp_json = std::env::temp_dir().join(format!(
        "lidash-export-{}-{}.json",
        std::process::id(),
        EXPORT_TEMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let output = binary(&[
        "export",
        "--json",
        "--output",
        temp_json.to_str().unwrap(),
        "--interval-ms",
        "250",
    ]);
    let code = output.status.code().unwrap_or(-1);
    assert!(code == 0 || code == 1);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Exported diagnostic evidence to"));
    assert!(temp_json.exists());

    let content = std::fs::read_to_string(&temp_json).unwrap();
    let data: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(data["schema_version"], "0.1");
    assert_eq!(data["export_type"], "offline_diagnostic_evidence");
    assert_eq!(data["clock_source"], "collector_monotonic");
    let _ = std::fs::remove_file(&temp_json);

    let temp_txt = std::env::temp_dir().join(format!(
        "lidash-export-{}-{}.txt",
        std::process::id(),
        EXPORT_TEMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let output_txt = binary(&[
        "export",
        "--output",
        temp_txt.to_str().unwrap(),
        "--interval-ms",
        "250",
    ]);
    let code_txt = output_txt.status.code().unwrap_or(-1);
    assert!(code_txt == 0 || code_txt == 1);
    assert!(temp_txt.exists());
    let txt_content = std::fs::read_to_string(&temp_txt).unwrap();
    assert!(txt_content.contains("LiDB Diagnostic Evidence Bundle v0.1"));
    let _ = std::fs::remove_file(&temp_txt);
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn export_command_reads_fixture_evidence() {
    let fixture = Fixture::new();
    let net_dir = fixture.0.join("net");
    fs::create_dir_all(&net_dir).unwrap();

    let dev_fixture = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0:    1000      10    0    0    0     0          0         0     2000      20    0    0    0     0       0          0
";
    fs::write(net_dir.join("dev"), dev_fixture).unwrap();

    let output = fixture.run(&["export", "--json", "--interval-ms", "250"]);
    assert!(output.status.success());
    let data = json(&output);
    assert_eq!(data["mode"], "fixture");
    assert_eq!(data["schema_version"], "0.1");
    assert_eq!(data["export_type"], "offline_diagnostic_evidence");
    assert_eq!(data["doctor"]["usable"], true);
    assert_eq!(data["doctor"]["mode"], "fixture");
    assert_eq!(data["snapshot"]["mode"], "fixture");

    let text = String::from_utf8(fixture.run(&["export", "--interval-ms", "250"]).stdout).unwrap();
    assert!(text.contains("LiDB Diagnostic Evidence Bundle v0.1"));
    assert!(text.contains("Mode: fixture"));
    assert!(text.contains("--- [1/3] Host Metrics Snapshot ---"));
    assert!(text.contains("--- [2/3] Source Doctor Report ---"));
    assert!(text.contains("--- [3/3] Network Telemetry Report ---"));
}
