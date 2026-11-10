# Agent worklog

**Append-only factual record.** Do not write fictional completions. Record UTC timestamps, branch/commit, task ID, changed paths, commands run, outcomes, simulated vs real environment and next action.

## 2026-10-08 — Documentation and agent-workspace bootstrap

- Actor: ChatGPT (repository documentation bootstrap).
- Phase: P0 preparation; **no phase implementation completed**.
- Scope: architecture reference, product and security specifications, open-source project setup, agent guidance, delivery gates and templates.
- Environment: GitHub repository operations; no host compilation, GPU benchmark, network diagnostics or eBPF execution.
- Verification: document links and repository tree review planned at handoff; **runtime/unit/hardware tests NOT RUN** because no application implementation exists.
- Risks/open decisions: minimum kernel/toolchain, IPC format, exact telemetry schema and hardware validation matrix.
- Next action: maintainer selects P0 first vertical slice, then an agent creates tested Rust workspace scaffolding.

## 2026-10-08 — CI/CD infrastructure proposal

- Actor: ChatGPT (automation implementation), on branch `ci/automated-release-and-guardrails` and PR #2.
- Phase: P0 preparation, **runtime implementation still NOT STARTED**.
- Scope: SemVer planner/tests; main-only guarded release workflow; cross-architecture Rust checks; CodeQL; Dependabot; proposed main/tag rulesets; release policy.
- Environment: GitHub repository and GitHub-hosted CI; no local NVIDIA/GPU hardware access.
- Verification: CI and docs workflows are being executed on PR #2; their concrete results must be verified before merge. No real binary release was produced.
- Risk: branch ruleset application requires separate `Administration:write` GitHub authorization; no guarantee of enabled protection from JSON files alone.

## 2026-10-08 — Multi-agent worktree tooling

- Actor: ChatGPT (remote GitHub collaboration bootstrap).
- Status: Worktree manager and integration tests proposed in a feature PR; no worktree has been created on the user's Windows or Antigravity machine.
- Scope: cross-platform Python CLI, Windows/Linux wrappers, canonical agent worktree coordination policy, native Claude/Antigravity instruction files, CI smoke tests.
- Verification: GitHub CI results belong to the associated PR; actual agent sessions and cross-device synchronization are **NOT** initiated or validated from this remote workflow.
- Next: maintainer clones/updates LiDB locally and runs each agent in its own worktree after assigning distinct tasks.

## 2026-10-09 — runnable public Linux Core delivery

- Coordinator: Codex, branch `agent/codex/public-core`, issues #21–#24. The maintainer explicitly requested removal of non-general-product scope and completion of Core. ADR-021 records the revised direction. Integration and release acceptance remain maintainer decisions.
- Preserved foundation: core/protocol contracts from PR #9 and bounded history from PR #20, cherry-picked with original authorship. Other owners' remote branches and existing PRs were not modified or merged. This delivery includes those foundations and should be reviewed as one integration candidate.
- Implemented: typed bounded snapshot JSON schema 0.1; integer-preserving exports; source/reason validation; unit-consistent bounded histories; local procfs CPU/memory/swap/load/uptime/network/disk/PSI collection; actual-source doctor; text/JSON and keyboard terminal views; no-daemon startup; safe polling and terminal restoration. Removed inactive daemon scaffolding and active specialized product plans/skills.
- Ownership: isolated `public-core`, `public-collector`, `public-cli` and `public-docs` worktrees. Workers committed only assigned modules; the coordinator integrated shared schema, CI, state and evidence. Scoped evidence is in `.AGENTS/worklogs/issue-22.md`, `.AGENTS/worklogs/issue-24.md` and `apps/lidash/TASK_23.md`.
- Local environment: Debian 13.6, Linux 6.18.44 x86_64, Rust 1.85.0, unprivileged UID 1000. Actual procfs output contained 146 observations, with three unsupported PSI observations rather than fabricated readings. Fixtures are explicitly labeled and do not establish live-host validation.
- Final runtime revision reviewed: `fed5557`. Independent read-only reviewer reproduced the valid-interface-name failure and verified its fix via public CLI fixtures (`eth0`, `vpn+prod`, literal `vpn%2Bprod`, `vpn@prod`, legacy `cciss!c0d0`). Narrow paging and pause/resume findings were also fixed and regression-tested. No blocking finding remained in the reviewed scope. This is agent review, not human merge/release approval.

### Commands and actual results

All commands below exited 0 on the local environment unless otherwise stated.

