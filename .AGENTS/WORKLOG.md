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

## 2026-10-12 — shared GitHub skill, persistent Antigravity queue and Core review

- Maintainer supplied github-style and requested installation for future sessions, durable Antigravity continuation instructions and architecture review. Coordinator owns issue #26 and `agent/codex/agent-handoff`, based explicitly on PR #25's `ec1d307` candidate. No other agent worktree or primary main checkout was edited; no sub-agent was launched for this task.
- Added canonical native `.agents/skills/github-style/SKILL.md`, universal/native entry-point links, safe standard-library user-scope installer, provenance and machine/session limits. Skill text had been read before its temporary attachment vanished during a resumed session; installed copy SHA-256 is recorded in the scoped log. Actual local Codex/Claude/Antigravity paths were installed and repeat verified; no client or remote session launched.
- Added `trigger: always_on` Antigravity startup/resume rule and ordered AG-00–AG-08 task queue with sources, ownership, dependencies, bounds, meaningful tests and acceptance. The maintainer's continuation request covers routine unprivileged implementation choices; merge/release/admin/privileged actions remain explicitly authorized boundaries.
- Reviewed actual source and Cargo dependency graph. Corrected arm64 O_NOFOLLOW after verifying Linux v6.18 primary headers, added a direct open-time symlink regression independent of metadata preinspection, and removed the unused pre-stable bootstrap registry/test. Corrected source-export/async-runtime documentation drift and recorded ADR-022. The five-axis review and remaining limits are factual; this is self-review rather than independent approval.
- Local Debian 13.6 / Linux 6.18.44 x86_64 / Rust 1.85.0 / UID 1000: formatter, warnings-as-errors Clippy, 49 Rust tests (19 Core + 2 protocol + 15 collector + 13 app), doctest command (zero cases), 21 Python tests and PTY lifecycle passed. Real snapshot/doctor exported 146 observations with explicit unsupported PSI. Frontmatter, installation idempotency and actual acyclic workspace edges were verified. Detailed commands and outcomes: [issue-26 task note](worklogs/issue-26.md).
- Final local checks passed: 195 local Markdown links and `git diff --check`. Native GitHub ARM regression/CodeQL outcomes must be observed on this delivery's actual head before handoff. No new dependency, unsafe code, privileged runtime, daemon, listener or outbound runtime connection; no release/tag/merge performed. Public release readiness and hard synchronous-read deadlines remain unclaimed.

## 2026-10-12 — reconcile resumed work with current remote main

- Fresh remote discovery found PR #25 merged on 2026-10-10 with maintainer `ec50747` correcting flags/bootstrap, and PR #34 merged with CPU/Memory navigation. Main `842f877` and published prereleases v0.1.0/v0.2.0 supersede earlier pending-integration/no-release statements; those original worklog entries remain historical.
- Locally merge current main into the already published `agent/codex/agent-handoff` branch without force-push/history rewrite. Preserve current main runtime source, CLI fixes and CPU/Memory tab. Our duplicate open-reader refactor/regression is withdrawn because the separately owned PR #27 already supplies direct-open native regression evidence. Other pending naming/device-limit/protocol/interface-counter work is listed for Antigravity discovery, not imported or merged.
- PR #35 now targets main and delivers the skill/installer, durable continuation queue and factual current architecture documentation. Correct the stale README/changelog/state release claims after observing published assets. No GitHub PR merge, tag, release or other owner's branch mutation was performed by this task. Final validation belongs to the reconciled head.

- Reconciled local checks passed on the recorded Debian/Rust environment: formatting, warnings-as-errors Clippy, 51 Rust tests (19 Core + 2 protocol + 14 collector + 16 app), zero-case doctest command, PTY lifecycle, real 146-observation snapshot/doctor, 195 local Markdown links and whitespace. The 21 Python tests cover the unchanged installer/policy code. Final runtime is identical to current main; native PR #35 checks remain distinct from these local results.

## 2026-10-16 — AG-01 bounded interactive collector lifecycle

- Fresh discovery reconfirmed main `842f877`, merged Core/CPU tab, open guidance PR #35 at `5360ace`, published prereleases and separately owned candidates #27–31/#33. Create issue #43 and a new isolated `agent/codex/collector-shutdown` worktree, explicitly stacked on PR #35. Preserve the reader/other branches and avoid main edits, merges, force pushes or releases.
- Reproduce the unconditional-join bug: a blocked callback waited for the 2.00s test rescue and returned success; the timeout regression exited 101 before the fix. Implement a 100ms shutdown grace after terminal restoration, a 5s interactive batch watchdog, cancellation/publication serialization and process-wide admission that stays closed after a shutdown timeout. Repeated shutdown/Drop never repeats the grace. Arbitrary kernel reads remain uncancellable and headless snapshot/doctor still have no read deadline; record ADR-023 and operator help/docs.
- Parallel ownership was explicit: issue #44 supplied only test-only Linux PTY fixtures in a separate worktree (`cacec99`, cherry-picked `014e67d`); root owns worker/TUI, docs and central state serially. Independent read-only agent review found/fixed a racing timeout classification and confirmed teardown ordering; no further runtime blocker reported. This is agent source review, not human approval. The first integrated PTY run exposed ANSI-separated words in the readiness assertion; fix that test and rerun successfully.
- Actual local Debian 13.6 / Linux 6.18.44 x86_64 / Rust 1.85.0 / UID 1000: strict workspace Clippy, 56 Rust tests (19 Core, 2 protocol, 14 collector, 21 app), zero-case doctest command and 21 Python policy/installer/worktree tests passed. The passing Linux parent explicitly launches its ignored helper five times for blocked q/raw Ctrl-C/SIGINT/SIGTERM/watchdog PTY cases, requiring exit 2 and exact restoration before diagnostics. Ordinary PTY quit/signals/resize smoke and real live snapshot/doctor (146 observations; degraded PSI) passed. Final format/docs/whitespace and native head-specific checks belong to the delivery checkpoint in [issue #43 evidence](worklogs/issue-43.md).
- Corrected PR #35's misleading CodeRabbit sentence: the status wrapper succeeded but the bot explicitly skipped review for the OSS repository; no approval is inferred. No new dependency, unsafe code, daemon, privilege, listener or runtime network call. Stable/phase acceptance and performance claims remain unapproved. AG-01 is a candidate, not integrated; AG-02 is the next independent eligible queue item.
- Delivery: pushed runtime `a9ab108`, opened [PR #45](https://github.com/hami9/LiDB/pull/45) targeting `agent/codex/agent-handoff` / PR #35; update queue/state to `under_review`. Final format, 207 local Markdown links and whitespace passed. Head-specific native CI/CodeQL outcomes and run URLs are recorded on PR #45; check its actual current head rather than infer success from earlier baseline checks. This checkpoint changes documentation only.
