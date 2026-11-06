# Task AG-02 — Typed network contracts and source policy

- Task ID: `AG-02`
- Owner: `agent/antigravity/ag-02-network`
- Worktree: `.worktrees/antigravity/ag-02-network`
- Base commit: `842f877f62cc19d68a0a3a6f81777848d19a9e77` (`main`, including merged Core #25 and CPU/Memory tab #34)
- Scope: `crates/lidb-core/src/network.rs`, `crates/lidb-core/src/lib.rs`, `docs/CORE.md` (or ADR)
- Status: `completed`

## Acceptance criteria

- Pure, platform-agnostic typed domain contracts in `lidb-core`: no filesystem access, no OS-specific headers, no network side effects.
- Structured representations for routes, endpoints/sockets, and interface attributes:
  - Address family (`Ipv4`, `Ipv6`).
  - Routes: destination, prefix/netmask, next-hop/gateway, interface name, metric/priority.
  - Sockets: protocol (`Tcp`, `Udp`), local/remote address and port, socket state, address family, inode.
  - Interfaces: interface name, ifindex, MTU, operational status (`Up`, `Down`, `Unknown`), MAC/hardware address, flags.
- Bounded, versioned `NetworkReport` (schema `0.1`):
  - Provenance: monotonic collection timestamp (`monotonic_ns`), collection duration (`collection_duration_ns`), clock source, mode (`SnapshotMode::Live` / `SnapshotMode::Fixture`), network namespace (`NetworkNamespace`).
  - Explicit source availability: distinct typed states for interfaces, routes, and sockets (differentiating source failure/unsupported/denied from an observed empty table).
  - Bounded capacities and dropped-record accounting (`dropped_records`).
  - Text and identifier validation (rejection of ASCII control characters, bounds on string lengths).
- Unit tests covering:
  - IPv4 and IPv6 records and invariants.
  - Exact counters, identifiers, and states.
  - Rejection of malformed, control-bearing, or oversized fields.
  - Distinction between empty tables and source errors/unavailability.
  - Enforced capacity bounds and dropped record counters.
  - Serialization to JSON and schema compatibility.

## Validation evidence

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo test -p lidb-core --locked`: PASS (26/26 tests, including 7 comprehensive network tests)
- `cargo test --workspace --locked`: PASS (all workspace unit, integration, and doc tests)
- `python scripts/check_docs.py`: PASS (140 local Markdown links checked)
- Documentation: Added network inventory section to `docs/CORE.md` and ADR-022 to `docs/DECISIONS.md`.

