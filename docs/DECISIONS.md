# Architecture decision records (ADR)

Decisions are **proposed** unless explicitly accepted by maintainers. Changing one requires an ADR entry with context, alternatives, tradeoffs, security effects and migration.

| ID | Status | Decision | Reason |
| --- | --- | --- | --- |
| ADR-001 | proposed | Rust userspace + C eBPF | Safe orchestration; native CO-RE integration |
| ADR-002 | proposed | Terminal-first, no web UI | Fast SSH workflows, low dependencies |
| ADR-003 | proposed | Unprivileged TUI; separate optional privileged helper | Reduce blast radius |
| ADR-004 | proposed | Read-only, metadata-only baseline | Operator trust and privacy |
| ADR-005 | proposed | Capability-gated integrations | Hardware/runtime support is highly variable |
| ADR-006 | proposed | Collector-local monotonic timestamps; no cross-domain comparability | Avoid false timeline correlations |
| ADR-007 | proposed | Default Tokio/epoll; optional later io_uring benchmark | Reduce initial complexity |
| ADR-008 | proposed | Defer external adapters; no native dynamic loading | Avoid unsafe native dynamic loading |
| ADR-009 | proposed | No automatic remediation or job scheduler | Needs separate safety and reliability validation |
| ADR-010 | proposed | MIT for project-owned code | Permissive open-source contribution and redistribution |
| ADR-011 | proposed | Semantic versioning for operator-visible interfaces | Predictable community ecosystem |
| ADR-012 | proposed | Explicit unavailable state, never numeric zero fallback | Preserve correctness |

Rust 1.85 is the selected minimum toolchain for the runnable foundation. Record native platform/toolchain evidence before claiming validation. Kernel fields are capability-detected; persistence, IPC and external adapters are deferred until an actual product need justifies a separate decision.

## Automation decisions (2026-10-08)

- **ADR-013, proposed:** Conventional Commits are the single automatic SemVer bump signal; explicit `Release-Bump:` commit footers may override. Non-feature commits default to PATCH.
- **ADR-014, proposed:** Release only after reusable CI and reproducible architecture-specific builds. Bootstrap docs-only commits must not produce a pretend binary.
- **ADR-015, proposed:** Protect `main` with a stable aggregated `CI Gate`, PR-only squash/rebase, no deletion/force-push. Ruleset activation is an administrative operation outside coding-agent authority.
- **ADR-016, proposed:** Public build archives include checksums and GitHub provenance attestations. This does not replace independent artifact verification or SBOM/license review.

## Multi-agent execution decisions (2026-10-08)

- **ADR-017, proposed:** Use real Git worktrees in an ignored `.worktrees/` folder for local agent isolation; no committed `.git` internals or copies of the repository.
- **ADR-018, proposed:** GitHub Issues + protected branch PRs are the cross-device ownership and integration control, not `git worktree` locking.
- **ADR-019, proposed:** The canonical agent policies remain `.AGENTS/` plus root `AGENTS.md`; integration shims `CLAUDE.md` and `.agents/rules/` are limited adapters.
- **ADR-020, proposed:** A single integrator owns central `.AGENTS/STATE.md`, `.AGENTS/WORKLOG.md`, shared schemas and CI while task agents edit disjoint modules.

## ADR-021 — public Linux product scope (2026-10-09)

**Status:** accepted direction under the maintainer's instruction to remove non-general-product scope and complete Core. Implementation acceptance and release approval remain separate gates.

**Context:** The repository already has typed Rust core/protocol contracts and bounded telemetry history. The earlier architecture expanded into specialized vendor hardware, runtime integrations and distributed operations before an ordinary Linux operator had a useful baseline.

**Decision:** Keep and extend the existing core. Deliver one local read-only Linux application: bounded procfs collection → typed snapshot → text/JSON and terminal UI. Focus on CPU, memory, load, uptime, disk/interface counters and optional PSI. Retain the pure `lidb-protocol` compatibility library; remove inactive daemon scope and require no helper/IPC/service for startup. Optional deeper Linux networking/eBPF remains later, explicitly scoped work. The selected userspace minimum is Rust 1.85.

