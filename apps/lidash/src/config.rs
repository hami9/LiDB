use std::{ffi::OsString, path::PathBuf, time::Duration};

pub const ASCII_LOGO: &str = r#"888      d8b 8888888b.                 888     888888b.                                    888
888      Y8P 888  "Y88b                888     888  "88b                                   888
888          888    888                888     888  .88P                                   888
888      888 888    888  8888b.   .d8888b  88888b.  8888888K.   .d88b.   8888b.  888d888  .d88888
888      888 888    888      "88b 88K      888 "88b 888  "Y88b d88""88b      "88b 888P"   d88" 888
888      888 888    888 .d888888 "Y8888b. 888  888 888    888 888  888 .d888888 888     888  888
888      888 888  .d88P 888  888      X88 888  888 888   d88P Y88..88P 888  888 888     Y88b 888
88888888 888 8888888P"  "Y888888  88888P' 888  888 8888888P"   "Y88P"  "Y888888 888      "Y88888"#;

pub const HELP: &str = r#"888      d8b 8888888b.                 888     888888b.                                    888
888      Y8P 888  "Y88b                888     888  "88b                                   888
888          888    888                888     888  .88P                                   888
888      888 888    888  8888b.   .d8888b  88888b.  8888888K.   .d88b.   8888b.  888d888  .d88888
888      888 888    888      "88b 88K      888 "88b 888  "Y88b d88""88b      "88b 888P"   d88" 888
888      888 888    888 .d888888 "Y8888b. 888  888 888    888 888  888 .d888888 888     888  888
888      888 888  .d88P 888  888      X88 888  888 888   d88P Y88..88P 888  888 888     Y88b 888
88888888 888 8888888P"  "Y888888  88888P' 888  888 8888888P"   "Y88P"  "Y888888 888      "Y88888

LiDashBoard — read-only Linux host monitor
Usage: lidash [snapshot|doctor|routes|interfaces|sockets|network|export|tui] [OPTIONS]

Commands:
  snapshot       Print one host snapshot (alias: status)
  doctor         Report observed source capabilities (alias: capabilities)
  routes         Print routing table entries (alias: route)
  interfaces     Print network interface metadata (alias: interface, iface)
  sockets        Print TCP and UDP socket endpoints (alias: socket, sock)
  network        Print network report (alias: net)
  export         Export offline diagnostic evidence bundle (alias: bundle)
  tui            Open the keyboard-driven terminal monitor

Options:
  --json                 JSON output; no terminal required
  --output PATH          Write export bundle to a specified file
  --interval-ms N        Sample interval, 250..60000 milliseconds
  --proc-root PATH       Read a fixture instead of /proc; output is labeled fixture
  --sys-root PATH        Read a fixture instead of /sys; output is labeled fixture
  --no-color             Monochrome output (also the default)
  -h, --help             Print help
  -V, --version          Print version

Without a command: open TUI when stdin and stdout are terminals, otherwise snapshot.
Snapshot/doctor take two samples (default 250ms); TUI defaults to 1000ms.
Doctor is read-only. Exit: 0 usable (possibly degraded), 1 no available metrics,
2 invalid arguments or runtime failure. Snapshot preserves unavailable states.
TUI keys: q/Esc/Ctrl-C quit, Tab/1/2/3/Left/Right tabs, Space pause, [ / ] history, ?/h help, Enter detail, Up/Down scroll.
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Snapshot,
    Doctor,
    Routes,
    Interfaces,
    Sockets,
    Network,
    Export,
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
    pub sys_root: PathBuf,
    // Explicit --proc-root or --sys-root is a fixture request.
    pub fixture: bool,
    pub output_file: Option<PathBuf>,
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
        let mut sys_root: Option<PathBuf> = None;
        let mut fixture = false;
        let mut output_file: Option<PathBuf> = None;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("-h" | "--help" | "help") => {
                    return Ok(Self::special(Command::Help));
                }
                Some("-V" | "--version") => return Ok(Self::special(Command::Version)),
                Some("snapshot" | "status") => set_command(&mut command, Command::Snapshot)?,
                Some("doctor" | "capabilities") => set_command(&mut command, Command::Doctor)?,
                Some("routes" | "route") => set_command(&mut command, Command::Routes)?,
                Some("interfaces" | "interface" | "iface") => {
                    set_command(&mut command, Command::Interfaces)?
                }
                Some("sockets" | "socket" | "sock") => set_command(&mut command, Command::Sockets)?,
                Some("network" | "net") => set_command(&mut command, Command::Network)?,
                Some("export" | "bundle") => set_command(&mut command, Command::Export)?,
                Some("tui") => set_command(&mut command, Command::Tui)?,
                Some("--json") => json = true,
                Some("--output") => {
                    let path: PathBuf = args.next().ok_or("--output requires a path")?.into();
                    if path.as_os_str().is_empty() {
                        return Err("--output requires a nonempty path".into());
                    }
                    output_file = Some(path);
                }
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
                Some("--sys-root") => {
                    let path: PathBuf = args.next().ok_or("--sys-root requires a path")?.into();
                    if path.as_os_str().is_empty() {
                        return Err("--sys-root requires a nonempty path".into());
                    }
                    sys_root = Some(path);
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
                "--json is supported by snapshot, doctor, routes, interfaces, sockets, network, and export; choose one of those commands"
                    .into(),
            );
        }
        if output_file.is_some() && command != Command::Export {
            return Err(
                "--output is supported only by the export command; use redirection for other commands"
                    .into(),
            );
        }
        let sys_root = sys_root.unwrap_or_else(|| {
            if fixture {
                if proc_root.join("sys").is_dir() {
                    proc_root.join("sys")
                } else {
                    proc_root.clone()
                }
            } else {
                PathBuf::from("/sys")
            }
        });
        Ok(Self {
            command,
            json,
            interval: interval.unwrap_or(Duration::from_millis(if command == Command::Tui {
                1000
            } else {
                250
            })),
            proc_root,
            sys_root,
            fixture,
            output_file,
        })
    }

    fn special(command: Command) -> Self {
        Self {
            command,
            json: false,
            interval: Duration::from_millis(250),
            proc_root: "/proc".into(),
            sys_root: "/sys".into(),
            fixture: false,
            output_file: None,
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
        assert_eq!(parse(&["routes"], false).unwrap().command, Command::Routes);
        assert_eq!(
            parse(&["routes", "--json"], false).unwrap().command,
            Command::Routes
        );
        assert_eq!(
            parse(&["interfaces"], false).unwrap().command,
            Command::Interfaces
        );
        assert_eq!(
            parse(&["interface"], false).unwrap().command,
            Command::Interfaces
        );
        assert_eq!(
            parse(&["iface", "--json"], false).unwrap().command,
            Command::Interfaces
        );
        assert_eq!(
            parse(&["sockets"], false).unwrap().command,
            Command::Sockets
        );
        assert_eq!(parse(&["socket"], false).unwrap().command, Command::Sockets);
        assert_eq!(
            parse(&["sock", "--json"], false).unwrap().command,
            Command::Sockets
        );
        assert_eq!(
            parse(&["network"], false).unwrap().command,
            Command::Network
        );
        assert_eq!(parse(&["net"], false).unwrap().command, Command::Network);
        assert_eq!(
            parse(&["net", "--json"], false).unwrap().command,
            Command::Network
        );
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
