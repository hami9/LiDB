# LiDB TUI Prototype (`prototypes/tui`)

**Issue:** [#8](https://github.com/hami9/LiDB/issues/8) — *P0: Antigravity isolated Ratatui TUI prototype*  
**Agent Branch:** `agent/antigravity/p0-tui`  
**Worktree:** `.worktrees/antigravity/p0-tui`  

A modular, terminal-native Rust dashboard engineered with **Ratatui 0.30** and **Crossterm 0.29**. This prototype delivers the complete visual layout, keyboard navigation, and diagnostic state engine for the LiDashBoard (`LiDB`) P0 milestone.

The Issue [#17](https://github.com/hami9/LiDB/issues/17) adapter adds a local `lidb-core` dependency. The prototype remains a separate Cargo workspace, but building this integration slice requires `crates/lidb-core` and the root workspace alongside it. See the [capability ADR](../../docs/adr/0001-core-capabilities.md).

---

## 1. Multi-Agent Coordination & Workspace Isolation

In compliance with `.AGENTS/WORKTREES.md` and the LiDB multi-agent isolation policy:
- **Zero Central Conflicts:** Built as an independent crate in `prototypes/tui/`. Root `Cargo.toml`, `crates/lidb-core/`, `crates/lidb-protocol/`, `apps/lidash/`, `apps/lidashd/`, and shared `.AGENTS/STATE.md` remain untouched.
- **Parallel Integration:** Coordinates with [PR #9](https://github.com/hami9/LiDB/pull/9) (`agent/chatgpt/p0-core`), which defines the core contracts. The TUI prototype models these domain contracts via clearly delineated simulated fixtures and typed unsupported states.
- **Dedicated Worktree:** Authored and tested solely within the isolated Git worktree `agent/antigravity/p0-tui`.

---

## 2. Implemented Subsystems & Views

The dashboard provides a keyboard-driven interface structured into 7 dedicated tabs:

1. **`1:System` — System Overview:**
   - Host identity: Hostname, OS name, Kernel version, Architecture, Uptime, 1m/5m/15m Load Average.
   - Coordination & Governance: Active agent ID, sandbox execution status, and adherence to LiDB phase gates.
   - Prominent telemetry provenance badge (`[SIMULATED FIXTURE]` vs `[LIVE TELEMETRY]`).

2. **`2:CPU/Mem` — Host Compute & Memory Pressure:**
   - Overall CPU gauge with dynamic color thresholds and recent activity sparkline.
   - Core topology matrix: Real-time utilization and clock frequency per logical core.
   - Memory breakdown: Physical RAM (Used, Free, Available, Buffers, Cached) and Swap space.
   - Linux PSI (Pressure Stall Information) metrics: Simulated fixtures advanced by `FixtureManager` (CPU, Memory, I/O stall averages), marked with `[SIMULATED FIXTURE]` badges.

3. **`3:Network` — Linux Networking:**
   - Linux network interfaces table: Interface name, state (UP/DOWN), IP addresses, MAC, MTU, RX/TX byte and packet rates, drop/error counters.
   - Real-time traffic trend sparklines for inbound (RX) and outbound (TX) throughput.
   - Protocol socket overview (TCP Established, TCP Listen, TCP TimeWait, UDP sockets).

4. **`4:GPU/AI` — Accelerators & Model Serving Workloads:**
   - **Host Reality Mode (Default):** Displays the core GPU capability state/reason separately from telemetry. The bootstrap registry reports `[DISABLED]` with `Collector not implemented in P0`, while telemetry remains `[NOT PROBED]`. The core registration source is visible; it is not a host probe or a live GPU sample.
   - The adapter preserves `Available`, `Unsupported`, `Disabled`, `PermissionDenied`, `TemporarilyUnavailable`, `Stale`, and `Error`. Missing registration stays `[NOT PROBED]`; available capability never manufactures numeric telemetry. `App::with_capabilities` accepts an explicit registry snapshot for deterministic integration tests.
   - **Simulated DGX Spark Fixture (`g` key):** High-fidelity simulation of a 2-node NVIDIA DGX Spark cluster with GB10 Grace Blackwell accelerators (SM utilization, 128 GB coherent LPDDR5x unified system memory pressure, NVLink-C2C intra-node interconnect, ConnectX-7 200GbE RoCEv2 inter-node fabric, realistic power draw within 140 W SoC TDP).
   - AI serving KPIs: Simulated vLLM and TensorRT-LLM telemetry (tokens/sec, TTFT, TPOT, KV cache utilization %, batch sizes).
   - Interconnect topology contract (**Rule R03**): Spark CPU↔GPU NVLink-C2C is strictly modeled as intra-node; inter-node communication uses ConnectX-7 Ethernet/RoCE (no external GPU-to-GPU NVLink).

5. **`5:Processes` — Interactive Process Table:**
   - Interactive process inspection: PID, User, PR/State (`R`, `S`, `D`, `Z`), CPU %, MEM %, RSS, Threads, I/O rates, Command line.
   - Live search filter: Press `/` to enter search mode (filters by command, user, or PID).
   - Sorting: Press `s` to cycle sorting by CPU %, MEM %, PID, or Command.
   - Row inspection: Press `Enter` to open an in-depth process inspection modal.
   - Stateful scrolling: Table selection automatically scrolls via `TableState`.

6. **`6:Diagnostics` — Automated Health Checks:**
   - Truthful environment and demo assessments (`SIM-PASS`, `UNPROBED`, `WARN`, `FAIL`, `N/A`):
     - `DIAG-01`: procfs Base Telemetry (`UNPROBED` in standalone TUI prototype; scheduled for P1).
     - `DIAG-02`: eBPF Subsystem Access (`N/A` in unprivileged standalone prototype; fallback active).
     - `DIAG-03`: cgroup v2 Controllers (`UNPROBED` in standalone prototype).
     - `DIAG-04`: Linux Netlink Routing (`UNPROBED` in standalone prototype).
     - `DIAG-05`: NVML Hardware Runtime (`N/A` pending P5 accelerator capability probe).
     - `DIAG-06`: Security & Privacy Contract (`SIM-PASS` demo check for Rule R07 zero payload policy; runtime audit not claimed).
     - `DIAG-07`: Unprivileged Execution (`SIM-PASS` demo check for Rule R05 non-root baseline; privileges not audited).
   - Remediation panel: Contextual remediation guidance for every check.

7. **`7:Settings/Help` — Configuration & Shortcuts:**
   - Real-time display parameters: Color theme selector, telemetry streaming toggle, terminal geometry.
   - Complete keyboard shortcut cheatsheet.
   - Architectural references and links to repository contracts.

---

## 3. Keyboard Navigation

| Key | Action |
| --- | --- |
| `1` .. `7` | Direct jump to tab 1 through 7 |
| `Tab` / `Right` / `l` | Switch to next tab |
| `BackTab` / `Left` / `h` | Switch to previous tab |
| `Down` / `j` | Select next row in tables / lists |
| `Up` / `k` | Select previous row in tables / lists |
| `Enter` | Open detailed inspection modal for selected process or diagnostic check |
| `/` | Activate process text search filter (on Tab 5) |
| `s` | Cycle process sorting order (CPU% → MEM% → PID → Command) |
| `Space` | Pause / resume telemetry updates |
| `g` | Toggle GPU tab between Host Reality (unprobed) and Simulated DGX Spark Fixture |
| `t` | Cycle visual theme (Dark → Light → High-Contrast → Monochrome) |
| `?` / `F1` | Open interactive Help Modal overlay |
| `Esc` / `q` | Dismiss open modal or exit the application |

---

## 4. Architectural Rules & Compliance

| Rule | Implementation Guarantee |
| --- | --- |
| **R01 / R14** | **Truthful Metrics:** All mock telemetry is explicitly stamped with `[SIMULATED FIXTURE]` badges. Live, unprobed, and unsupported states are never conflated. |
| **R03** | **DGX Spark Topology:** NVLink-C2C is documented and displayed as intra-node only. Inter-node DGX fabric uses ConnectX-7 Ethernet/RoCE. |
| **R05 / R06** | **Non-Root Baseline:** Operates completely unprivileged without `CAP_SYS_ADMIN` or `CAP_NET_ADMIN`. |
| **R07** | **Zero Payload Snooping:** Telemetry models restrict strictly to performance metadata; zero packet payload or secret inspection. |
| **R10 / R19** | **Container / Host Portability:** Gracefully identifies unprobed or missing capabilities using typed `NotProbed` / `Unsupported` enums. |
| **R25 / R27** | **Worktree Invariants:** Confined to `agent/antigravity/p0-tui` without modifying central shared agent state. |

---

## 5. Verification & Testing

### Running Automated Test Suites
```bash
cargo test --manifest-path prototypes/tui/Cargo.toml --locked --all-targets
```
The test suite validates:
- Core capability states, reasons, absent registration, snapshot ownership and GPU telemetry separation (`tests/core_adapter_tests.rs`).
- Direct and cyclic tab navigation (`tests/app_tests.rs`)
- Process filtering, sorting, and modal interactions (`tests/app_tests.rs`)
- Deterministic telemetry simulation and bounds checking (`tests/fixture_tests.rs`)
- DGX Spark GB10 realistic power and unified memory bounds (`tests/fixture_tests.rs`)
- Truthful unprobed and simulated diagnostic states (`tests/fixture_tests.rs`)
- Full headless rendering of all 7 tabs and modals via Ratatui `TestBackend` (`tests/render_tests.rs`)
- Graceful terminal resize fallback under small dimensions (`tests/render_tests.rs`)
- Color theme generation across Dark, Light, High-Contrast, and Monochrome modes (`tests/theme_tests.rs`)

### Running Formatting and Linter
```bash
cargo fmt --manifest-path prototypes/tui/Cargo.toml -- --check
cargo clippy --manifest-path prototypes/tui/Cargo.toml --all-targets --locked -- -D warnings
```

### Running Non-Interactive Headless Tests
```bash
# Validates headless rendering across all views:
cargo run --manifest-path prototypes/tui/Cargo.toml -- --headless-test

# Runs automated smoke test (5 ticks) without an interactive terminal:
cargo run --manifest-path prototypes/tui/Cargo.toml -- --smoke-test
```

### Interactive Execution
```bash
cargo run --manifest-path prototypes/tui/Cargo.toml -- --tick-rate 500 --theme dark
```
