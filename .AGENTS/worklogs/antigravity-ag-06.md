# Task AG-06 — Unified network report and interactive TUI network dashboard

- Task ID: `AG-06`
- Issue: [#57](https://github.com/hami9/LiDB/issues/57)
- PR: [#58](https://github.com/hami9/LiDB/pull/58)
- Owner: `agent/antigravity/ag-06-network-ui`
- Worktree: `.worktrees/antigravity/ag-06-network-ui`
- Base commit: `2cddeee` (`agent/antigravity/ag-05-sockets`, PR #56)
- Head commit: `91f2d27`
- Stacked PR base: `agent/antigravity/ag-05-sockets`
- Scope: `crates/lidb-collect/src/lib.rs`, `crates/lidb-collect/src/tests.rs`, `crates/lidb-collect/README.md`, `crates/lidb-core/src/network.rs`, `apps/lidash/src/config.rs`, `apps/lidash/src/output.rs`, `apps/lidash/src/lib.rs`, `apps/lidash/src/worker.rs`, `apps/lidash/src/tui.rs`, `apps/lidash/tests/cli.rs`, `apps/lidash/README.md`, `docs/DECISIONS.md`
- Status: `ready_for_review`

## Acceptance criteria

- Unified network collection in `lidb-collect`:
  - `LinuxCollector::network(&self) -> NetworkReport` unifying interface metadata, network namespace identity, IPv4/IPv6 routing tables, and TCP/UDP socket observations.
  - Consistent monotonic timestamp across sub-collections in the single unified network snapshot.
  - Correct aggregation of statuses (`interfaces_status`, `routes_status`, `sockets_status`), dropped counters (`dropped_interfaces`, `dropped_routes`, `dropped_sockets`), and collections into `NetworkReport`.
  - Integration tests in `crates/lidb-collect/src/tests.rs` verifying clock coherence and platform behavior.
- CLI presentation in `apps/lidash`:
  - Add `network` command (aliases: `network`, `net`) and `--json` format to `apps/lidash`.
  - Format unified textual summary in `output::network_text`: network namespace, interface overview, route summary, and socket counts/states.
  - CLI integration tests for `lidash network` and `lidash network --json`.
- Interactive TUI usability in `apps/lidash/src/tui.rs`:
  - Add `Tab::Network` (key '3' and Tab/Shift-Tab / arrow cycling) to TUI tab selector.
  - Render unified network diagnostics:
    - Active network namespace identity & procfs availability indicators
    - Interfaces list with flags, MTU, MAC, and assigned IPv4/IPv6 CIDRs
    - Routing table summary (default routes, gateways, interfaces)
    - Active sockets table (protocol, local/remote endpoints, states)
    - Capacity and dropped counter warnings if truncation occurred
  - Respect terminal boundaries, narrow terminals, and unprivileged execution.
  - TUI rendering tests for Tab::Network and navigation.
- Documentation & ADR:
  - Add ADR-029 in `docs/DECISIONS.md`.
  - Update `crates/lidb-collect/README.md` and `apps/lidash/README.md`.

## Implementation details

1. **Unified collector (`crates/lidb-collect/src/lib.rs`)**:
   - Added `pub fn network(&self) -> NetworkReport` to `LinuxCollector`.
   - Captures batch monotonic timestamp at construction and reads interfaces, routes, and sockets.
   - Populates records and source availability statuses into `NetworkReport` (schema `0.1`).
   - Handles route family status divergence via `consolidated_status()` without losing evidence.

2. **Core domain (`crates/lidb-core/src/network.rs`)**:
   - Derived `PartialEq, Eq` on `NetworkReport`.

3. **CLI command and formatting (`apps/lidash/src/config.rs`, `output.rs`, `lib.rs`)**:
   - Added `Command::Network` with aliases `network` and `net`.
   - Added `--json` support serializing the full `NetworkReport`.
   - Added `output::network_text` cleanly presenting namespace identity, interface overview, routing table, and active sockets with state and dropped counts.

4. **Background worker (`apps/lidash/src/worker.rs`)**:
   - Added `network: Option<NetworkReport>` to `worker::Update`.
   - Updated `Worker::start` to collect both snapshot and network report during sampling ticks.

5. **Terminal UI (`apps/lidash/src/tui.rs`)**:
   - Added `Tab::Network` (2) to `Tab` enum with cycling support.
   - Bound key `'3'` to directly activate `Tab::Network`.
   - Added `draw_network` rendering namespace status, interfaces, routing table, and active sockets.
   - Handled compact rendering for narrow terminals and small terminal areas.

6. **Testing (`crates/lidb-collect/src/tests.rs`, `apps/lidash/src/tui.rs`, `apps/lidash/tests/cli.rs`)**:
   - Added unit and integration tests across collector, TUI, and CLI binary.

## Verification

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo test --workspace --locked --all-targets`: PASS (99 tests passed)
- `python scripts/check_docs.py`: PASS (140 local Markdown links checked)
- GitHub Actions CI (Run [#42227574362](https://github.com/hami9/LiDB/actions/runs/42227574362)):
  - `CI Gate`: PASS
  - `Rust (x86_64)`: PASS
  - `Rust (aarch64)`: PASS
  - `Worktree Windows smoke`: PASS
  - `CodeQL (Rust & Actions)`: PASS
  - `Docs and policy`: PASS
  - `markdown-links`: PASS

