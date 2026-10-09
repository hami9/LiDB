//! Local, non-root Linux host monitor. No daemon or listener is installed.
#![forbid(unsafe_code)]

use lidash::{
    config::{Command, Config},
    execute,
};
use std::{
    env,
    io::{self, IsTerminal},
    process::ExitCode,
};

fn main() -> ExitCode {
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let config = match Config::parse(env::args_os().skip(1), interactive) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("lidash: {error}\nUse --help for usage.");
            return ExitCode::from(2);
        }
    };
    if config.command == Command::Tui && !interactive {
        eprintln!(
            "lidash: tui requires terminal stdin and stdout; use snapshot or snapshot --json"
        );
        return ExitCode::from(2);
    }
    // TUI owns stdout while drawing. Headless writes propagate pipe failures.
    match execute(config, io::stdout()) {
        Ok(code) => ExitCode::from(code),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lidash: {error}");
            ExitCode::from(2)
        }
    }
}
