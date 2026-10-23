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
