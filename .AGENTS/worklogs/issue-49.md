# Issue #49 — route report provenance and failure fidelity

- Owner: Codex coordinator, working alone in `.worktrees/codex/route-report`, branch `agent/codex/route-report`.
- Base: new clean worktree from main `842f877`, explicitly fast-forwarded to Antigravity PR #48 candidate `87531e0c28aced618c51011349cab72f7ccbe560` (depends on PR #46 `c271518`). No other owner's branch/worktree changed. Primary main and Codex PR #35/#45 are preserved.
- Status: `under_review`, [PR #51](https://github.com/hami9/LiDB/pull/51), explicitly targeting Antigravity #48. Inspect exact current-head checks and review before integration.
- Owned scope: collector RouteReport/clock plumbing, CLI report rendering/tests, minimal NetworkReport omission accounting/error contracts, docs/ADR and coordinator evidence. Source reader, parser semantics, dependency/CI configuration, interfaces and sockets are outside this slice.

## Acceptance

Reproduce missing live/fixture JSON provenance; add schema/source IDs/clock/batch time/duration. Preserve explicit `--proc-root /proc` fixture labeling. Show source-specific error reasons and distinguish unavailable sources from observed empty tables. Preserve upstream and destination-capacity omissions in NetworkReport, rejecting known context mismatches and lossy source-state collapse before mutation. Run meaningful unit/CLI tests, real non-root Linux route smoke and observe actual native x86_64/aarch64 checks.

Snapshot schema 0.1 is unchanged. No new dependency, unsafe code, daemon/listener, active probe, privilege, process inspection or host configuration. Route/endpoint metadata may identify local infrastructure; output remains explicitly requested. Synchronous reads remain without deadlines and procfs visibility is the current process namespace, not proof of host-wide coverage. Clock-domain identity/import remains future work; matching timestamps alone cannot prove common domains.

## Discovery / checkpoint — 2026-10-23

PR #35 `5360ace` and #45 `cf1fa0b` remain open with their recorded passing CI and no new requested review changes. Fresh fetch found Antigravity PR #46 (AG-02 contracts) and #48 (AG-03 routes) with native CI/CodeQL passed; do not duplicate their work. Their branches start from main and do not yet contain the guidance/lifecycle candidates. Canonical continuation instructions are on the still-open PR #35/#45 stack; this note records the route branch's exact base and scope.

Source review found route CLI JSON missing mode/schema/timestamps/source IDs; text hides failure reasons and reports no entries observed when reads failed. `populate_into` drops source omission counts and can relabel source context. Regression and final command outcomes will be appended after execution. Parser source-selector/flags semantics and temporary allocation bounds require a separate focused follow-up, not an all-routing-correctness claim.

## Implementation and local evidence

- Before changing runtime code, `cargo test -p lidash --locked --test cli routes_command_reads_fixture_routes -- --exact --nocapture` exited **101**: expected fixture mode, observed JSON `Null`.
- Route-only schema 0.1 now includes mode/source IDs/collector-local start/duration; CLI text uses the report's mode and preserves each source reason. An unavailable family is distinct from an observed empty table. Snapshot JSON is unchanged.
- `NetworkReport` now initializes uncollected tables as disabled/not-collected. Explicit setters are required after observation. `populate_into` returns a typed result, rejects known mode/time/window/namespace and unequal family-state loss before mutation, and preserves upstream plus destination omissions with saturating u64 counters. Matching times cannot verify common clock domains; callers still own that precondition.
- ADR-024 renumbers Antigravity's candidate-only network ADR-022 to avoid colliding with PR #35's Core ADR; ADR-023 belongs to lifecycle PR #45. ADR-025 records this report boundary/migration. The earlier candidate task note's numbering is historical.
- Local environment: Debian 13, Linux 6.18.55 x86_64, Rust 1.85.0, non-root UID 1000. `cargo test --workspace --locked --all-targets` passed **77 tests** (27 Core, 2 protocol, 29 collector, 19 app). Focused CLI checks also passed after adding a denied-source fixture and explicit `/proc` fixture labeling. The before/fix test is fixture evidence, not hardware validation.
- `cargo test --workspace --locked --doc` passed with zero cases. Python policy/worktree/release suite passed 18 tests; installer tests belong to the separate guidance stack and are absent from this branch. Ordinary PTY q/raw Ctrl-C/signals/resize/exact restoration passed after `cargo build -p lidash --locked`.
- Real route smoke passed metadata/bounds checks: live retained 4 records with IPv4 available and IPv6 error; explicit `/proc` fixture retained 0 with rejected-source errors and remained labeled fixture. The existing fixture symlink policy is preserved. No private addresses/paths were logged.
- A distinct IPv6 source-selector fixture reproduced the existing parser defect: two source prefixes collapsed to a duplicate error with zero records. **Open issue #50** tracks selectors/flags and temporary parser bounds, before routing completeness is claimed. This fix retains that error rather than inventing an empty successful table. No source-specific host routing configuration was changed/tested.
- Final formatter/Clippy/link/whitespace and remote exact-head checks remain to be recorded. Clippy caught a non-octal permission literal in the new fixture test; changed `0` to `0o0`, with no lint suppression.

Final local checkpoint: formatter, warnings-as-errors locked workspace Clippy, all 77 Rust tests, 142 local Markdown links and `git diff --check` passed (exit 0). Permission-denied fixture executed under UID 1000; the injected collector start offset makes the shared-clock regression deterministic without sleeps. Remote exact-head CI/review is pending at this checkpoint; its actual results will be recorded on the delivery PR and issue #49.

## Independent review and final delivery checkpoint

At pinned runtime `73ea3a2`, separate read-only agent source review found a collection-window gap: timestamp was captured before calling the route reader, whose duration timer started later. Descheduling between them could understate the window and admit an insufficient conversion target. Capture one `Instant`, derive its collector-local offset and pass that same start through for duration. A deterministic backdated-start regression checks that an injected gap is included and a shorter destination is rejected; no sleep/performance claim. Review found no other blocker in checked state/text/omission logic and is rechecking this correction. This is separate agent review, not human approval.

After correction, locked warnings-as-errors Clippy and all **78 Rust tests** passed (27 Core, 2 protocol, 30 collector, 19 app). The earlier 77-case results are historical. Initial runtime `73ea3a2` passed native x86_64/aarch64 [CI](https://github.com/hami9/LiDB/actions/runs/39944505505); that is not substituted for the corrected final head. Final exact-head results/run URLs are recorded in [PR #51](https://github.com/hami9/LiDB/pull/51) and issue #49, and must be checked anew at session startup.

The canonical queue on our separate PR #45 branch now records Antigravity #46/#48 and Codex #51 as `under_review`, with reproduced parser repair #50 next. Its documentation-only refresh preserves lifecycle runtime and has its own head checks. Other owners' branches and main remain untouched; no merge/tag/release performed. Resume requested #51 fixes in this owned worktree; otherwise claim #50 with a new isolated branch, preserving report fixes and serial schema ownership. Do not duplicate AG-02/03 or overwrite the separate #35/#45 guidance/lifecycle stack during future integration.

## Review axes / handoff

Linux/source: no reader/parser/host changes; actual IPv6 limitation is explicit in issue #50. Telemetry: fixture labels, independent reasons, omissions, monotonic windows and uncollected-vs-empty semantics verified. Privacy: local explicitly requested route metadata, sanitized source IDs, no extra reads/dependencies/privileges or implicit persistence. Reliability: bounds retained, rejection before mutation and saturating counts tested; temporary parser retention/read deadlines remain open. Product: help/operator docs explain report semantics, exit 0 reporting convention and cross-run/container limits. This is coordinator self-review until a separate review is actually recorded; no merge/release approval inferred.
