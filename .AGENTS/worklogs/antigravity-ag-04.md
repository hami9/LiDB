# Task AG-04 — Interface metadata and namespace identity

- Task ID: `AG-04`
- Issue: [#53](https://github.com/hami9/LiDB/issues/53)
- Owner: `agent/antigravity/ag-04-interfaces`
- Worktree: `.worktrees/antigravity/ag-04-interfaces`
- Base commit: `979f756` (`agent/antigravity/issue-50-routes`, PR #52)
- Stacked PR base: `agent/antigravity/issue-50-routes`
- Scope: `crates/lidb-collect/src/interface.rs`, `crates/lidb-collect/src/lib.rs`, `crates/lidb-collect/README.md`, `crates/lidb-core/src/network.rs`, `apps/lidash/src/config.rs`, `apps/lidash/src/output.rs`, `apps/lidash/src/lib.rs`, `apps/lidash/tests/cli.rs`, `docs/DECISIONS.md`
- Status: `ready_for_review`

## Acceptance criteria

- Dedicated interface module in `lidb-collect`: `crates/lidb-collect/src/interface.rs`.
- Read interface list and attributes from `/sys/class/net` (with custom `sys_root: &Path` for test fixtures).
- Collect attributes for each interface:
  - `ifindex`: read from `ifindex` file (`u32`).
  - `mtu`: read from `mtu` file (`u32`).
  - `oper_state`: read from `operstate` file, mapped to `InterfaceOperState` (`up`, `down`, `unknown`, `testing`, `dormant`, `lowerlayerdown`, `notpresent`).
  - `mac_address`: read from `address` file (`String`).
  - `speed_mbps`: read from `speed` file (`Option<u64>`). Safely treat missing, negative (e.g. `-1` on loopback/unplugged ethernet), or unsupported speed as `None` without collection error.
  - `flags`: read from `flags` file (hex formatted or decoded string list).
- Network namespace identity:
  - Best-effort capture from `/proc/self/ns/net` using stat inode (`st_ino`) or readlink (`net:[<inode>]`).
  - Return `NetworkNamespace::new("current", Some(inode))`.
  - In fixtures, read `self/ns/net` if present under `proc_root`, or return `None` if absent.
- Bounded capacities:
  - Enforce `MAX_NETWORK_INTERFACES = 64`.
  - Respect 1 MiB per-file limit (`MAX_SOURCE_BYTES`).
  - Omitted interfaces beyond 64 are tracked in `dropped_interfaces`.
- Symlink and path safety:
  - In live mode (`/sys`), permit valid device symlinks (`/sys/class/net/<iface> -> ...`) while ensuring paths remain within sysfs.
  - In fixture mode, reject directory traversal escapes (`..`) or symlinks escaping the fixture root.
  - Reject interface names with ASCII control characters or path separators (`/`, `..`).
- Graceful handling:
  - Handle disappearing interfaces during iteration without failing the entire collection.
  - Differentiate legitimate empty table from missing/unsupported sysfs or permission-denied.
  - Return typed `SourceAvailability` (`Available`, `Unsupported`, `PermissionDenied`, `Error`).
- Contract integration:
  - Expose `InterfaceReport` in `lidb-collect` with schema `0.1`, `monotonic_ns`, `collection_duration_ns`, `mode`, `status`, `namespace`, `interfaces`, and `dropped_interfaces`.
  - Implement `InterfaceReport::populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError>`.
  - Expose `LinuxCollector::interfaces(&self) -> InterfaceReport`.
- CLI presentation:
  - Add `interfaces` command (aliases: `iface`, `interface`) and `--json` format to `apps/lidash`.
  - Text view displays tabular interface attributes cleanly without terminal control injection.
- Comprehensive test coverage:
  - Unit tests for interface parsing, operstate mapping, negative speed handling, flags, capacity truncation, symlink traversal prevention, and empty/missing/denied sources.
  - CLI integration tests for `lidash interfaces` and `lidash interfaces --json`.
  - Clean `unsupported` status on non-Linux platforms.

## Validation evidence

All checks executed on local toolchain (Rust 1.85.0, Python 3.12.10):

| Gate / Command | Outcome | Detail |
| --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | Zero formatting differences across all crates |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | 0 warnings, warnings treated as errors |
| `cargo test --workspace --locked --all-targets` | PASS | 80 tests passed (29 core, 2 protocol, 32 collect, 17 app/cli) |
| `cargo test --workspace --locked --doc` | PASS | 0 doc tests (clean) |
| `python -m unittest discover -s tests/ci -p 'test_*.py' -v` | PASS | 18 policy, worktree, and release tests passed |
| `python scripts/check_docs.py` | PASS | 140 local Markdown links checked |
| `git diff --check` | PASS | No whitespace or line-ending anomalies |
| `cargo run -p lidash -- interfaces --json` | PASS | Reports `schema_version: 0.1`, typed `unsupported` on Windows |
| `cargo run -p lidash -- interfaces` | PASS | Tabular text with clean unavailable notice |
