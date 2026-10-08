# P0 Core Contracts — First Implementation Slice

**Scope:** GitHub Issue [#7](https://github.com/hami9/LiDB/issues/7). Work in `agent/chatgpt/p0-core`. This is the **first vertical slice**, not a declaration that P0 is complete.

## Delivered contracts

- A Rust 2021 workspace with `lidb-core`, `lidb-protocol`, `lidash` and `lidashd` packages. No external crate dependencies for this initial baseline.
- A duplicate-safe `CapabilityRegistry` with explicit unavailable reasons and known `org.lidb.*` identifiers. Bootstrap capabilities are **disabled** until collectors are implemented.
- Provenance-aware `MetricObservation<T>` with source, unit, collector-local monotonic timestamp and `MetricState<T>` distinguishing valid zero from unavailable/stale data.
- A pure `negotiate()` version contract, which does **not** imply that a socket transport or peer authentication exists.
- Unprivileged `lidash --help`, `lidash status [--json]`, `lidash capabilities [--json]`, and inactive `lidashd status` scaffolds.
- Unit/CLI integration tests for duplicate IDs, invalid reasons, unavailability states, protocol major mismatch and command failure modes.

## Limitations

- No CPU, GPU, filesystem, network, process, NVIDIA, RDMA, NCCL or eBPF collector exists yet.
- No Ratatui TUI or daemon listener exists. Antigravity's standalone prototype is tracked separately in [#8](https://github.com/hami9/LiDB/issues/8).
- The `monotonic_ns` timestamp carries a clock-domain assumption provided by the collector. Cross-node sync/offset uncertainty is deferred; this data is **not** automatically comparable across hosts.
- No protobuf/JSON IPC encoding, permission negotiation, socket peer credential verification or remote transport exists.
- `0.0.0` is the workspace's pre-release internal version and should only be updated with an approved release process.

## Next integration gates

1. Verify `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked --all-targets` on Linux x86_64/aarch64.
2. Decide minimum kernel, distro and Rust toolchain baselines using published support data.
3. Implement collector traits and an actual local daemon handshake as separate reviewed tasks, with bounded IPC frames and Unix peer credentials.
4. Wire Antigravity's terminal UI through accepted core contracts after the prototypes are reviewed; no shared crate edits in parallel.

## Security

All new Rust crates forbid unsafe code and P0 deliberately opens no privileged resources or network listeners. Nothing attempts to manipulate firewall rules, routing, namespace settings, kernel probes or GPU runtimes.
