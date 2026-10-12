# Antigravity continuation contract

The maintainer requested durable instructions for completing the public LiDB product without repeatedly writing implementation prompts. This file supplies the queue and acceptance criteria; [the always-on rule](../.agents/rules/continue-project.md) loads it in future Antigravity sessions. Follow the [shared GitHub style skill](../.agents/skills/github-style/SKILL.md) and root [AGENTS.md](../AGENTS.md). Ordinary bounded implementation of these read-only tasks is authorized. This does not authorize merging, publishing, privileged operations, active probes or replacing other agents' work.

## Resume and delivery loop

1. Read this file, [STATE.md](STATE.md), [WORKTREES.md](WORKTREES.md), [architecture](../docs/ARCHITECTURE.md), [architecture review](../docs/ARCHITECTURE_REVIEW.md) and your existing task note. Inspect `git status`, branch/base, local changes, current issues/PRs and actual source. Do not start from a remembered phase label.
2. Resume your owned unfinished task first. Otherwise select the earliest eligible queue item whose inputs exist and whose files are unclaimed. Do not ask the maintainer to choose among already-listed routine tasks. A task blocked on a review, hardware or authorization does not block an independent eligible task.
3. Look for an existing issue with that task ID before creating another. Give it one owner, acceptance criteria, dependencies and expected paths. Task IDs such as `AG-01` use lowercase branch slugs, for example `python3 scripts/worktrees.py create antigravity ag-01-lifecycle`, yielding `agent/antigravity/ag-01-lifecycle`. Open that worktree as the editor workspace. Do not edit a Codex/Claude worktree or reuse another owner's branch.
4. If a future required dependency is still unmerged and `main` lacks its implementation, fetch the actual candidate refs. On a newly created clean worktree only, fast-forward to the required candidate with `git merge --ff-only origin/<verified-candidate-branch>` and record that branch as the stacked PR base. Do not merge either PR yourself, invent its number/status or independently re-import obsolete scaffolds. If fast-forward fails, inspect the dependency and preserve the worktree; do not reset it.
5. Before editing, write a lowercase task note such as `.AGENTS/worklogs/antigravity-ag-01.md` with task ID, issue, owner, base SHA/branch, owned paths, acceptance criteria and status. Record a checkpoint after each meaningful slice. File existence, a GitHub issue assignment or a local worktree does not by itself provide a distributed lock; inspect current work and coordinate genuine collisions.
6. Implement one reviewable slice. Keep collection outside UI rendering, domain contracts independent of OS/UI libraries, exact counters and explicit failures. Reuse supported source readers and contracts. Introduce dependencies only with license/MSRV review. Document ordinary design choices in an ADR instead of asking for a new prompt; an unapproved privileged/listener/persistence change is a real authorization boundary.
7. Run relevant gates below; fix reproducible failures and relevant review findings. Report blocked/unrun platform tests precisely. Update operator docs and the scoped task note. Only the coordinator edits central STATE/WORKLOG/CI/shared schemas during concurrent work; a sole authorized integration owner may do so serially. Otherwise put the proposed central update in the handoff.
8. Commit with imperative subjects, push your branch, open a Conventional Commit PR linked to the issue, and inspect CI at the actual head SHA. Record `under_review`, not `done`, while integration remains pending. Continue an independent queue item when appropriate. Dependent work can use an explicitly recorded stacked base; phase/release acceptance remains separate.
9. On session end or context compaction, leave the exact checkpoint, changed/dirty files, commands/results, blockers, PR URL and next eligible task in the scoped note. The next session resumes from source and this note. Never redo completed work solely because chat history is missing.

Valid task statuses: `planned`, `in_progress`, `blocked`, `under_review`, `integrated`. Set `integrated` only after observing the merge and its checks. Queue entries below are specifications, not claims of implementation or an active assignment.

## Ordered task queue

| ID | Work | Dependency / eligibility | Initial state |
| --- | --- | --- | --- |
| AG-00 | Verify the public Core candidate and ownership | First session; actual PR/branch evidence | planned |
| AG-01 | Bound collection lifecycle and shutdown behavior | AG-00 verification; candidate base available | planned |
| AG-02 | Define typed network records and source policy | AG-00 verification; coordinator owns shared schema | planned |
| AG-03 | Read-only IPv4/IPv6 route observations | AG-02 contracts present; reader limits respected | planned |
| AG-04 | Interface metadata and namespace identity | AG-02 contracts present; source-specific symlink policy | planned |
| AG-05 | Bounded socket metadata | AG-02 contracts present; no process inspection | planned |
| AG-06 | Bounded history and offline evidence | AG-01 lifecycle decision; explicit clock-domain contract | planned |
| AG-07 | Public release hardening and measured compatibility | Candidate implementation and CI; no publication authority | planned |
| AG-08 | Optional eBPF or active diagnostics | Explicit use case and operator authorization required | blocked |

