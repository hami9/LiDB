# Task AG-07 — Offline diagnostic evidence export and bounded history integration

- Task ID: `AG-07`
- Issue: [#59](https://github.com/hami9/LiDB/issues/59)
- PR: [#60](https://github.com/hami9/LiDB/pull/60)
- Owner: `agent/antigravity/ag-07-export-history`
- Worktree: `.worktrees/antigravity/ag-07-export-history`
- Base commit: `0d08b49` (`agent/antigravity/ag-06-network-ui`, PR #58)
- Head commit: `346e791`
- Stacked PR base: `agent/antigravity/ag-06-network-ui`
- Scope: `apps/lidash/src/config.rs`, `apps/lidash/src/output.rs`, `apps/lidash/src/lib.rs`, `apps/lidash/src/worker.rs`, `apps/lidash/src/tui.rs`, `apps/lidash/tests/cli.rs`, `apps/lidash/README.md`, `docs/DECISIONS.md`
- Status: `ready_for_review`

## Acceptance criteria

- Offline diagnostic evidence export in `apps/lidash`:
  - Add `export` command (alias: `bundle`) and `--output PATH` option.
  - Implement `EvidenceExport` (schema `0.1`, type `offline_diagnostic_evidence`) aggregating host snapshot, source doctor capabilities, and network report.
  - Embed explicit privacy declaration: non-root, read-only, metadata-only; zero packet payloads, zero decrypted TLS, zero environment variables, zero process arguments, zero secrets (Rule R07).
  - Formatted text output (`output::export_text`) separating snapshot, doctor, and network sections.
  - `--output PATH` writes directly to the destination file safely.
  - CLI integration tests for `lidash export`, `lidash export --json`, and `lidash export --output <path>`.
- Interactive TUI bounded history in `apps/lidash/src/tui.rs`:
  - Maintain an in-memory ring buffer capped at `MAX_VIEW_HISTORY = 120` samples (~2 minutes of 1s samples, ~700 KB RAM).
  - Navigation keys `[` / `,` (step backward) and `]` / `.` (step forward) to inspect previous updates.
  - Auto-pause when stepping backward into history.
  - Header bar indicates historical position (`PAUSED [-N/total]`) and true sample age.
  - Pressing `Space` resumes live streaming and immediately snaps back to latest update.
  - Unit tests verifying bounded retention (pop oldest at 120) and cursor stepping.
- Documentation & ADR:
  - Add ADR-030 in `docs/DECISIONS.md`.
  - Update `apps/lidash/README.md`.

## Implementation details

1. **CLI configuration (`apps/lidash/src/config.rs`)**:
   - Added `Command::Export` with aliases `export` and `bundle`.
   - Added `output_file: Option<PathBuf>` and `--output PATH` argument parsing.
   - Updated help text and validated `--json` compatibility.

2. **Evidence export bundle (`apps/lidash/src/output.rs`, `lib.rs`)**:
   - Defined `EvidenceExport` (schema `0.1`, type `offline_diagnostic_evidence`, and explicit `EXPORT_PRIVACY_DECLARATION`).
   - Derived `Clone, Debug, Serialize` on `Failure`, `SourceCapability`, and `DoctorReport`.
   - Implemented `export_text` formatting numbered sections for snapshot, doctor capabilities, and network report.
   - In `apps/lidash::execute`, executed `Command::Export`:
     - Samples baseline, sleeps interval, takes second snapshot for rates.
     - Creates `DoctorReport::from_snapshot(&snapshot)` and collects `LinuxCollector::network()`.
     - Buffers to file if `--output` is provided or writes to stdout. Returns exit code matching source capability usability.

3. **TUI bounded history (`apps/lidash/src/tui.rs`, `worker.rs`)**:
   - Derived `Clone` on `worker::Update`.
   - Added `MAX_VIEW_HISTORY: usize = 120`.
   - In `View`: added `history: VecDeque<Update>` and `history_cursor: Option<usize>`.
   - Implemented `push_update`, `step_history_back`, and `step_history_forward`.
   - Updated header bar to show `PAUSED [-offset/total]` and age since historical sample collection.
   - Added key bindings for `[` / `,` and `]` / `.` and snap-back on unpause (`Space`).

4. **Testing (`apps/lidash/src/output.rs`, `apps/lidash/src/tui.rs`, `apps/lidash/tests/cli.rs`)**:
   - Added `output::tests::evidence_export_formats_cleanly_with_privacy_declaration`.
   - Added `tui::tests::bounded_history_ring_caps_at_max_and_steps_correctly`.
   - Added `export_command_runs_cleanly_on_all_platforms` and `export_command_writes_to_output_file` in `cli.rs`.
   - Added fixture-based test `export_command_reads_fixture_evidence` on Linux.

## Verification

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo test --workspace --locked --all-targets`: PASS (103 tests passed across workspace)
- `python scripts/check_docs.py`: Verified documentation links
