# Issue #50 — IPv6 route selectors, route flags, and bounded parser retention

- Owner: Antigravity worker, in isolated worktree `.worktrees/antigravity/issue-50-routes`, branch `agent/antigravity/issue-50-routes`.
- Base: fast-forwarded from main `842f877` to PR #46 -> PR #48 -> PR #51 (`origin/agent/codex/route-report`, commit `364c5e8`).
- Status: `ready_for_review`, targeted to base branch `agent/codex/route-report`.
- Owned scope: `RouteRecord` typed source prefix selector (SADR) and route flags fields, bounded streaming route parsing in `crates/lidb-collect/src/route.rs`, omission accounting, and malformed overflow tail validation. CLI text formatting in `apps/lidash/src/output.rs`. Interfaces, sockets, CI/CD, and release metadata outside this slice remain untouched.

## Problem and reproduction

In PR #48 / Codex review on #49, Linux `/proc/net/ipv6_route` parsing extracted destination, prefix length, gateway, metric, and interface, but skipped columns 2/3 (`src_ip`, `src_prefix`) and column 8 (`flags`). Consequently:
1. Two source-address-dependent routing (SADR) entries with identical destination prefixes but differing source prefix selectors collapsed to equal `RouteRecord` instances, falsely triggering `SourceAvailability::Error("source contains duplicate route")` and zeroing out valid IPv6 routing tables.
2. Distinct routes differing solely in route flags (such as `RTF_UP` vs `RTF_REJECT`) collided as duplicates.
3. Public parser functions `parse_v4_routes` and `parse_v6_routes` accepted arbitrary string sizes without bounding input or lazy evaluation, collecting all lines into intermediate heap vectors before truncation, and used $O(N^2)$ duplicate scans.
4. Truncation at 1,024 records occurred post-parse without validating that subsequent lines in an overflow tail were syntactically valid or free from duplicate anomalies.

Reproduction fixture:
```text
20010db8000000000000000000000000 20 20010db8000100000000000000000000 40 00000000000000000000000000000000 00000400 00000000 00000000 00000001 eth0
20010db8000000000000000000000000 20 20010db8000200000000000000000000 40 00000000000000000000000000000000 00000400 00000000 00000000 00000001 eth0
```
Prior behavior: `v6_status: Error("source contains duplicate route")`, 0 routes retained.

## Implementation details

1. **`crates/lidb-core/src/network.rs`**:
   - Derived `PartialOrd, Ord, Hash` on `AddressFamily` and `RouteRecord`.
   - Added typed fields to `RouteRecord`:
     - `source_prefix: Option<IpAddr>`
     - `source_prefix_len: Option<u8>`
     - `flags: Option<u32>`
   - Added validated builder and accessor methods:
     - `with_source_prefix(mut self, source: IpAddr, prefix_len: u8) -> Result<Self, NetworkError>`
     - `with_flags(mut self, flags: u32) -> Self`
     - `source_prefix(&self) -> Option<IpAddr>`
     - `source_prefix_len(&self) -> Option<u8>`
     - `flags(&self) -> Option<u32>`

2. **`crates/lidb-collect/src/route.rs`**:
   - Defined `ParsedRoutes` struct implementing `Deref<Target = [RouteRecord]>` and `IntoIterator<Item = RouteRecord>` with `dropped_routes: u64`.
   - Bounded input string parsing at `MAX_SOURCE_BYTES` (1 MiB) and rejected non-whitespace ASCII control characters.
   - Refactored `parse_v4_routes` and `parse_v6_routes` to iterate lazily over lines without allocating intermediate line vectors.
   - Preserved IPv4 route flags from column 3 and IPv6 route flags from column 8.
   - Preserved IPv6 source selector (`src_ip` from col 2, `src_prefix` from col 3) when present (`src_prefix > 0 || !src_ip.is_unspecified()`).
   - Bounded deduplication lookup using `BTreeSet<RouteRecord>` capped at `MAX_NETWORK_ROUTES` (1,024).
   - Validated malformed overflow tails: any malformed line in an oversized routing table returns a typed `SourceAvailability::Error` rather than being silently dropped.
   - Updated `collect_routes_at` to incorporate parser omission counters into the consolidated report `dropped_routes`.

3. **`apps/lidash/src/output.rs`**:
   - Updated `routes_text` to format source prefix selector `from <src>/<len>` when present, following Linux `ip -6 route` convention.

4. **Testing and Verification**:
   - Added unit tests for `RouteRecord` source prefix validation, family consistency, and `BTreeSet` deduplication.
   - Added route parser tests for:
     - IPv6 SADR source prefix variants (Issue #50 reproduction fixture).
     - Route flags and reject route differentiation (IPv4 and IPv6).
     - Malformed source prefix, source IP, or route flag hex.
     - Input size limit (> 1 MiB) and control character rejection.
     - Bounded retention with 1,030 routes (1,024 retained, 6 dropped).
     - Overflow tails with malformed lines (rejected with error).
     - Overflow tails with duplicate records (rejected with error).