| Command / observation | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked --all-targets` | PASS: 49 tests (20 core, 2 protocol, 14 collector, 13 CLI/presentation) |
| `cargo test --workspace --locked --doc` | PASS; zero doctest cases, not additional test coverage |
| `python3 -m unittest discover -s tests/ci -p 'test_*.py' -v` | PASS: 18 policy/worktree/release tests |
| `python3 scripts/check_docs.py` / `git diff --check` | PASS: local links and whitespace |
| Workflow YAML parsing with PyYAML / `bash -n scripts/release/package.sh` | PASS: syntax |
| `cargo run -p lidash --locked -- snapshot --json` and `doctor --json` | PASS: real Linux data, schema/mode/timestamps and memory/CPU invariants checked |
| `python3 apps/lidash/tests/pty_smoke.py target/debug/lidash` | PASS: q, raw Ctrl-C, SIGINT, SIGTERM, resize, exact termios/cursor/alternate-screen restoration at 60000 ms polling |
| `LIDB_BUILD_VERSION=0.1.0 cargo build --release --workspace --locked --bins` | PASS on final runtime; binary reports `lidash 0.1.0` |
| `LIDB_VERSION=0.1.0 LIDB_ARCH=x86_64 bash scripts/release/package.sh` | PASS: fresh local archive contains only lidash, README and project LICENSE |
| SHA-256 check and archive-content inspection | PASS; local test artifact only, no tag or release published |
| Independent release-version override (`0.0.1`) | PASS: binary output matches the supplied build version |
| Locked dependency metadata | 75 packages, no missing license metadata; MIT/Apache-compatible alternatives plus Zlib/Unicode terms reviewed; full release notices/SBOM/advisory automation remains future hardening |

Native x86_64/aarch64 GitHub CI and CodeQL results must be read from the delivery PR's exact head/check URLs. Configuring runners alone does not establish success. Public release publication, repository ruleset activation, measured overhead and broader Linux/container compatibility were not performed by this task.

### Five-axis review and remaining limits

- Linux correctness: guest ticks are not double-counted; CPU idle/iowait and 512-byte disk-sector accounting are documented. Parent disks/partitions and stacked interfaces must not be summed indiscriminately. Real procfs and denied-source paths passed.
- Telemetry semantics: first/new/reset/nonadvancing samples remain explicit; memory uses MemAvailable; exact counters, monotonic domains, time window, finite values and unit/source boundaries are tested.
- Privacy/permissions: read-only, no listener or active probes, no process arguments/environments/payloads. File/device/name/snapshot bounds and terminal-control validation are enforced.
- Reliability: one latest-snapshot slot, interruptible polling, capacity accounting and terminal cleanup passed. Synchronous file reads lack deadlines; custom fixture directories must be trusted and static. No production overhead claim is made.
- Operator usability: TTY-aware startup, headless aliases, availability-only doctor, scrolling details/help, narrow terminal fallback, fixture labels and quit/resize/pause behavior are tested and documented.

The requested `github style` skill was searched in the available cloud/executor catalogs and local skill files and was not present. The known GitHub title preference was applied to coordinator commits and contributor guidance; PR titles remain Conventional Commits for the existing release planner. No outside skill was installed or falsely claimed to have run.

Next action: review/integrate the tested public Core PR, then select one focused P2 Linux networking slice. Phase completion and stable-release claims remain unapproved.

## 2026-10-09 23:29 UTC — PR and native CI evidence

- Coordinator: Codex, branch `agent/codex/public-core`; [PR #25](https://github.com/hami9/LiDB/pull/25) opened for maintainer review. No merge, tag, release publication or ruleset change was performed.
- Verified remote revision: `c1dd00f263142c924830ca0b98c32ac8e6fa5189`. [CI run 38004433311](https://github.com/hami9/LiDB/actions/runs/38004433311) completed successfully: docs/policy, native Ubuntu 24.04 x86_64 and aarch64 Rust jobs, Windows worktree smoke and CI Gate. Each native Rust job ran format, warnings-as-errors Clippy, all-target tests, doctest command, real unprivileged CLI JSON and PTY lifecycle smoke. This is native runner evidence, not cross-compilation.
- [CodeQL run 38004433296](https://github.com/hami9/LiDB/actions/runs/38004433296) completed successfully for Actions and Rust; the C job correctly skipped with no C source. [Documentation run 38004433345](https://github.com/hami9/LiDB/actions/runs/38004433345) passed. Static analysis success does not certify absence of vulnerabilities.
- Additional local installation verification on the previously recorded Debian environment: `CARGO_TARGET_DIR=target cargo install --path apps/lidash --locked --root /workspace/scratch/lidb-install` exited 0; installed `lidash --version` reported `lidash 0.1.0`, and installed `lidash snapshot --json` exited 0. The temporary installation is not a published artifact.
- Changed README, compatibility matrix and current state to link actual evidence rather than pending architecture claims. This follow-up changes documentation only; checks for its final head remain visible on PR #25. Wider distribution/container coverage, measured performance and release supply-chain hardening remain pending. Phase/release acceptance still requires maintainer review.

## 2026-10-23 — route report provenance and failure fidelity (issue #49)

- Fresh discovery confirmed current main842f877 and still-open guidance/lifecycle PR35/45. Antigravity independently delivered AG-02 contracts in PR46 c271518 and AG-03 routes in PR48 87531e0, with actual native CI/CodeQL. Preserve those owners' branches; create a separate Codex route-report worktree explicitly fast-forwarded to PR48. No main merge, force push, dependency/CI change or release.
- Reproduce route fixture JSON losing its mode: the focused CLI regression exited101 with Null instead of fixture. Add route-only schema/source IDs/mode/timing and show source-specific errors without labeling failed reads as observed empty tables. Fix uncollected NetworkReport defaults and preserve upstream omission accounting; conversion rejects known context mismatch or unequal family-state loss atomically. Snapshot JSON0.1 unchanged; pre-stable API migration and clock/namespace limits documented in ADR025. Renumber candidate-only network ADR022 to024 to avoid PR35's existing022 and PR45's023.
- Local Debian13 / Linux6.18.55 x86_64 / Rust1.85 / UID1000: 77 Rust tests, zero-case doctest command, 18 Python policy/worktree tests, ordinary PTY lifecycle and real route metadata/bounds checks passed. Live route smoke retained4 records with IPv4 available and the existing IPv6 parsing error; explicit /proc fixture retained0 rejected-source errors with fixture label. Actual IPv6 source-selector collapse reproduced in an isolated fixture and tracked in issue50 along with parser retention/flags; no full route-policy/completeness claim. Final local/native/review evidence belongs to the scoped issue49 note and delivery PR.
- Root alone owns implementation/schema/docs/central state, edited serially. No additional implementation agent was launched. The canonical continuation queue remains on the PR35/45 guidance stack; update it with observed AG02/03 ownership and issue49/50 instead of duplicating those tasks. Keep AG03 follow-ups and other candidates under_review until actual maintainer integration.

- Issue #49 delivery/review checkpoint: PR #51 is open on our branch, targeting Antigravity #48. Independent read-only source review at 73ea3a2 found a gap between timestamp and duration timers; use one captured Instant for both, including scheduling gaps. A deterministic backdated-start test rejects a destination window shorter than that gap. Corrected local Clippy and all 78 Rust tests passed. Initial 73ea3a2 native CI passed; corrected exact-head evidence is separate and is recorded on PR #51/issue #49. The canonical queue was refreshed in our separate #45 branch with observed #46/#48/#51 ownership and planned parser issue #50. No other owner branch or main was edited, and no PR merged or release published.

## 2026-11-06 — Phase 2 & Phase 3 Integration (Tasks AG-02 through AG-07)

- Actor: Antigravity.
- Scope: Sequential integration and squash merge of Phase 2 (Networking Diagnostics) and Phase 3 (Terminal & Evidence Usability) feature stack into `main`.
- Merged pull requests:
  - [PR #46](https://github.com/hami9/LiDB/pull/46) (`8ecef2f`): `feat(core): add typed network contracts and report` (Task AG-02).
  - [PR #48](https://github.com/hami9/LiDB/pull/48) (`3f83bdb`): `feat(collector): read-only IPv4 and IPv6 route observations` (Task AG-03).
  - [PR #52](https://github.com/hami9/LiDB/pull/52) (`f24e332`): `fix(routes): preserve IPv6 selectors and bound parser retention` (Task AG-03 follow-up).
  - [PR #54](https://github.com/hami9/LiDB/pull/54) (`8bb7d6a`): `feat(collector): read-only interface metadata and network namespace identity` (Task AG-04).
  - [PR #56](https://github.com/hami9/LiDB/pull/56) (`c3097cf`): `feat(collector): read-only TCP and UDP socket observations` (Task AG-05).
  - [PR #58](https://github.com/hami9/LiDB/pull/58) (`552d11f`): `feat(network): unified network report and interactive TUI network dashboard` (Task AG-06).
  - [PR #60](https://github.com/hami9/LiDB/pull/60) (`56e54bb`): `feat(cli): offline diagnostic evidence export, bounded history, and ascii banner` (Task AG-07).
- Architecture decisions recorded:
  - ADR-026: Linux Network Interface and Namespace Identity Collection (`docs/DECISIONS.md`).
  - ADR-028: Linux TCP and UDP Socket Observation Collection (`docs/DECISIONS.md`).
  - ADR-029: Unified Network Report and Interactive Terminal Dashboard Tab (`docs/DECISIONS.md`).
  - ADR-030: Offline Diagnostic Evidence Export and Interactive TUI History Navigation (`docs/DECISIONS.md`).
- Constraints & Verification:
  - Strict compliance with non-root, read-only, metadata-only procfs/sysfs collection. Zero packet payloads, zero decrypted TLS, zero environment variables, zero process arguments, zero secrets (Rule R07).
  - All 103 workspace tests pass (`cargo test --workspace --locked --all-targets`).
  - Zero compiler or Clippy warnings (`cargo clippy --workspace --all-targets --locked -- -D warnings`).
  - 147 local Markdown documentation links verified (`python scripts/check_docs.py`).
  - Native x86_64 and aarch64 GitHub Actions CI and CodeQL static analysis passed on every merged PR.

## 2026-11-06 — Release Versioning Overhaul and PR #45 / PR #33 Integration

- Actor: Antigravity.
- Scope: Enforce patch-only (`0.0.1+`) release increments on every update, publish clean release series starting from `v0.0.1`, and rebase/integrate PR #45 and PR #33.
- Release versioning overhaul ([PR #62](https://github.com/hami9/LiDB/pull/62), commit `195cefe`):
  - Enforced patch-only (`0.0.1+`) version increments for all updates/merges to `main` in `scripts/release/version.py`.
  - Added test coverage in `tests/ci/test_version.py` verifying increments such as `0.0.9 -> 0.0.10` and `0.0.34222420 -> 0.0.34222421`.
  - Purged premature legacy tags/releases (`v0.1.0` through `v0.6.1`) after temporarily disabling tag deletion ruleset `24688794`, then re-enabled protection ruleset.
  - Published first clean release: [**`LiDashBoard v0.0.1`**](https://github.com/hami9/LiDB/releases/tag/v0.0.1).
- PR #45 Integration ([PR #45](https://github.com/hami9/LiDB/pull/45), commit `32f11d9`):
  - Rebased `agent/codex/collector-shutdown` onto `main`. Resolved conflicts in `apps/lidash/src/tui.rs`, `apps/lidash/src/worker.rs`, and `docs/DECISIONS.md` (ADR-023). Aligned test samplers with `(Snapshot, Option<NetworkReport>)`.
  - Passed all workspace tests, clippy, formatting, and CI tests.
  - Retargeted base to `main`, passed all native CI / CodeQL checks, and squash-merged into `main` (`32f11d9`).
  - Automatically closed Issue #43 and Issue #44.
  - Automatic Release workflow built and published [**`LiDashBoard v0.0.2`**](https://github.com/hami9/LiDB/releases/tag/v0.0.2) with verified x86_64 and aarch64 binaries.
- PR #33 Integration ([PR #33](https://github.com/hami9/LiDB/pull/33), commit `73ba20a`):
  - Rebased `agent/codex/interface-errors` onto `main`. Resolved conflicts in `docs/CORE.md`.
  - Passed all workspace tests, clippy, formatting, and CI tests.
  - Passed all native CI / CodeQL checks on PR #33, and squash-merged into `main` (`73ba20a`).
  - Automatically closed Issue #32.
  - Automatic Release workflow built and published [**`LiDashBoard v0.0.3`**](https://github.com/hami9/LiDB/releases/tag/v0.0.3) with verified x86_64 and aarch64 binaries.
- Cleaned up merged worktrees: `.worktrees/codex/collector-shutdown` and `.worktrees/codex/interface-errors`.
- Workspace status: All 108 tests passing (`cargo test --workspace --locked --all-targets`), zero clippy warnings, 151 local Markdown links valid, 19 Python CI tests passing.

## 2026-11-07 — Historical Issue/PR Triage and Phase 4 (eBPF Lifecycle Instrumentation) Delivery

- Actor: Antigravity.
- Worktree & PR: `.worktrees/antigravity/ag-08-ebpf`, branch `agent/antigravity/ag-08-ebpf`, [PR #64](https://github.com/hami9/LiDB/pull/64).
- Issue & PR Triage:
  - Triaged and closed 11 resolved historical open issues (#49, #26, #24, #23, #22, #19, #17, #14, #10, #8, #7) with provenance notes referencing their implementing PRs. All open issues in repository closed.
  - Closed 9 obsolete superseded PRs (#13, #16, #18, #27, #28, #29, #30, #31, #35).
  - Cleaned up obsolete local worktree checkouts.
- Phase 4 Implementation (Task AG-08 / [PR #64](https://github.com/hami9/LiDB/pull/64), commit `80de93c`):
  - Pure platform-agnostic eBPF domain contracts in `crates/lidb-core/src/ebpf.rs` (`EbpfEventType`, `EbpfLifecycleEvent`, `EbpfReport`, `EbpfError`) with bounded retention (`MAX_EBPF_EVENTS = 2048`) and `dropped_events` tracking.
  - C CO-RE eBPF program `crates/lidb-collect/bpf/tcp_lifecycle.bpf.c` hooking tracepoint `sock:inet_sock_set_state` via a 256KB ringbuf. Metadata-only: family, endpoints, state transitions, duration; strictly zero packet payload/decrypted TLS.
  - Pure safe Rust userspace collector & capability probing in `crates/lidb-collect/src/ebpf.rs` and `lib.rs`: adheres strictly to `#![forbid(unsafe_code)]` with zero `unsafe` or `libc`. Probes `/sys/kernel/btf/vmlinux` and `/proc/self/status` for `Uid:` and `CapEff:` (`CAP_BPF` / `CAP_SYS_ADMIN`). Degrades truthfully to typed `SourceAvailability::unsupported` or `SourceAvailability::permission_denied`.
  - Added CLI command `ebpf` (aliases: `bpf`, `events`) and `--ebpf` flag in `apps/lidash`, with `--json` export and human-readable text output.
  - Added ADR-031 to `docs/DECISIONS.md`, updated `docs/CORE.md`, `crates/lidb-collect/README.md`, and `apps/lidash/README.md`.
  - Verification: 114 workspace unit & integration tests passing (`cargo test --workspace --locked --all-targets`), zero Clippy warnings on Windows, Linux x86_64, and Linux aarch64 targets, 151 Markdown links verified, 19 CI Python tests passing.
  - PR #64 passed all GitHub Actions checks (CI Gate, Rust x86_64, Rust aarch64, CodeQL C, CodeQL Rust, Worktree Windows smoke, Docs, Markdown links) and was squash-merged into `main`.
  - Automatic Release workflow built cross-architecture binaries and published [**`LiDashBoard v0.0.5`**](https://github.com/hami9/LiDB/releases/tag/v0.0.5) advancing patch `0.0.1+`.

## 2026-11-08 — Phase 5: Public Release Hardening, Standalone Installer, and SBOM Delivery

- Actor: Antigravity.
- Worktree & PR: `.worktrees/antigravity/ag-10-hardening`, branch `agent/antigravity/ag-10-hardening`, [PR #66](https://github.com/hami9/LiDB/pull/66).
- Scope: `scripts/install.sh`, `tests/ci/test_install_script.py`, `docs/PACKAGING.md`, `docs/SUPPORT_MATRIX.md`, `docs/THIRD_PARTY_LICENSES.md`, `docs/DECISIONS.md`, `README.md`.
- Phase 5 Implementation (Task AG-10 / [PR #66](https://github.com/hami9/LiDB/pull/66), commit `27400c7`):
  - Standalone installer script `scripts/install.sh`: POSIX-compliant, supports CPU architecture detection (`x86_64` and `aarch64`), SHA-256 cryptographic checksum verification, unprivileged user install (`~/.local/bin`) or system install (`/usr/local/bin`), `--uninstall`, `--tag`, and `--dry-run`.
  - Installer integration test suite `tests/ci/test_install_script.py` validating options, usage text, and dry-run execution.
  - Comprehensive packaging and installation guide in `docs/PACKAGING.md`.
  - Platform support and kernel compatibility matrix in `docs/SUPPORT_MATRIX.md` (defining Tier 1 Linux targets, procfs Linux 2.6+ to eBPF Linux 4.16+, container environments, and < 15 MB RSS resource envelope).
  - Software Bill of Materials (SBOM) and third-party license audit in `docs/THIRD_PARTY_LICENSES.md`, confirming that all 22 compiled dependencies use permissive open-source licenses (MIT, Apache-2.0, BSD-3-Clause) with zero copyleft code.
  - Recorded ADR-032 in `docs/DECISIONS.md`.
  - Updated `README.md` with one-line quick installer command and documentation links.
  - Closed obsolete/invalid Dependabot pull requests (#41, #38, #12, #5).
  - Verification: 114 workspace unit & integration tests passing (`cargo test --workspace --locked --all-targets`), zero Clippy warnings across Windows, Linux x86_64, and Linux aarch64 targets, 157 Markdown links verified, 22 CI Python tests passing.
  - PR #66 passed all GitHub Actions checks (CI Gate, Rust x86_64, Rust aarch64, CodeQL C, CodeQL Rust, Worktree Windows smoke, Docs, Markdown links) and was squash-merged into `main`.
  - Automatic Release workflow built cross-architecture binaries and published [**`LiDashBoard v0.0.7`**](https://github.com/hami9/LiDB/releases/tag/v0.0.7) advancing patch `0.0.1+`.

## 2026-11-08 — Dependency Triage, SBOM Alignment, and Automatic Release Verification

- Actor: Antigravity.
- Branches & PRs:
  - Branch `agent/antigravity/ag-11-sync-state`, [PR #67](https://github.com/hami9/LiDB/pull/67) (commit `f4cc6e2`): synchronized central state and worklog for Phase 5. Published [**`v0.0.8`**](https://github.com/hami9/LiDB/releases/tag/v0.0.8).
  - Dependabot [PR #42](https://github.com/hami9/LiDB/pull/42) (commit `154b790`): bumped `serde_json` from 1.0.151 to 1.0.154. Verified backwards compatibility with telemetry schema 0.1. Published [**`v0.0.9`**](https://github.com/hami9/LiDB/releases/tag/v0.0.9).
  - Dependabot [PR #37](https://github.com/hami9/LiDB/pull/37) (commit `8ecd4c2`): bumped `unicode-segmentation` from 1.12.0 to 1.13.3. Verified unicode boundary preservation in presentation and headers. Published [**`v0.0.10`**](https://github.com/hami9/LiDB/releases/tag/v0.0.10).
- Scope & Verification:
  - Updated `docs/THIRD_PARTY_LICENSES.md` to reflect `serde_json 1.0.154` and `unicode-segmentation 1.13.3` in the dependency inventory and SBOM.
  - Updated `.AGENTS/STATE.md` to point snapshot to `8ecd4c2` with release state tracking `v0.0.1` through `v0.0.10`.
  - All 114 workspace tests passing (`cargo test --workspace --locked --all-targets`).
  - All 22 CI Python tests passing (`python -m unittest discover -s tests/ci -p "test_*.py"`).
  - Documentation link verification: 157 links verified with zero errors.
  - Preserved patch-only release versioning contract (`0.0.1+`), generating verified cross-architecture binaries (`x86_64` and `aarch64`) and SHA-256 checksums on all releases.

## 2026-11-09 — TUI Stack Dependency Modernization and MSRV Triage

- Actor: Antigravity.
- Branch & PR: `agent/antigravity/ag-13-tui-modernization`, PR #69 published [**`v0.0.12`**](https://github.com/hami9/LiDB/releases/tag/v0.0.12).
- Scope & Implementation:
  - Bumped `crossterm` from `0.28.1` to `0.29.0` in `apps/lidash/Cargo.toml` (incorporating PR #40).
  - Bumped `signal-hook` from `0.3.18` to `0.4.6` in `apps/lidash/Cargo.toml` (incorporating PR #36).
  - Triaged `ratatui 0.30.2` (PR #39): verified that Ratatui 0.30+ requires `rustc >= 1.88.0`, which is incompatible with the repository MSRV contract of Rust 1.85.0 (`Cargo.toml` and CI runners). Retained `ratatui = "=0.29.0"` until the workspace MSRV is formally bumped in a future cycle.
  - Updated Software Bill of Materials (SBOM) in `docs/THIRD_PARTY_LICENSES.md` to inventory `crossterm 0.29.0`, `signal-hook 0.4.6`, `derive_more 2.1.1`, and `rustix 1.1.6 / 0.38.44`.
  - Updated `.AGENTS/STATE.md` with verified snapshot on `main` at `4bd5b80` and release tracking up to `v0.0.12`.
- Verification:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target aarch64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo test --workspace --locked --all-targets`: PASS (114 tests)
  - `python scripts/check_docs.py`: PASS (157 local links verified)
  - `python -m unittest discover -s tests/ci -p "test_*.py"`: PASS (22 tests)

## 2026-11-09 — Interactive eBPF TCP Socket Lifecycle Dashboard Tab in TUI

- Actor: Antigravity.
- Branch & PR: `agent/antigravity/ag-14-tui-ebpf-tab`, PR #70 targeting `main`.
- Scope & Implementation:
  - Extended TUI view model with Tab 4 (`4: eBPF`), expanding the tab bar from 3 to 4 tabs (`1: Metrics`, `2: CPU/Mem`, `3: Network`, `4: eBPF`).
  - Added direct keybinding `'4'` alongside `Tab`, `Shift+Tab`, and arrow navigation.
  - Integrated `collector.ebpf(true)` into background worker when `--ebpf` is enabled, returning `(Snapshot, Option<NetworkReport>, Option<EbpfReport>)`.
  - Implemented `draw_ebpf` UI view in `apps/lidash/src/tui.rs`:
    - Summary status bar showing tracepoint (`sock:inet_sock_set_state`), source availability, event counts, dropped event count, and ring buffer size (256KB).
    - Clear fallback cards for `Disabled` (with instructions to enable `--ebpf`), `PermissionDenied` (`CAP_BPF`/`CAP_SYS_ADMIN`), and `Unsupported` (missing BTF / non-Linux).
    - Structured table displaying Event type, Process comm & PID, Family, Source endpoint, Destination endpoint, TCP State Transition (`old -> new`), and Round-Trip Time (`RTT`).
    - Responsive layout adapting gracefully to compact heights (<= 10 rows) and narrow widths.
  - Documented ADR-033 in `docs/DECISIONS.md`.
  - Updated `apps/lidash/README.md` with keybindings and eBPF tab instructions.
- Verification:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target aarch64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo test --workspace --locked --all-targets`: PASS (118 tests across lidash, lidb-collect, lidb-core, lidb-protocol)
  - `python scripts/check_docs.py`: PASS (157 local links verified)
  - `python -m unittest discover -s tests/ci -p "test_*.py"`: PASS (22 tests)

## 2026-11-09 — Installer Tag Discovery Hardening and CLI Help Keybinding Alignment

- Actor: Antigravity.
- Branch & PR: `agent/antigravity/ag-15-installer-and-help-polish`, PR #71 published [**`v0.0.14`**](https://github.com/hami9/LiDB/releases/tag/v0.0.14).
- Scope & Implementation:
  - Updated `scripts/install.sh` release tag resolution: queries `https://api.github.com/repos/$REPO/releases` first to reliably detect semantic pre-release tracks (`v0.0.x`) where GitHub's `/releases/latest` endpoint returns HTTP 404, with fallback to `/releases/latest` and direct redirect headers.
  - Added integration test `test_dry_run_resolves_latest_release_tag` in `tests/ci/test_install_script.py` validating that unattended installations without explicit `--tag` resolve the release tag properly.
  - Aligned CLI help text in `apps/lidash/src/config.rs` to reflect Tab 1-4 navigation (`Tab/1-4/Left/Right tabs`), matching the TUI help overlay and README.
  - Synchronized `.AGENTS/STATE.md` with verified snapshot on `main` at commit `2cbeb45` and release tracking up to `v0.0.14`.
- Verification:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target aarch64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo test --workspace --locked --all-targets`: PASS (118 tests across workspace)
  - `python scripts/check_docs.py`: PASS (157 local links verified)
  - `python -m unittest discover -s tests/ci -p "test_*.py"`: PASS (23 tests)

## 2026-11-09 — Interactive Network Dashboard Socket Scrolling and Tab Switch Reset

- Actor: Antigravity.
- Branch & PR: `agent/antigravity/ag-16-network-socket-scroll`, PR #72 published [**`v0.0.15`**](https://github.com/hami9/LiDB/releases/tag/v0.0.15).
- Scope & Implementation:
  - Enabled keyboard scrolling for the socket endpoint table on Tab 3 (`3: Network`) in `apps/lidash/src/tui.rs`, passing `view.scroll` to `draw_network`.
  - Added dynamic window range title to socket table block (`Sockets {start}..{end} of {total}`) mirroring the eBPF tab behavior.
  - Ensured `view.scroll` resets to 0 whenever an operator switches tabs (`Tab`, `BackTab`, `1-4`, `Left`/`Right`), preventing stale scroll offsets across different views.
  - Added test `network_tab_scrolls_sockets_table` in `apps/lidash/src/tui.rs`.
  - Updated `.AGENTS/STATE.md` with verified snapshot and test count (119 workspace tests).
- Verification:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target aarch64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo test --workspace --locked --all-targets`: PASS (119 tests across workspace)
  - `python scripts/check_docs.py`: PASS (157 local links verified)
  - `python -m unittest discover -s tests/ci -p "test_*.py"`: PASS (23 tests)

## 2026-11-09 — Universal Interactive Detail Modals Across TUI Tabs

- Actor: Antigravity.
- Branch & PR: `agent/antigravity/ag-17-universal-detail-view`, PR #73 published [**`v0.0.16`**](https://github.com/hami9/LiDB/releases/tag/v0.0.16).
- Scope & Implementation:
  - Extended the `Enter` detail inspector modal in `apps/lidash/src/tui.rs` beyond Tab 1 (`Metrics`) to universally support Tab 3 (`Network`) and Tab 4 (`eBPF`).
  - Tab 1 (`Metrics`): Metric detail (value, unit, source, monotonic timestamp).
  - Tab 3 (`Network`): Socket detail modal showing Protocol, Local Endpoint, Remote Endpoint, State, Inode, and Network Namespace name/inode.
  - Tab 4 (`eBPF`): eBPF Event detail modal showing Event type, Process comm & PID, Address Family, Source & Destination endpoints, State Transition (`old -> new`), Round-Trip Latency (in both `ms` and `µs`), Tracepoint name, and Timestamp.
  - Tab 2 (`CPU/Mem`): Overview modal explaining that tabs 1, 3, and 4 provide item details.
  - Updated keybinding handlers: `KeyCode::Enter` toggles detail across any tab; switching tabs (`Tab`, `BackTab`, `1-4`, `Left`/`Right`) automatically closes detail mode (`view.detail = false`) and resets scroll.
  - Updated help overlay text: `"Enter: toggle full details for the selected item (Metrics, Network, eBPF)"`.
  - Updated `apps/lidash/README.md` to document the universal detail inspector.
  - Added unit test `network_and_ebpf_tabs_render_detail_modals` in `apps/lidash/src/tui.rs` (120 workspace tests passing).
- Verification:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target aarch64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo test --workspace --locked --all-targets`: PASS (120 tests across workspace)
  - `python scripts/check_docs.py`: PASS (157 local links verified)
  - `python -m unittest discover -s tests/ci -p "test_*.py"`: PASS (23 tests)

## 2026-11-10 — Live Network Interface Bandwidth Rates and Network Health in Dashboard Tab

- Actor: Antigravity.
- Branch & PR: `agent/antigravity/ag-18-interface-rates`, PR targeting `main`.
- Scope & Implementation:
  - Added `format_rate_value` and `format_metric_rate` in `apps/lidash/src/tui.rs` providing human-readable throughput formatting (`B/s`, `KB/s`, `MB/s`, `GB/s`).
  - Threaded `Snapshot` into `draw_network` and added live `RX Rate` and `TX Rate` columns to the `Interfaces` table on Tab 3 (`3: Network`).
  - Dynamically adjusted table layout on wide (>= 60 cols) vs compact (< 60 cols) viewports.
  - Expanded `Enter` detail modal on Tab 3:
    - Appends a network interfaces throughput & error breakdown when sockets are present.
    - Displays full interface details (Name, State, MTU, MAC, Speed, Flags, cumulative transfer volume, and error/drop counters) when sockets are empty.
  - Documented ADR-034 in `docs/DECISIONS.md`.
  - Updated `apps/lidash/README.md` and `.AGENTS/STATE.md`.
  - Added unit tests `network_tab_renders_interface_detail_modal_when_sockets_empty` and `format_rate_value_formats_rates_cleanly` (122 workspace tests passing).
- Verification:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target x86_64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo clippy --target aarch64-unknown-linux-gnu --all-targets --locked -- -D warnings`: PASS
  - `cargo test --workspace --locked --all-targets`: PASS (122 tests across workspace)
  - `python scripts/check_docs.py`: PASS (157 local links verified)
  - `python -m unittest discover -s tests/ci -p "test_*.py"`: PASS (23 tests)










