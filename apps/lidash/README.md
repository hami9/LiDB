# lidash host monitor

`lidash` reads local Linux procfs without a helper, listener, active probe, or
configuration changes. It presents CPU, memory, swap, load, uptime, network and
disk counters/rates, and pressure metrics supported by the collector. This slice
has no process attribution, hardware adapters, daemon, or diagnostic engine.

```bash
cargo run -p lidash -- --help
cargo run -p lidash -- snapshot
cargo run -p lidash -- snapshot --json
cargo run -p lidash -- doctor --json
cargo run -p lidash -- tui --interval-ms 1000 --no-color
```

With terminal stdin and stdout, the default command opens the TUI; with pipes it
prints one text snapshot. `--json` selects a snapshot when no command is given.
Explicit `tui` requires both streams to be terminals. `status` and `capabilities`
are compatibility aliases for `snapshot` and `doctor`. Options can precede or
follow the command.

`--interval-ms` accepts 250 through 60000 milliseconds. Snapshot and doctor take
an initial sample, wait one interval (250ms by default), and return the second
sample so rate counters have a baseline. TUI samples at 1000ms by default and
shows initial rates as unavailable until two valid observations exist. CPU busy
may remain unavailable when the underlying tick counters do not advance.

`--proc-root PATH` explicitly selects **fixture** input; every text view and JSON
report retains that label, including `--proc-root /proc`. Fixture data is never
reported as hardware validation. Missing or denied sources retain typed states
and reasons; missing data never becomes zero. Integer counters retain precision
in JSON. Source timestamps use one collector-local monotonic clock and cannot be
compared between different runs or hosts.

The TUI is monochrome by default, accepts `--no-color`, and adapts to narrow
windows. Press `q`, `Esc`, or `Ctrl-C` to quit; `Space` pauses displayed readings;
`?` or `h` toggles help; Enter opens the full details of the first visible metric.
Arrows or `j`/`k` scroll metrics, PageUp/PageDown scroll ten metrics or scroll
wrapped text while help/details are open,
and Home/End jump to the first/last. Windows below 35x10 show a resize prompt.
Sampling continues while paused; the age
indicator shows the displayed snapshot getting older. A bounded slot retains
only the latest collection; `skipped` counts replaced, unconsumed snapshots,
while `dropped` counts omitted metrics. Collection runs outside the rendering
thread. Cancellation wakes the sampling wait immediately and joins the worker;
shutdown also waits for the current bounded procfs read to finish. Terminal modes,
alternate screen, and cursor are restored on normal exit, errors, and unwinding.
SIGINT and SIGTERM request a graceful exit through the same cleanup path.

Doctor reports source capabilities from actual observation states. A source with
both successful and unavailable observations is `degraded`. It performs no
connectivity test or privilege escalation. Exit codes are 0 when at least one
metric is available (including degraded operation), 1 when no metric is available,
and 2 for invalid arguments or runtime failure. The JSON `collection_status`
(`available`, `degraded`, `unavailable`) and per-source failures describe source
collection availability; they do not diagnose host or subsystem health.
Snapshot exits 0 even when some/all source readings are unavailable: callers
must inspect its metric states. A closed output pipe is handled quietly.

```bash
cargo test -p lidash
cargo clippy -p lidash --all-targets -- -D warnings
cargo build -p lidash
python3 apps/lidash/tests/pty_smoke.py target/debug/lidash
```

Tests use injected procfs files, Ratatui TestBackend, and a local pseudoterminal.
They verify read-only host observation and failure handling, not specialized
hardware support. Unix process termination with SIGKILL cannot run cleanup.
