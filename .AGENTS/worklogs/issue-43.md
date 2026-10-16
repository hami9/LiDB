# Issue #43 — bounded interactive collector lifecycle (AG-01)

- Owner: Codex coordinator; branch/worktree `agent/codex/collector-shutdown`, `.worktrees/codex/collector-shutdown`.
- Base: main `842f877`; new clean worktree explicitly fast-forwarded to PR #35 candidate `5360ace9907819056fe4f67a1542e735df635358`. Runtime PR must target `agent/codex/agent-handoff` while that dependency is open. No merge, force push or release is authorized/performed.
- Ownership verified on 2026-10-16: PRs #27–31/#33 remain separately owned; #35 remains open. Reader tests belong to #27. Existing worktrees and primary main are preserved. Dependency PRs #36–42 are outside this task.
- Root owns worker/TUI implementation, ADR/docs and central state serially. Isolated issue #44 worker supplied Linux PTY fixtures on `agent/codex/blocked-pty`; cherry-picked as `014e67d` (original `cacec99`). Independent review/guidance workers read source only and made no changes.
- Status: `under_review`, [PR #45](https://github.com/hami9/LiDB/pull/45), targeting `agent/codex/agent-handoff`. Actual head/native/review outcomes are recorded in that PR's body/checks; inspect them at session startup. Resume review fixes on this branch instead of starting another AG-01 implementation.

## Reproduction and decision

Before changing the worker, `cargo test -p lidash --locked worker::tests::blocked_sampler_shutdown_returns_timeout_instead_of_waiting_for_read -- --exact --nocapture` exited **101**: shutdown waited for the 2.00s rescue, then returned `Ok(())` instead of the expected timeout. The rescue avoids a hung test runner and is not a real blocked-kernel experiment.

[ADR-023](../../docs/DECISIONS.md#adr-023--bounded-interactive-collector-lifecycle-2026-10-16) chooses one standard thread, 5s interactive batch watchdog and 100ms shutdown grace after terminal restoration. Cancellation and publication share a mailbox lock, held only for short state changes. No replacement worker is admitted after a shutdown timeout, even if the abandoned worker later returns; actual normal/panic join or spawn failure releases admission. Repeat shutdown/Drop reuses the timeout without another grace wait. No new dependency, unsafe code, helper, listener, privilege or runtime network request.

Snapshot/doctor remain synchronous without read deadlines. An admitted batch may finish remaining reads after cancellation but cannot publish or start another batch. This policy does not cancel arbitrary kernel reads or guarantee OS scheduling, terminal I/O, pathological thread teardown or hard real-time process death. JSON schema 0.1 and pure Core/protocol contracts are unchanged. Fixtures remain trusted/static.

## Review and fixes

Independent read-only lifecycle reviewer found a race between a successful UI deadline check and a late-completed worker: the timeout could become a generic stopped-worker error. The TUI now rechecks collection state after observing worker completion and restores the terminal before explicit shutdown. Reviewer found no further runtime blocker in the checked scope; this is agent source review, not human approval. The panic test now waits for actual completion before checking its error type, avoiding dependence on the 100ms grace under delayed scheduling.

The first integrated PTY run failed because Ratatui puts cursor-position escapes between words. The fixture now waits for the rendered `awaiting` token plus read-only header and callback marker rather than a contiguous sentence. All five cases then passed. No production test environment hooks exist; injection is crate-private, and marker/park logic is compiled only for tests.

PR #35 metadata was corrected: CodeRabbit's successful status wrapper explicitly **skipped** review for this OSS repository. No completed review or approval is inferred. Prior historical worklogs are retained rather than rewritten.

## Local evidence — 2026-10-16

Debian 13.6, Linux 6.18.44 x86_64, Rust 1.85.0, unprivileged UID 1000. Commands/results are actual local evidence, distinct from native GitHub runs.

- `cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS (exit 0).
- `cargo test --workspace --locked --all-targets` — PASS (exit 0): 56 tests (19 Core, 2 protocol, 14 collector, 21 app). One ignored helper is explicitly executed in five child PTY processes by the passing parent test.
- `cargo test --workspace --locked --doc` — PASS (exit 0), zero cases; not additional coverage.
- `python3 -m unittest discover -s tests/ci -p 'test_*.py' -v` — PASS (exit 0), 21 tests.
- `cargo fmt --all -- --check`, `python3 scripts/check_docs.py`, `git diff --check` — PASS (exit 0); 205 local Markdown links at the initial final check.
- `cargo build -p lidash --locked` and `python3 apps/lidash/tests/pty_smoke.py target/debug/lidash` — PASS (exit 0): ordinary q/Ctrl-C/SIGINT/SIGTERM/resize/restoration at 60000ms interval.
- `target/debug/lidash snapshot --json` and `doctor --json` — PASS (exit 0): 146 actual live observations, schema 0.1 and collector-monotonic clock; doctor reports degraded availability for unsupported PSI.
- `python3 apps/lidash/tests/blocked_pty.py target/debug/deps/lidash-e686f242ff2c2b37 5 0.1` — PASS (exit 0). Observed exits after readiness: q 0.15s, raw Ctrl-C 0.15s, SIGINT 0.20s, SIGTERM 0.20s; no-input watchdog 5.19s. Each exited 2 with exact terminal restoration before its diagnostic. These are one local simulation run's observed times, not performance/OS guarantees. The binary filename is this build's test runner, not a portable hard-coded command for other machines.
- Native Linux x86_64/aarch64 CI and CodeQL — pending when this initial delivery checkpoint was written. Final actual head/results are recorded on [PR #45](https://github.com/hami9/LiDB/pull/45) and its [check page](https://github.com/hami9/LiDB/pull/45/checks); do not substitute cross-compilation, configured runners or earlier PR #35 results. The initial runtime commit is `a9ab108d4e5a5c0ed79a3a3e4e72159d750aa245`; later checkpoint-only commits do not change that runtime diff.

## Five-axis assessment

- **Linux/source:** No reader/parser/source-policy changes; existing live/denied/malformed coverage remains. Kernel read cancellation remains explicitly unsupported.
- **Telemetry:** Deadline rejects an overdue batch and pending mailbox data; no fabricated per-source timeout readings or reused late snapshot. Exact integers, clock and schema unchanged.
- **Privacy/permissions:** Existing local metadata-only, non-root scope retained. Test marker files are disposable, and helper children are killed/reaped on error.
- **Reliability:** Blocked sample, repeated cleanup, concurrent/replacement admission, late publication, deadline boundary, captured-destructor stall and panic cleanup tested. One bounded mailbox and at most one abandoned worker. No CPU/RSS or kernel-stall performance claim.
- **Usability/compatibility:** Help/operator docs explain fixed deadline, grace, restart requirement, exit 2 and headless limit. PTY tests verify q/raw Ctrl-C/SIGINT/SIGTERM/watchdog and restoration before diagnostics; ordinary resize smoke remains separate.

## Delivery checkpoint

Initial runtime commit `a9ab108` is pushed and PR #45 is open/mergeable, explicitly based on PR #35. Final local formatting, 207 Markdown links and whitespace passed. PR #35's metadata correction is applied, without changing its head. No other owner's branch was modified and no merge/tag/release was performed. This checkpoint updates delivery metadata only; exact final CI run URLs/results remain in PR #45's body/checks and must be inspected for its current head before integration.

Resume by fetching #35/#45 and checking review/CI. If changes are requested, use this owned branch. AG-02 can proceed independently under a new issue/worktree; AG-01 stays `under_review` until observing maintainer integration. Do not merge either PR automatically. Existing native-agent startup guidance and shared skill continue unchanged.
