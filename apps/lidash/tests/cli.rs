//! Exercising the actual executable from integration tests.
use std::process::Command;

fn execute(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lidash"))
        .args(args)
        .output()
        .expect("lidash binary should be launchable");
    (
        output.status.success(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn cli_help_and_version_work_without_privileges() {
    let (success, stdout, _) = execute(&["--help"]);
    assert!(success);
    assert!(stdout.contains("P0 foundation"));
    let (success, stdout, _) = execute(&["--version"]);
    assert!(success);
    assert!(stdout.contains("bootstrap"));
}

#[test]
fn json_status_explicitly_lacks_collectors() {
    let (success, stdout, _) = execute(&["status", "--json"]);
    assert!(success);
    assert!(stdout.contains("\"collector_running\":false"));
    let (success, stdout, _) = execute(&["capabilities", "--json"]);
    assert!(success);
    assert!(stdout.contains("org.lidb.ai.gpu"));
    assert!(!stdout.contains("\"status\":\"available\""));
}

#[test]
fn unsupported_command_fails() {
    let (success, _, stderr) = execute(&["start", "--enable-all"]);
    assert!(!success);
    assert!(stderr.contains("unknown command"));
}
