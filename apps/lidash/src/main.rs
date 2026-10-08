//! P0 CLI scaffold: explicitly no running daemon, collectors, or TUI.
#![forbid(unsafe_code)]

use lidb_core::capability::bootstrap_registry;
use lidb_protocol::PROTOCOL_VERSION;
use std::env;
use std::fmt::Write as _;

const HELP: &str = "LiDashBoard (LiDB) — P0 foundation\n\
Usage:\n\
  lidash --help\n\
  lidash --version\n\
  lidash status [--json]\n\
  lidash capabilities [--json]\n\
\n\
All metric collectors, the daemon IPC connection, and the TUI are not implemented yet.\n";

fn quote_json(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for c in value.chars() {
        match c {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0000}'..='\u{001f}' => {
                write!(&mut output, "\\u{:04x}", c as u32).expect("write to String cannot fail");
            }
            _ => output.push(c),
        }
    }
    output.push('"');
    output
}

fn run(mut args: impl Iterator<Item = String>) -> Result<String, String> {
    let command = args.next().unwrap_or_else(|| "--help".to_owned());
    if command == "--help" || command == "-h" || command == "help" {
        return Ok(HELP.to_owned());
    }
    if command == "--version" || command == "-V" {
        return Ok(format!(
            "lidash {} (bootstrap; no collectors)\n",
            env!("CARGO_PKG_VERSION")
        ));
    }
    if command != "status" && command != "capabilities" {
        return Err(format!("unknown command: {command}\n{HELP}"));
    }
    let fmt = args.next();
    if !matches!(fmt.as_deref(), None | Some("--json")) || args.next().is_some() {
        return Err(format!(
            "unknown option for {command}: use --json or no option"
        ));
    }
    let json = fmt.is_some();
    if command == "status" {
        if json {
            Ok(format!(
                "{{\"schema_version\":\"{}.{}\",\"component\":\"lidash\",\"runtime\":\"not_implemented\",\"collector_running\":false}}\n",
                PROTOCOL_VERSION.major, PROTOCOL_VERSION.minor
            ))
        } else {
            Ok("LiDB P0: no collectors or daemon IPC implemented; no live metrics.\n".into())
        }
    } else {
        let registry = bootstrap_registry();
        if json {
            let items = registry
                .iter()
                .map(|item| {
                    format!(
                        "{{\"id\":{},\"status\":{},\"reason\":{}}}",
                        quote_json(item.id()),
                        quote_json(item.state().as_str()),
                        quote_json(item.reason().unwrap_or(""))
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            Ok(format!(
                "{{\"schema_version\":\"0.1\",\"capabilities\":[{items}]}}\n"
            ))
        } else {
            let mut out = String::new();
            for item in registry.iter() {
                writeln!(
                    &mut out,
                    "{}: {} — {}",
                    item.id(),
                    item.state().as_str(),
                    item.reason().unwrap_or("No reason")
                )
                .expect("write to String cannot fail");
            }
            Ok(out)
        }
    }
}

fn main() {
    match run(env::args().skip(1)) {
        Ok(output) => print!("{output}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> impl Iterator<Item = String> {
        values
            .iter()
            .map(|x| (*x).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn safe_json_escapes_control_characters() {
        assert_eq!(quote_json("a\nb"), "\"a\\nb\"");
        assert_eq!(quote_json("\"slash\\"), "\"\\\"slash\\\\\"");
        assert_eq!(quote_json("\u{0001}"), "\"\\u0001\"");
    }

    #[test]
    fn status_is_truthful_and_immutable() {
        assert!(run(args(&["status", "--json"]))
            .unwrap()
            .contains("\"collector_running\":false"));
        assert!(run(args(&["capabilities", "--json"]))
            .unwrap()
            .contains("\"status\":\"disabled\""));
        assert!(run(args(&["made-up"])).is_err());
        assert!(run(args(&["status", "--json", "extra"])).is_err());
    }
}
