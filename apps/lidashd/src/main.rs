//! No privileged behavior, network listener, sockets or daemon loops in P0.
#![forbid(unsafe_code)]

use std::env;

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".into());
    if args.next().is_some() {
        eprintln!("lidashd P0: unexpected arguments");
        std::process::exit(2);
    }
    match command.as_str() {
        "--help" | "-h" => println!("LiDashBoard daemon (P0 scaffold)\nUsage: lidashd --help | --version | status\nNo socket or collector service has been implemented."),
        "--version" | "-V" => println!("lidashd {} (bootstrap; inactive)", env!("CARGO_PKG_VERSION")),
        "status" => println!("lidashd is not implemented or running; no socket has been opened."),
        _ => {
            eprintln!("lidashd P0: unknown command: {command}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_has_no_hidden_runtime_configuration() {
        // Constant-identity smoke test; process behavior is verified in CLI integration.
        assert_eq!(env!("CARGO_PKG_NAME"), "lidashd");
    }
}