### AG-00 — verify before extending

- Inspect current main and the actual status of PR #25 (Core merged), #34 (CPU/Memory tab merged), #35 (this guidance), plus the pending candidates below. Confirm code is present in your base; do not treat old AI/vendor TUI/IPC prototypes as integration requirements.
- Validate format/lint/tests, live snapshot/doctor and terminal lifecycle where supported. Record local OS/kernel/architecture/toolchain and each native GitHub result separately.
- Confirm ownership of schemas/CI/central state before choosing AG-01 or AG-02. Capture any genuine regression in a focused issue and fix it before extending the affected boundary.
- **Acceptance:** a scoped verification note with exact base/check URLs, pending integration status and the first eligible task. Checking an existing candidate is not permission to merge it or accept a phase.

## Existing candidate ownership to inspect

As checked on 2026-10-12, these PRs remain separate owners' work, not tasks to duplicate or auto-merge. Fetch their actual status at startup; this table is a discovery aid, not a permanent reservation or approval.

| PR | Existing scope | Continuation implication |
| --- | --- | --- |
| [#27](https://github.com/hami9/LiDB/pull/27) | Direct ARM64 no-follow regression; flags already corrected in main by maintainer | Inspect native before/fix evidence; preserve owner's reader tests |
| [#28](https://github.com/hami9/LiDB/pull/28) | Naming rules / contributor and agent entry points | Reconcile when integrated; shared supplied skill follows current repo conventions |
| [#29](https://github.com/hami9/LiDB/pull/29) | Device-limit partial collection | Do not reimplement or overwrite that owner's limit path |
| [#30](https://github.com/hami9/LiDB/pull/30) | Pure protocol frame codec | Evaluate actual need separately; no mandatory daemon/IPC inference |
| [#33](https://github.com/hami9/LiDB/pull/33) | Interface error/drop counters | Reuse checked records if integrated; avoid duplicating collector work |
| [#31](https://github.com/hami9/LiDB/pull/31) | Fixes to a separate earlier TUI prototype | Current baseline is the merged public app; do not import vendor/planned tabs by default |

### AG-01 — lifecycle under slow or failed reads

- Owned scope: `crates/lidb-collect/src/read.rs`, source/lifecycle contracts and `apps/lidash/src/worker.rs`, with targeted tests/docs. Coordinate if another task needs the reader.
- Current gap: reads are size-bounded but synchronous; cancellation wakes the refresh wait but cannot interrupt a blocked read, and `Worker::shutdown` joins the thread. Trusted fixture directories are still subject to filesystem stalls.
- Write an ADR evaluating source restrictions, reader isolation and timeout/shutdown behavior. Choose the smallest mechanism consistent with a non-root, local, no-daemon baseline; do not claim userspace can cancel an arbitrary kernel read. Declare worker lifetime, outstanding-read/process bounds, timeout states and cleanup explicitly.
- **Acceptance:** a deterministic blocked-reader test demonstrates bounded UI exit/error handling; no unbounded worker creation, mailbox growth or silently reused stale value; q/Ctrl-C/SIGINT/SIGTERM restore the terminal; slow/failed sources retain typed reasons. Record measured latency and remaining OS limits. Any change requiring a new privileged helper or listener stays blocked on explicit authorization.

### AG-02 — network contracts before network presentation

- Owned scope: additive typed records in `lidb-core`, collector source selection and `docs/CORE.md`/ADR; shared schema needs a single integration owner.
- Routes, endpoints and interface attributes are structured records, not numeric `MetricValue` strings. Keep numerical counters in Snapshot, and design a bounded versioned network report with namespace/provenance, collection time, availability and dropped-record accounting.
- Define per-report/file/record/text limits, live/fixture labeling, address-family representation and how source failure differs from an observed empty table. Use source-specific procfs/sysfs path rules; legitimate live sysfs symlinks cannot be handled by blindly loosening fixture validation.
- **Acceptance:** contract tests for IPv4/IPv6, exact identifiers/counters, malformed/control-bearing fields, empty/denied/partial data, over-limit input and version compatibility. Core has no filesystem, OS-specific or presentation dependency; schema changes have migration notes. No daemon/IPC is introduced merely to carry local records.

### AG-03 — routes

- Owned scope: a dedicated route module under `lidb-collect`, fixtures and one CLI/network report view.
- Read the current namespace's `/proc/net/route` and `/proc/net/ipv6_route`, or justify a read-only netlink adapter with equivalent limits. Do not run `ip`/shell commands inside collection, modify routes or claim host-wide visibility from a container.
- **Acceptance:** correct address byte order, prefix/mask/interface representation, independent IPv4/IPv6 failures, legitimate empty tables, duplicate/malformed/oversized fixtures and one real Linux metadata-only smoke. Show source/namespace and explicit unsupported cases in text/JSON/doctor. No connectivity probe or asserted causal diagnosis.

### AG-04 — interfaces and namespace identity

- Owned scope: interface metadata module, fixture-root handling and relevant presentation.
- Add bounded interface index, MTU, operational state and optional link speed where supported. Capture best-effort current network-namespace identity. Keep kernel names reversibly encoded; do not aggregate layered devices into invented host totals.
- **Acceptance:** fixtures for disappearing interfaces, unknown speed, denied sysfs, valid live symlinks versus rejected fixture escapes, punctuation/Unicode names and record limits; real non-root smoke. No namespace switching, driver change or privileged capability request.

### AG-05 — sockets

- Owned scope: socket module and fixtures/report output. Read only the current namespace's bounded TCP/UDP IPv4/IPv6 metadata; UNIX sockets are a separate optional slice if path privacy is documented.
- Do not scan process directories, command lines, environments, payloads or credentials. Do not map sockets to PIDs by adding intrusive process inspection. Document endpoint metadata as potentially identifying and keep exports explicitly operator-selected.
- **Acceptance:** address/port byte order, state mapping, empty table versus failure, denied/malformed/truncated/oversized sources and dropped-record counts; deterministic IPv4/IPv6 fixtures and real non-root smoke. Network report consumers work without new privileges or external services.

### AG-06 — history and offline evidence

- Owned scope: application retention/presentation, additive replay/export validation and fixtures. Reuse `TelemetryHistory` for compatible numerical series.
- Add explicit clock-domain identity before comparing imported or restarted collectors; do not compare unrelated `collector_monotonic` timestamps. Set a global series/sample/byte budget as well as each history's bound. Retain unavailable/reset gaps and eviction counts.
- Export only to an operator-selected path, with bounded validated records and privacy documentation; do not silently persist or accept unchecked serde input. History/replay and export can be separate PRs.
- **Acceptance:** deterministic retention/eviction and mixed-domain rejection, corrupted/unknown-schema/control-bearing/oversized replay cases, pause/resume/narrow-terminal lifecycle, explicit live/fixture/replay labels and exact integer round trips. Default startup still needs no database, cloud or service.

### AG-07 — stable release readiness evidence

- Owned scope: packaging/licenses/support docs and measured benchmarks; read the CI/CD skill before touching CI or release/dependency configuration.
- Prereleases v0.1.0/v0.2.0 already exist; do not recreate published tags or pretend they are absent. Complete third-party license notices and SBOM/provenance checks for the locked dependencies before stable-release commitments. Verify installation/removal and archive content/checksum on native x86_64/aarch64. Measure CPU/RSS, collection latency and skipped updates with named workloads and intervals; record wider distribution/container visibility accurately.
- **Acceptance:** independently reviewable evidence and accurate support/release checklist; no fabricated overhead targets, undisclosed network operations, missing license notices or bootstrap-only binary. Open publication/tag/administration steps remain explicitly pending until authorized; completing this task does not publish a release.

### AG-08 — optional deeper diagnostics

This item is not blanket authorization for eBPF, packet capture, active probes, host configuration, privileges or vendor/cluster integrations. Wait for a concrete operator need and explicit scope/permission, then create its own threat/compatibility/cleanup and resource-bound plan. Skip this blocked item while eligible unprivileged work remains.

## Verification commands

Run only checks relevant to the changed slice; record actual results, not copied PASS labels.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
cargo test --workspace --locked --doc
python3 -m unittest discover -s tests/ci -p 'test_*.py' -v
python3 scripts/check_docs.py
git diff --check
cargo run -p lidash --locked -- snapshot --json
cargo run -p lidash --locked -- doctor --json
python3 apps/lidash/tests/pty_smoke.py target/debug/lidash
```

On non-Linux machines, collect pure/tooling evidence and use native Linux CI for live application claims. A doctest command with zero cases is not additional coverage. Check CI at the actual pushed SHA; do not infer ARM success from configured runners. Leave human/independent review and phase acceptance distinct from self-review and successful tests.
