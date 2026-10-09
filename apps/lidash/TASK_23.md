# Issue #23: public host CLI and terminal monitor

Owner: `agent/codex/public-cli`; scope: `apps/lidash/**`.

Acceptance: default interactive/headless selection; two-sample host snapshot and
actual-source doctor; labeled fixture JSON/text; typed unavailable values;
monochrome narrow-aware UI with pause/help/quit/details; bounded latest-slot
background collection and cancellation; terminal restoration on errors/unwind,
keyboard Ctrl-C and SIGINT/SIGTERM. No daemon, listener, active probe, GPU/fabric
view, configuration changes, or simulated live metrics.

Dependencies: core snapshot contract at `7900e75`, collector API at `f486bb7`.
Collector files were restored only to this worker's working tree for verification
and are not part of the app commit. Workspace lockfile belongs to the coordinator.

Verification on Linux x86_64, Rust 1.85.0:

- `cargo fmt -p lidash`: exit 0.
- `cargo test -p lidash`: exit 0; six unit tests, four CLI integration tests.
- `cargo clippy -p lidash --all-targets -- -D warnings`: exit 0.
- `cargo build -p lidash`: exit 0.
- `python3 apps/lidash/tests/pty_smoke.py target/debug/lidash`: exit 0; real
  pseudoterminal normal quit, keyboard Ctrl-C, SIGINT, SIGTERM, resize and exact
  terminal-mode restoration; 60000ms sampling interval proves cancellation wakes.

The first compile preceded collector integration and failed due to its placeholder
API. The first integration run failed because the test expected `procfs` instead
of the collector's actual `proc.meminfo` source identifier; fixed and rerun. The
first PTY helper expected contiguous rendered header words, but Ratatui separates
words with cursor escapes; readiness now checks the actual rendered keywords.

Independent read-only review by `/root/cli/cli_review` confirmed termination and
cleanup with its own PTY experiment. It identified wide-cell truncation and too
small initial terminal minimum; fixed with an Enter detail view and 35x10 minimum.
Remaining follow-up: allow scrolling long details in a short terminal. No actual
specialized hardware or aarch64 validation was performed by this worker.

Five review axes: security/privacy (local read-only file sources; no payloads or
network access); correctness (typed states, precise integer export, actual doctor
source failures); architecture (core/collector/presentation separated, bounded
single-slot worker); UX (headless, narrow, monochrome and keyboard paths tested);
delivery (English app docs/tests/evidence, Rust 1.85 pins, coordinator owns shared
state and CI). These are evidence and self-review, not maintainer approval.
