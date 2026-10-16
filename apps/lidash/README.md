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
cargo run -p lidash -- routes --json
cargo run -p lidash -- interfaces --json
cargo run -p lidash -- sockets --json
cargo run -p lidash -- network --json
cargo run -p lidash -- export --json --output evidence.json
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
windows. Press `q`, `Esc`, or `Ctrl-C` to quit; `Tab`, `1`, `2`, `3`, or `Left`/`Right` switches
between the full metric list (`1: Metrics`), the CPU & Memory overview tab (`2: CPU/Mem`), and the Network overview tab (`3: Network`); `Space` pauses
displayed readings; `?` or `h` toggles help; Enter opens the full details of the
first visible metric on the Metrics tab. `Up`/`Down` or `j`/`k` scroll metrics, PageUp/PageDown scroll ten
metrics or scroll wrapped text while help/details are open,
and Home/End jump to the first/last. Windows below 35x10 show a resize prompt.
Sampling continues while paused; resume immediately displays the latest retained
snapshot. The age
indicator shows the displayed snapshot getting older. A bounded slot retains
only the latest collection; `skipped` counts replaced, unconsumed snapshots,
while `dropped` counts omitted metrics. Collection runs outside the rendering
thread. SIGINT and SIGTERM request exit through the same cleanup path as `q`.
Terminal modes, alternate screen, and cursor are restored before waiting for the
worker on normal exit, errors, and unwinding.

Interactive collection has a fixed 5-second batch deadline, checked independently
of `--interval-ms` and pause. An overdue batch is discarded; the TUI restores the
terminal, reports a collection timeout, and exits 2. Quitting cancels publication
and wakes the refresh wait, then allows a 100ms userspace grace period for the
worker to finish. A still-running worker is detached, an explicit shutdown timeout
is reported, and the CLI exits 2. Repeated cleanup does not wait again. After a
shutdown timeout, this process refuses replacement workers even if the abandoned
worker later returns; restart the process instead.

These are application lifecycle limits, not cancellation of an arbitrary kernel
read or a hard real-time process-exit guarantee. One admitted batch may finish its
remaining source reads after cancellation but cannot publish or start another
batch. Snapshot/doctor remain synchronous and have no read deadline. See
[ADR-023](../../docs/DECISIONS.md#adr-023--bounded-interactive-collector-lifecycle-2026-10-16).

Doctor reports source capabilities from actual observation states. A source with
both successful and unavailable observations is `degraded`. It performs no
connectivity test or privilege escalation. Exit codes are 0 when at least one
metric is available (including degraded operation), 1 when no metric is available,
and 2 for invalid arguments or runtime failure. The JSON `collection_status`
(`available`, `degraded`, `unavailable`) and per-source failures describe source
collection availability; they do not diagnose host or subsystem health.
Snapshot exits 0 even when some/all source readings are unavailable: callers
must inspect its metric states. A closed output pipe is handled quietly.

`routes` explicitly reads IPv4/IPv6 routing metadata in the selected procfs view.
Its route-only JSON schema `0.1` includes `mode`, `clock_source`, `monotonic_ns`,
`collection_duration_ns`, stable `v4_source`/`v6_source` IDs, independent source
states, retained records and `dropped_routes`. Explicit `--proc-root /proc` stays
`fixture`. Text shows each source's failure reason; an unavailable table is not
reported as an observed empty table. Exit 0 means the report was written; inspect
both source states and omissions, just as for snapshot. No connectivity is tested.

Routes can identify local network infrastructure; save/share output deliberately.
The timestamp shares the owning collector's clock with snapshots and cannot be
compared across runs. Reads are sequential and synchronous, with no hard read
deadline. The procfs view is neither a complete policy-routing lookup nor proof
of host-wide visibility in a container. Namespace identity is not captured yet.
Route selector/flag semantics and temporary parser retention remain a separately
tracked follow-up before routing completeness is claimed.

`interfaces` explicitly reads network interface metadata from sysfs (`/sys/class/net`)
and best-effort network namespace context from `/proc/self/ns/net`.
Its interface-only JSON schema `0.1` reports interface index, MTU, operational state,
MAC address, link speed in Mbps, decoded flags, and namespace identity (`NetworkNamespace`).
Explicit `--sys-root PATH` or `--proc-root PATH` labels the output `fixture`. Text displays
state, flags, MTU, speed, and hardware addresses cleanly. Missing, unknown, or negative
virtual link speeds (such as loopback) are treated truthfully as `None` rather than errors.
Disappearing interfaces during enumeration do not cause collection aborts.

`sockets` explicitly reads TCP and UDP socket endpoints from `/proc/net/{tcp,tcp6,udp,udp6}`.
Its socket-only JSON schema `0.1` reports protocol (`tcp`, `udp`), local and remote IP/port endpoints, connection state, and socket inode numbers.
Output is bounded by `MAX_NETWORK_SOCKETS = 2048`, tracking omissions in `dropped_sockets`. Sockets collection is strictly read-only and non-privileged; it does not map sockets to PIDs or inspect process state.
Explicit `--proc-root PATH` labels the output `fixture`. Text displays protocol, endpoints, connection state, and inode cleanly.

`export` captures a unified offline diagnostic evidence bundle aggregating host metrics snapshot, source doctor capabilities, and network telemetry into a structured JSON or text report.
Its export schema `0.1` embeds an explicit privacy declaration: non-root, read-only, metadata-only with zero packet payloads, zero decrypted TLS, zero process arguments, zero environments, and zero secrets.
Use `--output PATH` to write directly to a destination file safely.

The TUI maintains an in-memory bounded ring buffer of historical updates (`MAX_VIEW_HISTORY = 120` samples, ~2 minutes of history at 1s intervals).
Press `[` or `,` to step backward in history (auto-pauses the live stream) and `]` or `.` to step forward.
While navigating history, the header bar indicates historical position (`PAUSED [-N/total]`) and true sample age; press `Space` to resume live streaming and snap back to the latest sample.

```bash
cargo test -p lidash
cargo clippy -p lidash --all-targets -- -D warnings
cargo build -p lidash
python3 apps/lidash/tests/pty_smoke.py target/debug/lidash
```

Tests use injected procfs files, Ratatui TestBackend, and a local pseudoterminal.
Linux `cargo test` also exercises an indefinitely parked test-only sampler in
separate PTY subprocesses for q/Ctrl-C/signals and the collection watchdog.
They verify read-only host observation and failure handling, not specialized
hardware support. Unix process termination with SIGKILL cannot run cleanup.
