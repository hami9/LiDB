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
