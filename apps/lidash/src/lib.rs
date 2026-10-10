//! Read-only host snapshots, source diagnostics, and terminal presentation.
#![forbid(unsafe_code)]

pub mod config;
pub mod output;
pub mod tui;
pub mod worker;

use config::{Command, Config};
use lidb_collect::LinuxCollector;
use std::{
    io::{self, Write},
    thread,
};

pub fn collector(config: &Config) -> LinuxCollector {
    let collector = LinuxCollector::new(config.proc_root.clone());
    if config.fixture {
        collector.with_fixture_mode()
    } else {
        collector
    }
}

pub fn execute(config: Config, mut output: impl Write) -> Result<u8, io::Error> {
    match config.command {
        Command::Help => {
            output.write_all(config::HELP.as_bytes())?;
        }
        Command::Version => {
            writeln!(
                output,
                "lidash {}",
                option_env!("LIDB_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
            )?;
        }
        Command::Tui => {
            tui::run(config)?;
        }
        Command::Snapshot | Command::Doctor => {
            let mut collector = collector(&config);
            let _ = collector.sample();
            thread::sleep(config.interval);
            let snapshot = collector.sample();
            if config.command == Command::Doctor {
                let report = output::DoctorReport::from_snapshot(&snapshot);
                if config.json {
                    serde_json::to_writer(&mut output, &report)?;
                    output.write_all(b"\n")?;
                } else {
                    output.write_all(report.text().as_bytes())?;
                }
                return Ok(report.exit_code());
            }
            if config.json {
                serde_json::to_writer(&mut output, &snapshot)?;
                output.write_all(b"\n")?;
            } else {
                output.write_all(output::snapshot_text(&snapshot).as_bytes())?;
            }
        }
    }
    Ok(0)
}
