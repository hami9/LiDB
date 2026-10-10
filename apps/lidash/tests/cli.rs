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
            ("net/dev", "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n eth0: 1000 2 0 0 0 0 0 0 2000 2 0 0 0 0 0 0\n"),
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
}
