//! Linux PTY regression fixtures. Injection and environment flags exist only in tests.

use crate::{
    config::Config,
    worker::{SAMPLE_TIMEOUT, SHUTDOWN_GRACE},
};
use std::{env, ffi::OsString, path::PathBuf, process::Command, thread};

#[test]
fn blocked_sampler_restores_terminal_on_quit_and_watchdog() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/blocked_pty.py");
    let output = Command::new("python3")
        .arg(script)
        .arg(env::current_exe().unwrap())
        .arg(SAMPLE_TIMEOUT.as_secs_f64().to_string())
        .arg(SHUTDOWN_GRACE.as_secs_f64().to_string())
        .output()
        .expect("Python 3 is required for the Linux terminal lifecycle regression");
    assert!(
        output.status.success(),
        "blocked-sampler PTY regression failed ({:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Separate processes prevent a deliberately abandoned sampler from affecting other tests.
/// Only the Python fixture selects this ignored helper and supplies its disposable marker.
#[test]
#[ignore = "subprocess fixture for the Linux blocked-sampler PTY regression"]
fn blocked_tui_child() {
    let ready = env::var_os("LIDB_TEST_BLOCK_READY")
        .expect("run this ignored fixture through tests/blocked_pty.py");
    let config = Config::parse(
        [
            "tui",
            "--interval-ms",
            "60000",
            "--proc-root",
            "injected-sampler",
        ]
        .map(OsString::from),
        true,
    )
    .unwrap();
    let result = crate::tui::run_with_sampler(config, move || {
        std::fs::write(&ready, b"sampler entered\n").expect("write disposable ready marker");
        // A deterministic in-flight callback, not a real blocked procfs/kernel read.
        loop {
            thread::park();
        }
    });
    // Match the CLI runtime-error contract after TUI teardown, without an expected panic.
    let code = match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("lidash: {error}");
            2
        }
    };
    std::process::exit(code);
}
