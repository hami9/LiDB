use super::*;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
use std::fs;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

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
        let path = std::env::temp_dir().join(format!(
            "lidb-collect-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn baseline(&self) {
        self.write("stat", "cpu 100 20 30 400 50 10 10 5 60 10\ncpu0 0 0 0 0\n");
        self.write(
            "meminfo",
            "MemTotal: 1024 kB\nMemAvailable: 256 kB\nSwapTotal: 32 kB\nSwapFree: 8 kB\n",
        );
        self.write("loadavg", "0.10 0.20 0.30 1/10 1234\n");
        self.write("uptime", "123.45 250.00\n");
        self.write("net/dev", &network_line("eth0", 100, 200));
        self.write("diskstats", "8 0 sda 1 0 10 0 1 0 20 0 0 0 0\n");
        for kind in ["cpu", "memory", "io"] {
            self.write(
                &format!("pressure/{kind}"),
                "some avg10=1.20 avg60=0.50 avg300=0.20 total=123\n",
            );
        }
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn network_line(name: &str, rx: u64, tx: u64) -> String {
    format!(
        "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n{name}: {rx} 0 0 0 0 0 0 0 {tx} 0 0 0 0 0 0 0\n"
    )
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn state<'a>(snapshot: &'a Snapshot, name: &str) -> &'a State {
    snapshot.get(name).unwrap().state()
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn exact(snapshot: &Snapshot, name: &str) -> u64 {
    match state(snapshot, name) {
        MetricState::Available(MetricValue::Integer(value)) => *value,
        other => panic!("expected exact value for {name}, received {other:?}"),
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn fractional(snapshot: &Snapshot, name: &str) -> f64 {
    state(snapshot, name).available_value().unwrap().as_f64()
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn fixture_sources_have_exact_values_and_common_timestamps() {
    let fixture = Fixture::new();
    fixture.baseline();
    let mut collector = LinuxCollector::new(&fixture.0);
    let snapshot = collector.sample_at(1_000_000_000);
    assert_eq!(snapshot.mode(), SnapshotMode::Fixture);
    assert_eq!(exact(&snapshot, "cpu.guest.ticks"), 60);
    assert_eq!(exact(&snapshot, "memory.total.bytes"), 1024 * 1024);
    assert_eq!(exact(&snapshot, "memory.available.bytes"), 256 * 1024);
    assert_eq!(exact(&snapshot, "memory.used.bytes"), 768 * 1024);
    assert_eq!(exact(&snapshot, "swap.used.bytes"), 24 * 1024);
    assert_eq!(exact(&snapshot, "disk.sda.read.bytes"), 10 * 512);
    assert_eq!(exact(&snapshot, "network.eth0.tx.bytes"), 200);
    assert_eq!(fractional(&snapshot, "load.5m"), 0.20);
    assert_eq!(fractional(&snapshot, "uptime.seconds"), 123.45);
    assert_eq!(
        fractional(&snapshot, "pressure.cpu.some.avg10.percent"),
        1.20
    );
    for name in [
        "cpu.busy.percent",
        "network.eth0.rx.bytes_per_second",
        "disk.sda.read.bytes_per_second",
    ] {
        assert_eq!(state(&snapshot, name).name(), "temporarily_unavailable");
    }
    assert!(snapshot
        .metrics()
        .iter()
        .all(|metric| metric.monotonic_ns() == 1_000_000_000));
    assert_eq!(snapshot.dropped_metrics(), 0);
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn cpu_and_transfer_deltas_exclude_guest_double_counting_and_iowait() {
    let fixture = Fixture::new();
    fixture.baseline();
    let mut collector = LinuxCollector::new(&fixture.0);
    collector.sample_at(1_000_000_000);
    // Busy delta: 20+10+10+5+5+10=60. Idle+iowait delta: 20+20=40.
    // Guest ticks are included in user/nice; adding them would give 66.6%.
    fixture.write("stat", "cpu 120 30 40 420 70 15 15 15 80 30\n");
    fixture.write("net/dev", &network_line("eth0", 160, 280));
    fixture.write("diskstats", "8 0 sda 2 0 14 0 2 0 26 0 0 0 0\n");
    let snapshot = collector.sample_at(3_000_000_000);
    assert_eq!(fractional(&snapshot, "cpu.busy.percent"), 60.0);
    assert_eq!(
        fractional(&snapshot, "network.eth0.rx.bytes_per_second"),
        30.0
    );
    assert_eq!(
        fractional(&snapshot, "network.eth0.tx.bytes_per_second"),
        40.0
    );
    assert_eq!(
        fractional(&snapshot, "disk.sda.read.bytes_per_second"),
        1024.0
    );
    assert_eq!(
        fractional(&snapshot, "disk.sda.write.bytes_per_second"),
        1536.0
    );
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn counter_resets_and_gaps_clear_rates_without_fabricating_zero() {
    let fixture = Fixture::new();
    fixture.baseline();
    let mut collector = LinuxCollector::new(&fixture.0);
    collector.sample_at(1_000_000_000);
    fixture.write("stat", "cpu 1 1 1 1 1 1 1 1 0 0\n");
    fixture.write("net/dev", &network_line("eth0", 5, 10));
    let reset = collector.sample_at(2_000_000_000);
    assert_eq!(state(&reset, "cpu.busy.percent").name(), "error");
    assert_eq!(
        state(&reset, "network.eth0.rx.bytes_per_second").name(),
        "error"
    );
    assert_eq!(exact(&reset, "network.eth0.rx.bytes"), 5);
    fixture.write("stat", "cpu 2 2 2 2 2 2 2 2 0 0\n");
    fixture.write("net/dev", &network_line("eth0", 15, 20));
    let recovered = collector.sample_at(3_000_000_000);
    assert_eq!(fractional(&recovered, "cpu.busy.percent"), 75.0);
    assert_eq!(
        fractional(&recovered, "network.eth0.rx.bytes_per_second"),
        10.0
    );
    fixture.write("stat", "cpu malformed\n");
    fs::remove_file(fixture.0.join("net/dev")).unwrap();
    let missing = collector.sample_at(4_000_000_000);
    assert_eq!(state(&missing, "cpu.user.ticks").name(), "error");
    assert_eq!(state(&missing, "network.state").name(), "unsupported");
    assert_eq!(exact(&missing, "memory.total.bytes"), 1024 * 1024);
    fixture.baseline();
    let fresh = collector.sample_at(5_000_000_000);
    assert_eq!(
        state(&fresh, "cpu.busy.percent").name(),
        "temporarily_unavailable"
    );
    assert_eq!(
        state(&fresh, "network.eth0.rx.bytes_per_second").name(),
        "temporarily_unavailable"
    );
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn missing_memavailable_has_no_free_memory_substitution() {
    let fixture = Fixture::new();
    fixture.write(
        "meminfo",
        "MemTotal: 1024 kB\nMemFree: 100 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n",
    );
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(
        state(&snapshot, "memory.available.bytes").name(),
        "unsupported"
    );
    assert_eq!(
        state(&snapshot, "memory.used.bytes").available_value(),
        None
    );
    assert_eq!(exact(&snapshot, "swap.used.bytes"), 0);
    assert_eq!(
        state(&snapshot, "pressure.cpu.some.avg10.percent").name(),
        "unsupported"
    );
    fixture.write("meminfo", "MemTotal: 1 kB\nMemAvailable: 2 kB\n");
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(state(&snapshot, "memory.used.bytes").name(), "error");
}

#[test]
fn negative_malformed_nonfinite_and_overflow_inputs_are_rejected() {
    for text in [
        "cpu -1 1 1 1",
        "cpu 18446744073709551616 0 0 0",
        "cpu 18446744073709551615 1 0 0",
        "cpu 1 2",
        "cpu 1 2 3 4\ncpu 1 2 3 4",
    ] {
        assert!(parse::cpu(text).is_err(), "{text}");
    }
    for text in [
        "-1 0 0 0/1 1",
        "NaN 0 0 0/1 1",
        "inf 0 0 0/1 1",
        "0 0 0 2/1 1",
        "0 0 0 0/1 1 extra",
    ] {
        assert!(parse::load(text).is_err(), "{text}");
    }
    for text in ["-0 1", "-1 1", "NaN 1", "0", "0 1 extra"] {
        assert!(parse::uptime(text).is_err(), "{text}");
    }
    for text in [
        "MemTotal: -1 kB",
        "MemTotal: 18446744073709551615 kB",
        "MemTotal: 1 MB",
        "MemTotal: 1 kB\nMemTotal: 2 kB",
        "MemTotal 1 kB",
    ] {
        assert_eq!(parse::memory(text)["MemTotal"].name(), "error");
    }
    assert!(parse::disks("8 0 sda 1 0 18446744073709551615 0 1 0 0 0 0 0 0").is_err());
    assert!(parse::disks("8 0 sda 1 0 -1 0 1 0 0 0 0 0 0").is_err());
    for text in [
        "some avg10=101 avg60=0 avg300=0 total=0",
        "some avg10=NaN avg60=0 avg300=0 total=0",
        "some avg10=0 avg60=0 avg60=0 total=0",
        "some avg10=0 avg60=0 avg300=0 total=-1",
    ] {
        assert!(parse::pressure(text).is_err(), "{text}");
    }
}

#[test]
fn device_counts_duplicates_and_names_are_bounded() {
    let network = network_line("eth0", u64::MAX, 0);
    assert_eq!(parse::network(&network).unwrap()["eth0"].0[0], u64::MAX);
    for name in ["bad/name", "bad name", "bad\u{1b}[2J", "a:b"] {
        assert!(parse::network(&network_line(name, 1, 2)).is_err());
    }
    let mut network = network_line("eth0", 1, 2);
    network.push_str("eth0: 1 0 0 0 0 0 0 0 2 0 0 0 0 0 0 0\n");
    assert!(parse::network(&network).is_err());
    let mut disks = String::new();
    for index in 0..MAX_DEVICES {
        disks.push_str(&format!("8 {index} disk{index} 1 0 1 0 1 0 1 0 0 0 0\n"));
    }
    let (kept, omitted) = parse::keep_devices(parse::disks(&disks).unwrap(), &["loop"]);
    assert_eq!((kept.len(), omitted), (MAX_DEVICES, 0));
    disks.push_str("8 99 overflow 1 0 1 0 1 0 1 0 0 0 0\n");
    let (kept, omitted) = parse::keep_devices(parse::disks(&disks).unwrap(), &["loop"]);
    assert_eq!((kept.len(), omitted), (MAX_DEVICES, 1));

    // Many loop devices must not hide a physical disk.
    let mut disks = String::new();
    for index in 0..MAX_DEVICES + 10 {
        disks.push_str(&format!("7 {index} loop{index} 1 0 1 0 1 0 1 0 0 0 0\n"));
    }
    disks.push_str("8 0 sda 1 0 1 0 1 0 1 0 0 0 0\n");
    let (kept, omitted) = parse::keep_devices(parse::disks(&disks).unwrap(), &["loop"]);
    assert!(kept.contains_key("sda"));
    assert_eq!((kept.len(), omitted), (MAX_DEVICES, 11));

    let punctuation = network_line("eth+0", 1, 2);
    assert!(parse::network(&punctuation)
        .unwrap()
        .contains_key("eth%2B0"));
    let percent = network_line("eth%2B0", 1, 2);
    assert!(parse::network(&percent).unwrap().contains_key("eth%252B0"));
    let disk = "8 0 cciss!c0d0 1 0 1 0 1 0 1 0 0 0 0";
    assert!(parse::disks(disk).unwrap().contains_key("cciss%21c0d0"));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn kernel_valid_punctuation_preserves_other_device_observations() {
    let fixture = Fixture::new();
    fixture.baseline();
    let mut network = network_line("eth0", 1, 2);
    network.push_str("vpn+prod: 3 0 0 0 0 0 0 0 4 0 0 0 0 0 0 0\n");
    network.push_str("vpn%2Bprod: 5 0 0 0 0 0 0 0 6 0 0 0 0 0 0 0\n");
    network.push_str("vpn@prod: 7 0 0 0 0 0 0 0 8 0 0 0 0 0 0 0\n");
    fixture.write("net/dev", &network);
    fixture.write(
        "diskstats",
        "8 0 sda 1 0 10 0 1 0 20 0 0 0 0\n8 1 cciss!c0d0 1 0 30 0 1 0 40 0 0 0 0\n",
    );
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(exact(&snapshot, "network.state"), 4);
    assert_eq!(exact(&snapshot, "network.eth0.rx.bytes"), 1);
    assert_eq!(exact(&snapshot, "network.vpn%2Bprod.rx.bytes"), 3);
    assert_eq!(exact(&snapshot, "network.vpn%252Bprod.rx.bytes"), 5);
    assert_eq!(exact(&snapshot, "network.vpn%40prod.rx.bytes"), 7);
    assert_eq!(exact(&snapshot, "disk.state"), 2);
    assert_eq!(exact(&snapshot, "disk.omitted"), 0);
    assert_eq!(exact(&snapshot, "disk.sda.read.bytes"), 10 * 512);
    assert_eq!(exact(&snapshot, "disk.cciss%21c0d0.read.bytes"), 30 * 512);

    let mut network = network_line(&"n".repeat(100), 1, 2);
    network.push_str("\u{03BB}: 3 0 0 0 0 0 0 0 4 0 0 0 0 0 0 0\n");
    fixture.write("net/dev", &network);
    fixture.write(
        "diskstats",
        &format!("8 0 {} 1 0 10 0 1 0 20 0 0 0 0\n", "d".repeat(100)),
    );
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(exact(&snapshot, "network.state"), 2);
    assert_eq!(exact(&snapshot, "disk.state"), 1);
    assert_eq!(
        snapshot
            .metrics()
            .iter()
            .map(|metric| metric.name().len())
            .max(),
        Some(128)
    );
    assert!(snapshot
        .metrics()
        .iter()
        .all(|metric| metric.name().is_ascii()));
}

#[test]
fn older_cpu_fields_and_zero_intervals_remain_explicit() {
    let older = parse::cpu("cpu 1 2 3 4").unwrap();
    assert_eq!(cpu_busy(None, &older).name(), "unsupported");
    let current = parse::cpu("cpu 1 2 3 4 5 6 7 8 0 0").unwrap();
    assert_eq!(
        cpu_busy(Some(&current), &current).name(),
        "temporarily_unavailable"
    );
    let counters = TransferCounters([1, 2]);
    let previous = Some((1, BTreeMap::from([("eth0".into(), counters.clone())])));
    assert_eq!(
        transfer_rates(&previous, 1, "eth0", &counters)[0].name(),
        "temporarily_unavailable"
    );

    let previous = parse::cpu("cpu 0 0 0 0 0 0 0 0 0 0").unwrap();
    let current = parse::cpu("cpu 12563368808249168529 0 0 0 0 0 0 0 0 0").unwrap();
    assert_eq!(cpu_busy(Some(&previous), &current), number(100.0));

    for key in ["avg10", "avg60", "avg300", "total"] {
        let duplicate = format!("some avg10=1 avg60=2 avg300=3 total=4 {key}=5");
        assert!(parse::pressure(&duplicate).is_err());
        let incomplete = format!("some {key}=1");
        assert!(parse::pressure(&incomplete).is_err());
    }
    assert_eq!(parse::memory("")["MemTotal"].name(), "error");
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn nonadvancing_batch_clocks_do_not_publish_rates() {
    let fixture = Fixture::new();
    fixture.baseline();
    let mut collector = LinuxCollector::new(&fixture.0);
    collector.sample_at(2_000_000_000);
    fixture.write("stat", "cpu 120 30 40 420 70 15 15 15 80 30\n");
    fixture.write("net/dev", &network_line("eth0", 160, 280));
    for timestamp in [2_000_000_000, 1_000_000_000] {
        let snapshot = collector.sample_at(timestamp);
        for name in ["cpu.busy.percent", "network.eth0.rx.bytes_per_second"] {
            assert_eq!(state(&snapshot, name).name(), "temporarily_unavailable");
        }
        assert_eq!(exact(&snapshot, "cpu.user.ticks"), 120);
    }
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn reads_reject_oversized_terminal_and_symlink_inputs() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fixture.baseline();
    fixture.write("stat", &"x".repeat(MAX_SOURCE_BYTES + 1));
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(state(&snapshot, "cpu.user.ticks").name(), "error");
    fixture.write("stat", "cpu 1 2 3 4\u{1b}[2J");
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(state(&snapshot, "cpu.user.ticks").name(), "error");
    fs::remove_file(fixture.0.join("stat")).unwrap();
    symlink("/proc/stat", fixture.0.join("stat")).unwrap();
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(state(&snapshot, "cpu.user.ticks").name(), "error");
    fs::remove_dir_all(fixture.0.join("net")).unwrap();
    symlink("/proc/net", fixture.0.join("net")).unwrap();
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(state(&snapshot, "network.state").name(), "error");
    assert_eq!(state(&snapshot, "network.omitted").name(), "error");
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn fifo_sources_are_rejected_without_waiting_for_a_writer() {
    let fixture = Fixture::new();
    let status = std::process::Command::new("mkfifo")
        .arg(fixture.0.join("stat"))
        .status()
        .expect("Linux fixture tests require mkfifo");
    assert!(status.success());
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(state(&snapshot, "cpu.user.ticks").name(), "error");
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn denied_file_does_not_prevent_other_sources_from_being_collected() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.baseline();
    let path = fixture.0.join("stat");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&path).is_ok() {
        // A root/capability-enabled test runner can bypass the fixture denial.
        // The deterministic permission mapping test still covers that branch.
        eprintln!(
            "permission fixture bypassed by the test runner; filesystem denial not exercised"
        );
        return;
    }
    let snapshot = LinuxCollector::new(&fixture.0).sample();
    assert_eq!(
        state(&snapshot, "cpu.user.ticks").name(),
        "permission_denied"
    );
    assert_eq!(state(&snapshot, "cpu.busy.percent").available_value(), None);
    assert_eq!(exact(&snapshot, "memory.total.bytes"), 1024 * 1024);
    assert_eq!(exact(&snapshot, "network.eth0.rx.bytes"), 100);
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn live_linux_smoke_reads_real_proc_and_batches_share_a_clock() {
    let mut collector = LinuxCollector::new("/proc");
    let first = collector.sample();
    assert_eq!(first.mode(), SnapshotMode::Live);
    assert!(exact(&first, "memory.total.bytes") > 0);
    assert!(exact(&first, "cpu.user.ticks") > 0);
    assert!(fractional(&first, "uptime.seconds") > 0.0);
    assert!(first
        .metrics()
        .iter()
        .all(|metric| metric.monotonic_ns() == first.monotonic_ns()));
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = collector.sample();
    assert!(second.monotonic_ns() > first.monotonic_ns());
    assert!((0.0..=100.0).contains(&fractional(&second, "cpu.busy.percent")));
    assert!(second.collection_duration_ns() > 0);
    assert_eq!(
        LinuxCollector::new("/proc")
            .with_fixture_mode()
            .sample()
            .mode(),
        SnapshotMode::Fixture
    );
}

#[test]
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
fn other_platforms_report_unsupported_without_reading_fixtures() {
    let snapshot = LinuxCollector::new("/proc").sample();
    assert!(snapshot
        .metrics()
        .iter()
        .all(|metric| metric.state().name() == "unsupported"));
}