**Alternatives:** A daemon-first architecture adds lifecycle/authentication cost to ordinary procfs reads. Vendor-first integration or distributed control does not serve the requested general Linux baseline. Replacing the existing typed core would discard useful tested contracts. These alternatives are not chosen for this delivery.

**Consequences:** Active product docs, roadmap and agent guidance adopt the public scope. Existing history and protocol contracts are preserved, and snapshot JSON remains pre-stable. No privileged operation, outbound runtime network request, persistent store or remote listener is introduced. Local x86_64 and pending native aarch64 evidence remain distinct; runnable code is not production or phase-exit approval.

**Migration:** The CLI presents `snapshot`, `doctor` and `tui`; legacy scaffold `status`/`capabilities` remain compatibility aliases. The internal workspace version remains pre-release. There is no published stable release to migrate, and no installed daemon service is removed by application startup. Historical task/worklog records remain intact.

## ADR-023 — bounded interactive collector lifecycle (2026-10-16)

**Status:** implemented proposal for AG-01 / issue #43, pending maintainer integration. This refines ADR-022's lifecycle policy without changing source or snapshot contracts.

**Context:** Refresh cancellation woke the interval wait, but `Worker::shutdown` unconditionally joined a synchronous sample. A blocked read could therefore retain the terminal application indefinitely. One snapshot slot and file byte limits bounded memory, not read time. An in-process completion message would not prove that thread-local/captured destructors had finished.

**Decision:** Keep one standard thread and one latest-snapshot mailbox. Acquire process-wide collector admission before spawning, release it only after an actual join or spawn failure, and permanently close admission until process exit after any shutdown timeout. Serialize cancellation, batch admission and publication through the mailbox mutex; hold no mailbox lock across source reads. A batch admitted before cancellation may finish remaining reads if resumed, but cannot publish or start another batch.

The TUI checks a fixed 5-second collection-batch deadline every event-loop iteration (normally up to 50ms input polling), including while paused and before a first sample. The refresh wait is outside that deadline. An overdue or late-completed batch is discarded and reported as `io::ErrorKind::TimedOut`, not fabricated per-source readings. The event loop also rechecks the deadline after observing a finished worker so a racing late completion retains its timeout reason.

Restore the terminal before worker cleanup on every return path. Cancellation wakes interval waiting; shutdown checks `JoinHandle::is_finished` for at most a 100ms userspace grace, joining only a finished thread. On timeout, detach the one unfinished worker, return an explicit `TimedOut`, consume its handle and cache the outcome so repeated shutdown/Drop does not repeat the wait. The CLI reports runtime failure (exit 2); callers cannot start a replacement in this process even if the detached worker later returns.

**Alternatives:** Unconditional join can hang. Releasing admission from the callback or a completion notification is premature because capture/TLS destruction may still block. Repeated detached replacement workers can exhaust resources. An async wrapper cannot cancel a synchronous kernel read. A separate helper process could improve isolation but adds transport/lifecycle cost and does not guarantee termination of an uninterruptible kernel task; no new helper, daemon, dependency or privilege is warranted for this slice.

**Limits and migration:** This bounds application waiting on samples under ordinary scheduling, not OS scheduling, terminal I/O, pathological thread teardown or hard real-time process death. An arbitrary kernel read remains uncancellable; snapshot/doctor remain synchronous with no read deadline. Process restart is required after a shutdown timeout. Snapshot JSON 0.1, source availability reasons, polling flags and pure Core/protocol APIs are unchanged. Fixtures must remain trusted/static. No new unsafe code, dependency, runtime network access or host configuration is introduced.

