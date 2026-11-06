# Task AG-05 — Bounded TCP/UDP socket metadata

- Task ID: `AG-05`
- Issue: [#55](https://github.com/hami9/LiDB/issues/55)
- PR: [#56](https://github.com/hami9/LiDB/pull/56)
- Owner: `agent/antigravity/ag-05-sockets`
- Worktree: `.worktrees/antigravity/ag-05-sockets`
- Base commit: `8daf803` (`agent/antigravity/ag-04-interfaces`, PR #54)
- Head commit: `abe3505`
- Stacked PR base: `agent/antigravity/ag-04-interfaces`
- Scope: `crates/lidb-collect/src/socket.rs`, `crates/lidb-collect/src/lib.rs`, `crates/lidb-collect/src/tests.rs`, `crates/lidb-collect/README.md`, `crates/lidb-core/src/network.rs`, `apps/lidash/src/config.rs`, `apps/lidash/src/output.rs`, `apps/lidash/src/lib.rs`, `apps/lidash/tests/cli.rs`, `docs/DECISIONS.md`
- Status: `ready_for_review`

## Acceptance criteria

- Dedicated socket collection module in `lidb-collect`: `crates/lidb-collect/src/socket.rs`.
- Read only the current namespace's bounded TCP/UDP IPv4/IPv6 metadata from `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, and `/proc/net/udp6`.
- Parse local and remote endpoints (IP address, port), connection state (`SocketState`), and inode into `SocketRecord`.
- Correct address and port network byte order parsing for both IPv4 and IPv6:
  - IPv4: little-endian 32-bit hex address, network-order 16-bit hex port.
  - IPv6: 128-bit hex address in four 32-bit host-order words, network-order 16-bit hex port.
  - Parse connection state (`01` = Established, `0A` = Listen, etc.) into `SocketState`.
  - Parse inode integer from column 9.
- Strict privacy:
  - Do NOT scan `/proc/<pid>` directories, command lines, environments, payloads, or credentials.
  - Do NOT map sockets to PIDs.
  - Document endpoint metadata as identifying.
- Strict bounds:
  - Enforce `MAX_NETWORK_SOCKETS = 2048`.
  - Respect 1 MiB per-file limit (`MAX_SOURCE_BYTES`).
  - Omitted sockets beyond 2048 are tracked in `dropped_sockets`.
- Truthful availability:
  - Differentiate legitimate empty tables (0 sockets) from missing, unsupported, or permission-denied procfs sources using `SourceAvailability`.
  - Aggregate status across TCP and UDP sources into a consolidated availability.
- Contract integration:
  - Expose `SocketReport` in `lidb-collect` with schema `0.1`, `monotonic_ns`, `collection_duration_ns`, `mode`, `status`, `sockets`, and `dropped_sockets`.
  - Implement `SocketReport::populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError>`.
  - Expose `LinuxCollector::sockets(&self) -> SocketReport`.
- CLI presentation:
  - Add `sockets` command (aliases: `socket`, `sock`) and `--json` format to `apps/lidash`.
  - Text view displays tabular socket records (protocol, local endpoint, remote endpoint, state, inode) cleanly without terminal control injection.
- Comprehensive test coverage:
  - Unit tests for IPv4/IPv6 TCP/UDP parsing, all connection states, capacity truncation (> 2048 sockets), and empty/missing/denied sources.
  - CLI integration tests for `lidash sockets` and `lidash sockets --json`.
  - Clean `unsupported` status on non-Linux platforms.

## Implementation details

1. **Core domain extensions (`crates/lidb-core/src/network.rs`)**:
   - Derived `PartialOrd`, `Ord`, `Hash` for `SocketProtocol`, `SocketState`, and `SocketRecord`.
   - Added `SocketState::from_linux_state(st: u8) -> Self` for standard Linux kernel TCP connection state numbers.
   - Added `as_str(&self) -> &'static str` and `fmt::Display` for `SocketProtocol` and `SocketState`.

2. **Socket collector (`crates/lidb-collect/src/socket.rs`)**:
   - Implemented `parse_v4_sockets` and `parse_v6_sockets` for `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, and `/proc/net/udp6`.
   - Correctly handles IPv4 8-hex little-endian addresses and IPv6 32-hex (four 32-bit host words decoded via `.to_le_bytes()`).
   - Parsed local and remote endpoints, mapping unconnected endpoints (`0.0.0.0:0` and `:::0`) to `remote_address: None` and `remote_port: None`.
   - Parsed socket connection state and socket inode (col 9).
   - Strict size bounds (`MAX_SOURCE_BYTES = 1 MiB`, `MAX_NETWORK_SOCKETS = 2048`).
   - Validates overflow lines and detects duplicate records even when capacity limit is reached.
   - Differentiates empty tables (`SourceAvailability::Available` with 0 records) from missing/denied sources.
   - Implemented `SocketReport::consolidated_status(&self)` and `SocketReport::populate_into(&mut self, report: &mut NetworkReport) -> Result<(), NetworkError>`.
   - Exposed `pub fn sockets(&self) -> SocketReport` on `LinuxCollector`.

3. **CLI command and presentation (`apps/lidash`)**:
   - Added `Command::Sockets` in `apps/lidash/src/config.rs` with aliases `sockets`, `socket`, `sock`.
   - Added `output::sockets_text` formatting protocol, endpoints, connection state, and inode cleanly.
   - Supported `lidash sockets --json`.
   - Added CLI tests in `apps/lidash/tests/cli.rs`.

4. **Documentation and Decisions**:
   - Added ADR-028 in `docs/DECISIONS.md`.
   - Updated `crates/lidb-collect/README.md` and `apps/lidash/README.md`.

## Verification evidence

- `cargo fmt --manifest-path Cargo.toml --all -- --check`: PASSED
- `cargo clippy --manifest-path Cargo.toml --workspace --all-targets --locked -- -D warnings`: PASSED
- `cargo test --manifest-path Cargo.toml --workspace --locked --all-targets`: PASSED (96 tests passed)
- `cargo test --manifest-path Cargo.toml --workspace --locked --doc`: PASSED
- `python -m unittest discover -s tests/ci -p 'test_*.py' -v`: PASSED (18 tests passed)
- `python scripts/check_docs.py`: PASSED (140 local Markdown links verified)
- `git diff --check`: PASSED
- GitHub Actions CI (run `42202168478`): PASSED all 10 checks including `CI Gate`, `Rust (x86_64)`, `Rust (aarch64)`, `Worktree Windows smoke`, `Docs and policy`, `CodeQL`, and `markdown-links`.
