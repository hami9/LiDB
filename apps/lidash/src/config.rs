use std::{ffi::OsString, path::PathBuf, time::Duration};

pub const HELP: &str = "LiDashBoard — read-only Linux host monitor
Usage: lidash [snapshot|doctor|tui] [OPTIONS]

Commands:
  snapshot       Print one host snapshot (alias: status)
  doctor         Report observed source capabilities (alias: capabilities)
  tui            Open the keyboard-driven terminal monitor

Options:
  --json                 JSON snapshot or doctor report; no terminal required
  --interval-ms N        Sample interval, 250..60000 milliseconds
  --proc-root PATH       Read a fixture instead of /proc; output is labeled fixture
  --no-color             Monochrome output (also the default)
  -h, --help             Print help
  -V, --version          Print version

Without a command: open TUI when stdin and stdout are terminals, otherwise snapshot.
Snapshot/doctor take two samples (default 250ms); TUI defaults to 1000ms.
Doctor is read-only. Exit: 0 usable (possibly degraded), 1 no available metrics,
2 invalid arguments or runtime failure. Snapshot preserves unavailable states.
TUI keys: q/Esc/Ctrl-C quit, Tab/1/2 switch tabs, Space pause, ?/h help, Enter detail, arrows scroll.
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Snapshot,
    Doctor,
    Tui,
    Help,
    Version,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub command: Command,
    pub json: bool,
    pub interval: Duration,
    pub proc_root: PathBuf,
    // Explicit --proc-root /proc is still a fixture request.
    pub fixture: bool,
}

impl Config {
    pub fn parse(
        args: impl IntoIterator<Item = OsString>,
        interactive: bool,
    ) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut command = None;
        let mut json = false;
        let mut interval = None;
        let mut proc_root = PathBuf::from("/proc");
        let mut fixture = false;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("-h" | "--help" | "help") => {
                    return Ok(Self::special(Command::Help));
                }
                Some("-V" | "--version") => return Ok(Self::special(Command::Version)),
                Some("snapshot" | "status") => set_command(&mut command, Command::Snapshot)?,
                Some("doctor" | "capabilities") => set_command(&mut command, Command::Doctor)?,
                Some("tui") => set_command(&mut command, Command::Tui)?,
                Some("--json") => json = true,
                Some("--no-color") => {}
                Some("--interval-ms") => {
                    let value = args.next().ok_or("--interval-ms requires a number")?;
                    let ms: u64 = value
                        .to_str()
                        .and_then(|text| text.parse().ok())
                        .filter(|value| (250..=60_000).contains(value))
                        .ok_or("--interval-ms must be an integer in 250..60000")?;
                    interval = Some(Duration::from_millis(ms));
                }
                Some("--proc-root") => {
                    proc_root = args.next().ok_or("--proc-root requires a path")?.into();
                    if proc_root.as_os_str().is_empty() {
                        return Err("--proc-root requires a nonempty path".into());
                    }
                    fixture = true;
                }
                _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
            }
        }
        let command = command.unwrap_or(if interactive && !json {
            Command::Tui
        } else {
            Command::Snapshot
        });
        if command == Command::Tui && json {
            return Err(
                "--json is supported by snapshot and doctor; choose one of those commands".into(),
            );
        }
        Ok(Self {
            command,
            json,
            interval: interval.unwrap_or(Duration::from_millis(if command == Command::Tui {
                1000
            } else {
                250
            })),
            proc_root,
            fixture,
        })
    }

    fn special(command: Command) -> Self {
        Self {
            command,
            json: false,
            interval: Duration::from_millis(250),
            proc_root: "/proc".into(),
            fixture: false,
        }
    }
}

fn set_command(current: &mut Option<Command>, command: Command) -> Result<(), String> {
    if current.replace(command).is_some() {
        return Err("choose only one command".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str], tty: bool) -> Result<Config, String> {
        Config::parse(args.iter().map(OsString::from), tty)
    }

    #[test]
    fn defaults_and_output_selection_respect_tty_and_json() {
        assert_eq!(parse(&[], true).unwrap().command, Command::Tui);
        let headless = parse(&[], false).unwrap();
        assert_eq!(headless.command, Command::Snapshot);
        assert_eq!(headless.interval, Duration::from_millis(250));
        assert_eq!(parse(&["--json"], true).unwrap().command, Command::Snapshot);
        assert!(parse(&["tui", "--json"], true).is_err());
    }

    #[test]
    fn rejects_invalid_ranges_and_ambiguous_commands() {
        for value in ["0", "249", "60001", "-1", "x"] {
            assert!(parse(&["--interval-ms", value], false).is_err());
        }
        for args in [
            &["--proc-root"][..],
            &["--interval-ms"],
            &["tui", "doctor"],
            &["--demo"],
        ] {
            assert!(parse(args, false).is_err());
        }
        let fixture = parse(&["--proc-root", "/proc", "snapshot"], false).unwrap();
        assert!(fixture.fixture);
    }
}
