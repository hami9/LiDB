# LiDashBoard (LiDB)

**A local, read-only Linux diagnostic tool for the terminal.**

LiDB provides a runnable foundation for developers, VPS owners and system operators inspecting a Linux host over SSH. The core collects CPU, memory, load, uptime, disk, network-interface and pressure observations without a cloud account, a daemon or elevated privileges. Missing data is reported with a reason instead of a fabricated zero.

The project is in core development. There is no published binary release or production performance claim. Native Linux x86_64 and aarch64 passed the baseline CI checks; see the [compatibility matrix](docs/INTEGRATION_MATRIX.md) for exact evidence and limits.

## Build and install

Requirements: Linux, Git and Rust **1.85 or newer**, including Cargo. The live baseline reads the current process's procfs view; restricted containers and hosts may expose fewer metrics.

```bash
git clone https://github.com/hami9/LiDB.git
cd LiDB
cargo build --workspace --locked
cargo install --path apps/lidash --locked
```

Cargo downloads the locked build dependencies when they are not already cached. The installed application makes no outbound network requests. Remove it with `cargo uninstall lidash`; installation does not enable a service or modify host configuration.

## Use

```bash
lidash                         # dashboard when input/output are TTYs; text otherwise
lidash snapshot                # one text snapshot
lidash snapshot --json         # machine-readable observations
lidash doctor                  # collector capabilities and failure reasons
lidash doctor --json
lidash tui                     # explicitly request an interactive dashboard
lidash --help
lidash --version
```

Use `--interval-ms` to choose a sampling interval from **250 to 60000 ms**: the default is 1000 ms for the dashboard and 250 ms for snapshot/doctor. One-shot commands collect two samples separated by that interval. The dashboard is monochrome; `--no-color` is accepted explicitly. A custom `--proc-root PATH` selects a procfs directory and labels output `fixture`; its data does not establish real-host validation. The CLI help documents where options are accepted.

The scaffold commands `status` and `capabilities` remain aliases for `snapshot` and `doctor`. Doctor exits 0 when at least one host value is available, 1 when none are available, and 2 for an input/runtime error. A partially unavailable host is still inspectable; check individual reasons.

In the dashboard, `q`, `Esc` or `Ctrl-C` quits; `Space` pauses displayed data while collection continues; `?`/`h` shows help. Arrow keys or `j`/`k`, PageUp/PageDown and Home/End scroll the observations.

CPU utilization requires consecutive samples. Disk and interface counters are cumulative observations; the UI must label any derived rates and counter-reset gaps. Memory availability uses the kernel's `MemAvailable` field when exposed. PSI observations can be unsupported on kernels or mounts that do not expose them. `doctor` reads metadata and collector sources; it does not run connectivity probes or request privileges.

## Scope and safety

- Local host diagnostics, CLI/text/JSON output and a keyboard-operated TUI.
- Typed availability, provenance, units, collector-local monotonic timestamps and bounded collection.
- Read-only operation with no automatic remediation, packet capture, remote listener or telemetry service.
- No collection of packet bodies, credentials, process environments or command-line arguments.
- Optional deeper Linux networking and eBPF work belongs to later, separately reviewed slices.

Vendor-specific hardware, model-serving integrations and multi-node orchestration are outside the public product scope. The retained `lidb-protocol` library contains pure version-negotiation contracts; it does not provide IPC or a running service.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
python3 scripts/check_docs.py
```

See the [Core data reference](docs/CORE.md), [architecture](docs/ARCHITECTURE.md), [product requirements](docs/PRODUCT_REQUIREMENTS.md), [roadmap](ROADMAP.md), [capability model](docs/CAPABILITY_MODEL.md), [telemetry contract](docs/OBSERVABILITY_CONTRACT.md), [test strategy](docs/TEST_STRATEGY.md) and [architecture decisions](docs/DECISIONS.md). The preserved [core contracts](docs/P0_CORE_CONTRACTS.md) and [bounded history](docs/P0_TELEMETRY_HISTORY.md) describe the existing foundation libraries.

Contributors should read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md). Parallel coding uses [isolated worktrees](.AGENTS/WORKTREES.md). [CI/CD](docs/CI_CD.md) and [branch protection](docs/BRANCH_PROTECTION.md) describe repository automation; committed rulesets require administrative activation.

LiDB is [MIT licensed](LICENSE). See [SECURITY.md](SECURITY.md) for vulnerability reporting, [GOVERNANCE.md](GOVERNANCE.md) for maintainership and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community conduct.
