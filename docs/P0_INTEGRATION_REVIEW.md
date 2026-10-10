# P0 Integration Review

Reviewed on 2026-10-08 by Codex for P0 CI & TUI Integration. This is an architectural, cross-workspace compatibility, and CI integration review, not approval to merge or a completed P0 milestone.

## Reviewed PR Snapshots

| PR | Owner | Exact Reviewed Head | Integration Boundary |
| --- | --- | --- | --- |
| [#9](https://github.com/hami9/LiDB/pull/9) | ChatGPT | `86f22115a85633f46a0ad32d2ef63e3d23ad3947` | Root workspace, typed core contracts (`crates/lidb-core`, `crates/lidb-protocol`), CLI/daemon scaffolds |
| [#11](https://github.com/hami9/LiDB/pull/11) | ChatGPT | `4a2a8d234721be701395c34308f7454e207eb234` | Bounded length-prefixed IPC byte framing (`crates/lidb-protocol/src/framing.rs`) |
| [#13](https://github.com/hami9/LiDB/pull/13) | Antigravity | `54b0d03c6ed5585cec6f2666936801244fe2a798` | Independent `prototypes/tui/` workspace, standalone fixtures, and terminal UI |

## Cross-Workspace Compatibility & Reproducibility (Task B)

1. **Root Workspace & TUI Workspace Coexistence**:
   - The addition of `[workspace]` in `prototypes/tui/Cargo.toml` prevents nested workspace membership conflicts.
   - Verified that root crates (`lidb-core`, `lidb-protocol`, `lidash`, `lidashd`) build and pass all 14 unit/CLI tests cleanly alongside the separate `prototypes/tui` workspace.
   - Verified that `prototypes/tui` builds and passes all 20 unit, fixture, render, and theme tests under locked dependencies.
2. **Cargo Lockfile Isolation**:
   - `prototypes/tui/Cargo.lock` operates independently from the root `Cargo.lock`.
   - CI detection (`scripts/ci/check_tui.py`) enforces that any present TUI prototype must commit a valid `Cargo.lock`.
3. **No File Overlaps or Incompatible Contracts**:
   - Zero path collisions exist between `crates/`, `apps/`, and `prototypes/tui/`.

## Critical Review of Adapter Architecture (Task C)

Antigravity proposed mapping rules between `lidb-core` contracts and TUI telemetry states. A critical architectural review identifies the following requirements for future adapter implementation:

1. **`CapabilityState::Disabled` vs `SimulatedFixture`**:
   - `CapabilityState::Disabled` means a collector or capability was intentionally disabled by configuration or feature flag on the host.
   - **Requirement**: `Disabled` must NEVER automatically become `SimulatedFixture`. Simulated fixtures are an explicit offline demo/test mode (`--fixture` or `GpuViewMode::SimulatedFixture`). If a live host feature is `Disabled`, the UI must truthfully display `Disabled` (or `Unavailable: Disabled by configuration`), never substitute mock numbers.
2. **Distinct Meanings of `NotProbed`, `Unsupported`, and `PermissionDenied`**:
   - `NotProbed`: Probing has not yet been executed by the collector.
   - `Unsupported`: The host OS, kernel, or hardware fundamentally lacks the capability (e.g. non-NVIDIA host, kernel without PSI).
   - `PermissionDenied`: The capability exists, but the process lacks permissions (e.g. unprivileged non-root process without capabilities).
   - **Requirement**: These states must maintain separate, unambiguous badges in the UI (`[UNPROBED]`, `[UNSUPPORTED]`, `[PERMISSION DENIED]`) and must not be collapsed into generic "N/A" or "Degraded".
3. **Missing Telemetry Is Never Zero**:
   - Under Rule R01, R14, and `lidb-core/src/telemetry.rs`, missing or unavailable metrics must render as `--` or `N/A`.
   - **Requirement**: Never substitute a numeric zero (which falsely implies 0% utilization or 0 bytes). Sparklines must handle gaps or unavailable ranges without plotting false zero baselines.
4. **Observation Provenance & Timestamps**:
   - Every `MetricObservation<T>` retains a collector `source` (e.g. `linux.procfs`, `sysfs`, `nvml`) and monotonic timestamp `monotonic_ns`.
   - **Requirement**: The UI status bars, detail modals, and inspector cards must surface the observation provenance and data age.
5. **IPC Framing Is Not an Authenticated Transport**:
   - PR #11 provides 4-byte big-endian length-prefixed stream framing (`encode_frame`, `FrameDecoder`) bounded to `MAX_FRAME_BYTES = 1 MiB`.
   - **Requirement**: IPC framing provides bounded packetization only. It does not provide Unix domain socket connection lifecycle, TLS/encryption, peer authentication (`SO_PEERCRED`), or schema serialization. Future daemon client adapters must implement an authenticated transport layer on top of framing.

## Recommended Integration Order

1. **Step 1: Merge CI Infrastructure (This PR)**: Establish the required `TUI (x86_64)` and `TUI (aarch64)` gates and standalone CodeQL detection on `main`.
2. **Step 2: Merge PR #9 (`agent/chatgpt/p0-core`)**: Land root Rust workspace, typed capabilities, and telemetry contracts.
3. **Step 3: Rebase & Merge PR #11 (`agent/chatgpt/p0-framing`)**: Land bounded IPC framing on top of merged core.
4. **Step 4: Merge PR #13 (`agent/antigravity/p0-tui`)**: Land standalone Ratatui prototype, validated by the dedicated TUI CI gate.
5. **Step 5: Author P1 Adapter Task**: Implement typed adapter connecting `lidash`/TUI to `lidb-core` and `lidb-protocol` without violating provenance or data truthfulness.