**Verification:** Deterministic blocked callbacks exercise explicit timeout, repeated shutdown/Drop, permanent admission, late publication suppression, deadline boundary, captured-destructor stalls and panic cleanup. Linux PTY subprocesses exercise q/raw Ctrl-C/SIGINT/SIGTERM and the no-input watchdog at a 60000ms interval, requiring exact termios/cursor/alternate-screen restoration before diagnostics. These callbacks are simulations, not uninterruptible-kernel or performance validation. Actual commands, independent review and native CI belong to [issue #43 evidence](../.AGENTS/worklogs/issue-43.md).

## ADR-024 — typed network inventory contracts and source availability (2026-10-17)

**Status:** proposed for Task AG-02 in `lidb-core`.

**Context:** LiDB requires deep network diagnostics (interfaces, routes, socket tables) without violating its core architectural principles: pure platform-agnostic core contracts, bounded resource use, truthful representation of missing or privileged data, and resistance against terminal injection attacks. This entry is renumbered from its candidate's ADR-022 to avoid colliding with PR #35's existing Core decision; ADR-023 belongs to lifecycle PR #45. These vectors allocate; no zero-allocation implementation is claimed.

**Decision:** Define structured network records (`InterfaceRecord`, `RouteRecord`, `SocketRecord`) and a bounded `NetworkReport` (schema `0.1`) in `lidb-core::network`. Differentiate empty tables from failed source reads using `SourceAvailability` (`Available`, `Unsupported`, `Disabled`, `PermissionDenied`, `TemporarilyUnavailable`, `Error`). Enforce address family consistency (destination vs gateway, local vs remote endpoint), bounded prefix lengths, bounded table sizes (64 interfaces, 1,024 routes, 2,048 sockets), and strip or reject ASCII control characters.

**Alternatives:** Combining network inventories into `Snapshot` as pseudo-metrics would obscure topology and bloat scalar metrics. Fabricating zeroes or empty tables when `/proc/net/*` is unreadable would mislead operators regarding security and visibility. Dynamic unbounded vectors would introduce memory-exhaustion vectors in busy environments.

**Consequences:** Downstream collectors (`lidb-collect`) and terminal renderers (`apps/lidash`) have strongly-typed contracts for network data with clear provenance and drop accounting. Unprivileged environments truthfully report `PermissionDenied` or `Unsupported` rather than producing misleading empty state.

## ADR-025 — route report provenance and lossless conversion (2026-10-23)

**Status:** implemented proposal in issue #49, explicitly stacked on route PR #48 and contract PR #46; integration/phase acceptance pending.

**Context:** `routes --json` serialized only route/status/drop fields, losing live/fixture provenance, source IDs and timing. Text concealed reasons and described failed reads as an empty observation. `populate_into` discarded upstream omissions and relabeled any report context. A new `NetworkReport` also claimed all sources were available before any read.

**Decision:** Route-only JSON declares schema `0.1`, mode, collector-local batch time/duration, clock source and stable family source IDs, retaining its existing fields. Start offset and duration derive from one captured `Instant`, including any scheduling gap before source reading. The CLI renders that report's own mode and each family reason; empty observation is distinguished from failure and omitted records. Uncollected generic tables default to explicit disabled/not-collected states. Keep numeric Snapshot JSON `0.1` unchanged.

`populate_into` returns `Result`, rejects known time/mode/window or asserted-namespace mismatch atomically, and rejects unequal family states because one aggregate state cannot represent them losslessly. It appends retained records and saturating upstream omissions, while destination-capacity omissions remain separately counted in the same total. Matching timestamps cannot establish common clock domains; callers must already know that both reports belong to the same collector. Standalone `collect_routes` has a fresh single-call domain at zero; `LinuxCollector::routes` shares its collector's snapshot clock.

**Alternatives:** Passing only a mode to the renderer leaves JSON ambiguous. Silently choosing one family's success/error or treating a lost prefix as an empty table hides evidence. Adding another transport, listener or generic imported-report deserializer does not solve provenance and is outside this local slice.

**Migration / limits:** These APIs are in unmerged/pre-stable candidates, not a published stable contract. Route JSON gains additive metadata; `populate_into` callers must handle its typed result, match the known context and retain independent family states when conversion is rejected. No new dependency, privilege, unsafe code, source read, listener or host change. Procfs scope is the current process view; namespace identity, route selector/flag semantics, temporary parser retention, cross-domain identity and hard read deadlines remain open. No complete route-policy lookup, connectivity or production overhead is inferred. Actual evidence is in [issue #49](../.AGENTS/worklogs/issue-49.md).
## ADR-026 — IPv6 route selectors, route flags, and bounded parser retention (2026-10-31)

**Status:** implemented proposal in issue #50, explicitly stacked on report PR #51, route PR #48, and contract PR #46; integration/phase acceptance pending.

**Context:** Linux `/proc/net/ipv6_route` parsing in PR #48 extracted destination, gateway, and metric, but ignored source selector columns 2/3 (`src_ip`, `src_prefix`) and route flags column 8 (`flags`). Consequently, two source-address-dependent routing (SADR) entries with identical destinations but distinct source prefixes collapsed into identical `RouteRecord` values, falsely triggering duplicate-route errors and invalidating the entire IPv6 table. Furthermore, routes differing only in flags (such as `RTF_UP` vs `RTF_REJECT`) collided as duplicates. The parsers also lacked upfront size bounds on public entry points, allocated intermediate line vectors, and truncated records post-parse without validating malformed overflow tails.

**Decision:**
1. Extend `RouteRecord` in `lidb-core::network` with `source_prefix: Option<IpAddr>`, `source_prefix_len: Option<u8>`, and `flags: Option<u32>`, deriving `PartialOrd, Ord, Hash`. Address family consistency between destination and source prefix is strictly validated.
2. In `lidb-collect::route`, enforce `MAX_SOURCE_BYTES` (1 MiB) and reject ASCII control characters on public parser entry points.
3. Stream lines lazily in `parse_v4_routes` and `parse_v6_routes`, preserving IPv4 flags (col 3), IPv6 source selectors (cols 2 & 3), and IPv6 flags (col 8).
4. Bound retained records and deduplication lookups using a `BTreeSet<RouteRecord>` capped at `MAX_NETWORK_ROUTES` (1,024).
5. Retain omission counts in `ParsedRoutes.dropped_routes`. Validate syntax and column constraints on all overflow tail lines, ensuring malformed lines fail with `SourceAvailability::Error` rather than being silently discarded.
6. In `apps/lidash`, format source prefix selectors (`from <src>/<len>`) when present, adhering to Linux `ip -6 route` conventions.

**Alternatives:** Treating SADR routes or reject routes as unsupported would losslessly fail valid Linux configurations. Silently dropping excess lines without syntax validation would permit corrupt data sources to pass undetected when tables exceed 1,024 entries.

**Migration / limits:** `RouteRecord` gains additive fields and accessors. Public parser functions `parse_v4_routes` and `parse_v6_routes` return `Result<ParsedRoutes, SourceAvailability>`, where `ParsedRoutes` dereferences to `[RouteRecord]` and implements `IntoIterator`. Actual evidence is recorded in [issue #50](../.AGENTS/worklogs/issue-50.md).

## ADR-027 — read-only network interface metadata and namespace collection (2026-11-06)

**Status:** implemented proposal in issue #53 (Task AG-04), explicitly stacked on PR #52 (`agent/antigravity/issue-50-routes`).

**Context:** Operators need read-only network interface metadata (index, MTU, operational state, MAC address, link speed, decoded flags) and best-effort network namespace context without requiring netlink, root privileges, or external network tools (`ip link`, `ifconfig`). Linux exposes interface attributes in `/sys/class/net/<iface>/` and namespace descriptors in `/proc/self/ns/net`. Collection must remain size-bounded, truthful when devices lack speed or disappear during iteration, and strictly protected against fixture directory traversal and symlink escapes.

**Decision:**
1. Implement `lidb-collect::interface` module collecting interface metadata from `/sys/class/net` and best-effort namespace identity from `/proc/self/ns/net`.
2. Map `operstate` strings to `InterfaceOperState` with standard fallback to `Unknown`. Parse link speed in Mbps from `speed`, treating missing, negative (`-1` on loopback or disconnected ethernet), or unsupported speed as `None` without collection error.
3. Decode standard Linux interface flags from `<linux/if.h>` (`UP`, `BROADCAST`, `LOOPBACK`, `RUNNING`, `MULTICAST`, etc.).
4. Detect namespace identity from `/proc/self/ns/net` via stat inode or `net:[<inode>]` target into `NetworkNamespace`.
5. Enforce `MAX_NETWORK_INTERFACES = 64` capacity limits, tracking excess devices in `dropped_interfaces`. Handle disappearing interfaces during iteration gracefully without aborting the batch.
6. Enforce strict symlink security: live sysfs symlinks are verified to remain within sysfs, and fixture-mode symlinks escaping the fixture root are rejected.
7. Expose `InterfaceReport` (schema `0.1`) with `populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError>` matching the route report contract.
8. Add CLI `interfaces` command (aliases: `interface`, `iface`) and `--json` format to `apps/lidash`.

**Alternatives:** Shelling out to `ip link` introduces subprocess execution, shell injection risks, and external dependencies. Loosening fixture symlink checks would allow path traversal vulnerabilities.

**Migration / limits:** Pure additive feature. Non-Linux platforms report `SourceAvailability::Unsupported` cleanly. Actual evidence is in [issue #53](../.AGENTS/worklogs/antigravity-ag-04.md).

## ADR-028 — read-only TCP and UDP socket observations (2026-11-06)

**Status:** implemented proposal in issue #55 (Task AG-05), explicitly stacked on PR #54 (`agent/antigravity/ag-04-interfaces`).

**Context:** Operators need read-only network socket visibility (protocol, local and remote endpoints, connection states, and socket inodes) across IPv4 and IPv6 without requiring netlink, root privileges, or external network tools (`ss`, `netstat`). Linux exposes TCP and UDP sockets in `/proc/net/{tcp,tcp6,udp,udp6}`. Socket collection must remain strictly bounded, truthful when sources fail or are denied, and respectful of operator privacy boundaries (no socket-to-PID mapping or process environment inspection).

**Decision:**
1. Implement `lidb-collect::socket` module parsing `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, and `/proc/net/udp6` into structured `SocketRecord` entries.
2. Parse IPv4 addresses from 8-hex little-endian procfs representations into `IpAddr::V4`.
3. Parse IPv6 addresses from four 32-bit words formatted in host byte order, converting them to canonical network octets via `.to_le_bytes()`.
4. Parse connection states via standard Linux TCP state mappings into `SocketState`.
5. Map unconnected endpoints (`0.0.0.0:0` or `:::0`) to `remote_address: None` and `remote_port: None`.
6. Enforce strict privacy: socket collection reads only metadata from procfs network tables; it never inspects `/proc/<pid>` or attempts to correlate sockets to PIDs.
7. Enforce `MAX_NETWORK_SOCKETS = 2048` capacity bounds, tracking excess records in `dropped_sockets`. Validate malformed overflow records even when capacity is exceeded.
8. Distinguish empty tables (`Available` with 0 records) from missing or permission-denied sources.
9. Expose `SocketReport` (schema `0.1`) with `populate_into(&self, report: &mut NetworkReport) -> Result<(), NetworkError>`.
10. Add CLI `sockets` command (aliases: `socket`, `sock`) and `--json` format to `apps/lidash`.

**Alternatives:** Shelling out to `ss` or `lsof` requires external binaries and privileges, and scanning `/proc/<pid>/fd` violates privacy constraints and risks high overhead.

**Migration / limits:** Pure additive feature. Non-Linux platforms report `SourceAvailability::Unsupported` cleanly. Actual evidence is recorded in [issue #55](../.AGENTS/worklogs/antigravity-ag-05.md).

## ADR-029 — unified network report collection and interactive terminal dashboard (2026-11-06)

**Status:** implemented proposal in issue #57 (Task AG-06), explicitly stacked on socket PR #56, interface PR #54, and route PR #52; integration/phase acceptance pending.

**Context:** Phase 2 introduced distinct network collection modules for routes (`lidb-collect::route`), interface metadata and namespace attribution (`lidb-collect::interface`), and socket endpoints (`lidb-collect::socket`). Operators require a unified collection entry point (`LinuxCollector::network`) that captures full host network diagnostics in a single coherent snapshot with a shared monotonic timestamp domain. Furthermore, terminal operators need interactive visibility into interfaces, routes, sockets, and namespace identity directly in `lidash` without navigating multiple standalone CLI invocations.

**Decision:**
1. Implement `LinuxCollector::network(&self) -> NetworkReport` in `lidb-collect`, populating interfaces, routes, and socket records into a consolidated `NetworkReport` (schema `0.1`).
2. Share a single monotonic timestamp domain (`monotonic_ns`) across all three sub-collections within the unified network sample, measuring overall batch collection duration.
3. Handle route population gracefully: if `populate_into` reports `RouteSourceStatusMismatch` due to unequal IPv4/IPv6 source states, preserve all observed routes and populate the report with the consolidated status rather than discarding routing data.
4. Add the `network` command (alias: `net`) with `--json` format support to `apps/lidash`. Render structured textual summaries in `output::network_text` detailing active network namespace, interfaces, routing table entries, and socket states.
5. In `apps/lidash::tui`, introduce `Tab::Network` (key '3' or Tab/Left/Right cycling). Render dedicated interface, route, and socket tables alongside namespace identity and source status indicators.
6. Gracefully adapt the interactive TUI to narrow and small terminal viewports (`area.height <= 10`), rendering compact summaries without clipping or control character injection.

**Alternatives:** Keeping routes, interfaces, and sockets exclusively as separate sub-commands forces operators to correlate timestamps manually and prevents unified interactive monitoring in the TUI.

**Migration / limits:** Pure additive feature. Non-Linux platforms report `SourceAvailability::Unsupported` cleanly. Actual evidence is recorded in [issue #57](../.AGENTS/worklogs/antigravity-ag-06.md).

## ADR-030 — offline diagnostic evidence export and bounded terminal history (2026-11-06)

**Status:** implemented proposal in issue #59 (Task AG-07), explicitly stacked on PR #58 (`agent/antigravity/ag-06-network-ui`); integration/phase acceptance pending.

**Context:** Operators troubleshooting degraded Linux hosts or capturing support bundles require a consolidated, self-contained evidence export command that packages host metric observations, capability diagnosis (`doctor`), and full network telemetry into a structured JSON or human-readable format. Furthermore, when analyzing transient spikes or regressions in the interactive TUI, operators need time-travel history navigation through recent samples without infinite memory growth or unbounded retention.

**Decision:**
1. Implement the `export` command (alias: `bundle`) in `apps/lidash`, supporting `--output PATH` and `--json`.
2. Package evidence into `EvidenceExport` (schema `0.1`, type `offline_diagnostic_evidence`) combining host snapshot, source doctor capabilities, and unified network report.
3. Explicitly declare strict privacy boundaries in `EvidenceExport::privacy_declaration`: non-root, read-only, metadata-only; zero packet payloads, zero decrypted TLS, zero environment variables, zero process arguments, and zero secrets.
4. If `--output PATH` is provided, stream output into a buffered file writer safely; otherwise write directly to stdout. Return exit code matching source capability usability (0 if usable/degraded, 1 if no metrics available).
5. In `apps/lidash::tui`, implement an in-memory bounded ring buffer capped at `MAX_VIEW_HISTORY = 120` samples (~2 minutes of history at 1s intervals, ~700 KB RAM).
6. Enable time-travel history inspection via `[` / `,` (step backward) and `]` / `.` (step forward), auto-pausing live updates.
7. Display historical offset and sample age in the header bar (`PAUSED [-N/total]`), and snap back to live streaming immediately when `Space` unpauses.

**Alternatives:** Ad-hoc shell scripting or piping multiple commands together introduces timestamp skews and lacks standard privacy declarations. Unbounded in-memory history risks OOM on constrained hosts.

**Migration / limits:** Pure additive feature. Non-Linux platforms report `SourceAvailability::Unsupported` cleanly. Actual evidence is recorded in [issue #59](../.AGENTS/worklogs/antigravity-ag-07.md).

## ADR-031 — read-only eBPF lifecycle instrumentation and unprivileged fallback (2026-11-07)

**Status:** implemented in Task AG-08 (Phase 4).

**Context:** Operators troubleshooting high-frequency connection churn, transient latency spikes, and socket lifecycle transitions (e.g. `SYN_SENT -> ESTABLISHED`, `CLOSE_WAIT`, `FIN_WAIT_1`, `RESET`) require deeper visibility than periodic polling of `/proc/net/tcp`. eBPF enables low-overhead, in-kernel event capture without polling delays. However, eBPF requires kernel support (>= 4.16), BTF (`/sys/kernel/btf/vmlinux`), and specific permissions (`CAP_BPF` or `CAP_SYS_ADMIN`), and must never violate operator privacy or crash on unprivileged hosts or non-Linux platforms.

**Decision:**
1. Define pure platform-agnostic eBPF contracts in `crates/lidb-core/src/ebpf.rs` (`EbpfEventType`, `EbpfLifecycleEvent`, `EbpfReport`, `EbpfError`).
2. Implement safe C CO-RE program `crates/lidb-collect/bpf/tcp_lifecycle.bpf.c` hooking tracepoint `sock:inet_sock_set_state` via a 256KB ring buffer. Capture strictly metadata (family, endpoints, state transition, duration/latency), zero payload, zero headers, zero memory reads.
3. Implement userspace loader, capability probing, and detach in `crates/lidb-collect/src/ebpf.rs` and `LinuxCollector::ebpf`.
4. Enforce strict capability checks: check `/sys/kernel/btf/vmlinux` and BPF permissions. Report typed availability (`Unsupported` when BTF or OS is missing/non-Linux, `PermissionDenied` when unprivileged, `Available` when attached).
5. Enforce bounded retention: ring buffer cap of `MAX_EBPF_EVENTS = 2048` per report, reporting `dropped_events` on overflow.
6. Add CLI command `ebpf` (aliases: `bpf`, `events`) and `--ebpf` flag in `apps/lidash`, with JSON and human-readable text formatting.
7. Support deterministic offline fixture playback for mock testing and verification across non-Linux and CI systems.

**Alternatives:** Kprobes (`tcp_v4_connect`) are unstable across kernel versions; packet capture (libpcap) violates read-only metadata privacy contracts and imposes high CPU overhead. Tracepoints with CO-RE are stable, safe, and kernel-verified.

**Migration / limits:** Pure additive feature. Non-Linux and unprivileged Linux systems degrade cleanly to typed availability without error.

## ADR-032 — standalone installer, packaging distribution, and verified support matrix (2026-11-08)

**Status:** implemented in Task AG-10 (Phase 5).

**Context:** With the core diagnostics, networking telemetry, terminal experience, and optional eBPF instrumentation completed, operators require a secure, reliable, and standardized mechanism to install, verify, upgrade, and cleanly remove `lidash` on production Linux systems without external package manager dependencies or root escalation requirements. Furthermore, open-source operators require full supply-chain transparency, license compliance verification (SBOM), and an explicit kernel compatibility matrix.

**Decision:**
1. Provide an official POSIX-compliant standalone installer and uninstaller script at `scripts/install.sh`. It automatically detects CPU architecture (`x86_64` and `aarch64`), verifies SHA-256 cryptographic checksums before file placement, installs to `/usr/local/bin` (root) or `~/.local/bin` (unprivileged), and supports `--uninstall` and `--dry-run`.
2. Document end-to-end binary distribution, manual checksum verification, system requirements, and clean uninstallation in `docs/PACKAGING.md`.
3. Provide a complete Software Bill of Materials (SBOM) and third-party license audit in `docs/THIRD_PARTY_LICENSES.md`, confirming that all statically linked libraries use permissive open-source licenses (MIT, Apache-2.0) with zero copyleft or restrictive licenses.
4. Define hardware tiers, Linux kernel version compatibility (from Linux 2.6+ baseline through Linux 4.16+ eBPF), and container runtime environments (Docker, Podman, Kubernetes, WSL2) in `docs/SUPPORT_MATRIX.md`.
5. Maintain zero system modifications: `lidash` remains a single self-contained executable creating zero daemons, services, or config databases.

**Alternatives:** Distributing solely via package managers (.deb/.rpm) introduces maintenance overhead across distinct distributions and delays security patches. Distributing unverified raw binaries without checksum manifests leaves operators vulnerable to incomplete downloads.

**Migration / limits:** Completely backward compatible. No breaking changes to existing CLI flags or schema contracts.

## ADR-033 — interactive eBPF socket lifecycle dashboard tab in TUI (2026-11-09)

**Status:** implemented in Task AG-14 (Phase 5).

**Context:** While ADR-031 introduced offline and CLI-level eBPF socket lifecycle collection via `cargo run -p lidash -- ebpf --json`, operators using the interactive TUI (`lidash tui`) lacked a live, real-time dashboard view for observing in-kernel TCP state transitions (`sock:inet_sock_set_state`). Operators diagnosing connection storms, transient connection resets, or TCP handshake stalls need to inspect active state transitions, process contexts, socket endpoints, and RTTs concurrently with host metrics and network topology.

**Decision:**
1. Extend the TUI view model with Tab `4: eBPF` (`Tab::Ebpf`), expanding the tab bar from 3 to 4 tabs (`1: Metrics`, `2: CPU/Mem`, `3: Network`, `4: eBPF`).
2. Add direct keybinding `'4'` alongside `Tab` and `Shift+Tab` / arrow navigation.
3. Thread eBPF sampling into the background worker and update loop when `--ebpf` is enabled, capturing `collector.ebpf(true)` concurrently with metric snapshots and network reports.
4. Implement dedicated `draw_ebpf` UI view:
   - Header summary banner showing tracepoint name (`sock:inet_sock_set_state`), source availability, event counts, dropped event count, and ring buffer capacity (256KB).
   - Graceful fallback cards displaying typed reasons when disabled (with hint to run with `--ebpf`), permission denied (`CAP_BPF` or `CAP_SYS_ADMIN`), or unsupported platform (missing BTF / non-Linux).
   - Structured tabular view of recent socket lifecycle events displaying Event type, Process comm & PID, Address Family, Source endpoint, Destination endpoint, TCP State Transition (`old -> new`), and Round-Trip Time (`RTT`).
   - Responsive layout adapting to small terminals, compact heights (<= 10 rows), and wide displays.
5. Guarantee `#![forbid(unsafe_code)]` compliance, zero heap allocations on rendering hot paths, and graceful degraded states on unprivileged systems.

**Alternatives:** Embedding eBPF events inside the Network tab would overcrowd the interface and degrade socket table readability. A separate standalone binary would fragment the operator UX.

**Migration / limits:** Completely backward compatible. Purely additive to TUI navigation. Defaults to disabled unless `--ebpf` flag is supplied.




