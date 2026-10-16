# Issue #44 — blocked-sampler terminal lifecycle regression

**Owner:** Codex lifecycle-test worker. **Branch:** `agent/codex/blocked-pty`.
**Base:** `5360ace`. The coordinator owns worker/TUI implementation, central
state, CI, issue delivery and integration.

## Acceptance and implementation

- Add a Linux-only unit regression that drives the actual TUI through a PTY
  with a test-only sampler callback parked indefinitely after a disposable
  readiness marker. No production environment flags or new executables exist.
- Isolate every blocked sampler in a child unit-test process. The ignored helper
  exits with the CLI runtime-error mapping after TUI teardown, rather than
  panicking on the expected timeout.
- Wait for both callback entry and rendered terminal header, then exercise `q`,
  raw Ctrl-C, SIGINT and SIGTERM. Require exit 2 well before the 5-second
  collection watchdog, exact original termios restoration, and leave-alternate
  screen/show-cursor escapes before the runtime diagnostic.
- Also run the no-input collection watchdog with the configured 60-second
  refresh interval, requiring a collection-timeout diagnostic and restored
  terminal. Children are killed and reaped on every exceptional path.

This is a deterministic blocked-callback simulation on Linux, not validation of
an uninterruptible procfs, FUSE or kernel read. It creates only disposable marker
files and a child pseudoterminal; no host settings or privileges are changed.

## Validation

Pending worker/TUI integration: the base does not yet provide the coordinator's
`tui::run_with_sampler`, `worker::SAMPLE_TIMEOUT` or `worker::SHUTDOWN_GRACE` APIs.
Compilation and runtime evidence must be recorded after integration. Executed
locally on 2026-10-16:

- `rustfmt --edition 2021 --check apps/lidash/src/lifecycle_tests.rs apps/lidash/src/lib.rs`
  — PASS (exit 0), after applying formatter output.
- `python3 -c 'import ast, pathlib; ast.parse(pathlib.Path("apps/lidash/tests/blocked_pty.py").read_text()); print("Python syntax passed")'`
  — PASS (exit 0).
- `git diff --check` — PASS (exit 0).
- Rust compilation and actual PTY execution — NOT RUN in this worker checkout;
  the required coordinator APIs are not yet present in its base.
