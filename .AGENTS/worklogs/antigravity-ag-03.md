# Task AG-03 — Read-only IPv4/IPv6 route observations

- Task ID: `AG-03`
- Issue: [#47](https://github.com/hami9/LiDB/issues/47)
- Owner: `agent/antigravity/ag-03-routes`
- Worktree: `.worktrees/antigravity/ag-03-routes`
- Base commit: `c271518` (`agent/antigravity/ag-02-network`, PR #46)
- Scope: `crates/lidb-collect/src/route.rs`, `crates/lidb-collect/src/lib.rs`, `crates/lidb-collect/README.md`, `apps/lidash/src/config.rs`, `apps/lidash/src/output.rs`, `apps/lidash/src/lib.rs`, `apps/lidash/tests/cli.rs`
- Status: `completed`

## Acceptance criteria

- Dedicated route module in `lidb-collect`: `crates/lidb-collect/src/route.rs`.
- Read IPv4 routes from `/proc/net/route` and IPv6 routes from `/proc/net/ipv6_route`.
- Support custom procfs root path (`proc_root: &Path`) for safe test fixtures.
- Parse IPv4 `/proc/net/route`:
  - Columns: `Iface`, `Destination`, `Gateway`, `Flags`, `RefCnt`, `Use`, `Metric`, `Mask`, `MTU`, `Window`, `IRTT`.
  - Destination and Mask: 8 hex characters, little-endian host order (standard Linux kernel `/proc/net/route` output format where e.g. `0001A8C0` is `192.168.1.0`).
  - Calculate CIDR prefix length from netmask.
  - Gateway: 8 hex characters, parse as `Option<IpAddr>` if non-zero.
  - Interface name: validated string bounds and control-character free.
  - Metric: decimal integer parsed to `Option<u32>`.
- Parse IPv6 `/proc/net/ipv6_route`:
  - Columns: `dest_ip` (32 hex characters), `dest_prefix` (2 hex characters), `src_ip` (32 hex characters), `src_prefix` (2 hex characters), `next_hop` (32 hex characters), `metric` (8 hex characters), `refcnt`, `use`, `flags` (8 hex characters), `iface`.
  - Parse 128-bit IPv6 destination address and prefix length (`0..=128`).
  - Next hop / gateway: parse if non-zero (`::`).
  - Metric: hex integer parsed to `Option<u32>`.
- Bound capacities:
  - Enforce `MAX_NETWORK_ROUTES = 1024`.
  - Respect 1 MiB per-file limit (`read_to_string_bounded` from `crate::read`).
  - Excess routes beyond limit are omitted and recorded in dropped records count.
- Source availability handling:
  - Legitimate empty tables (e.g. only header, or 0 routes) return `SourceAvailability::Available` with 0 records.
  - Missing file returns `SourceAvailability::temporarily_unavailable` or `SourceAvailability::unsupported`.
  - Permission denied returns `SourceAvailability::permission_denied`.
  - Malformed file returns `SourceAvailability::error`.
  - Non-Linux platforms report `SourceAvailability::unsupported`.
- Privacy and security boundaries:
  - Non-root, local, metadata only.
  - No shell calls (`ip route`), no socket opens, no route mutations.
  - Bounded allocations, no unbounded loops.
- Comprehensive unit tests:
  - Valid IPv4 default route and subnet route parsing.
  - Valid IPv6 default route and subnet route parsing.
  - Empty routing tables.
  - Missing and permission-denied files.
  - Malformed lines, invalid hex, invalid prefix lengths.
  - Capacity limit overflow and dropped record accounting.
  - CLI `routes` and `routes --json` integration tests.

## Validation evidence

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo test --workspace --locked`: PASS (56 passed: 14 collect, 26 core, 12 app + 4 cli, 2 protocol)
- `python scripts/check_docs.py`: PASS (140 local Markdown links checked)

