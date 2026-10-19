# Worklog — Dedicated TUI CI Integration

- **Owner**: Codex Integration Agent
- **Branch**: `agent/codex/p0-ci-integration`
- **Worktree**: `.worktrees/codex/p0-ci-integration`
- **Scope**: Dedicated GitHub Actions CI validation for standalone Ratatui TUI prototype, cross-PR integration testing, and adapter architecture review.

## Subsystems & Infrastructure Delivered

1. **GitHub Actions TUI Quality Gate (`.github/workflows/ci.yml`)**:
   - Added `tui-checks` job matrix:
     - `x86_64`: runs on `ubuntu-24.04`.
     - `aarch64`: runs on native `ubuntu-24.04-arm`.
   - Executes via `scripts/ci/check_tui.py`:
     - If `prototypes/tui/Cargo.toml` is absent: reports absence and exits gracefully with success.
     - If `prototypes/tui/Cargo.toml` is present: requires `Cargo.lock` and executes formatting (`cargo fmt --manifest-path prototypes/tui/Cargo.toml --all -- --check`), linting (`cargo clippy --manifest-path prototypes/tui/Cargo.toml --workspace --all-targets --locked -- -D warnings`), unit/integration tests (`cargo test --manifest-path prototypes/tui/Cargo.toml --workspace --all-targets --locked`), doc tests (`cargo test --manifest-path prototypes/tui/Cargo.toml --workspace --doc --locked`), headless rendering (`cargo run --manifest-path prototypes/tui/Cargo.toml --locked -- --headless-test`), and smoke tests (`cargo run --manifest-path prototypes/tui/Cargo.toml --locked -- --smoke-test`).
   - Integrated into `CI Gate`:
     - `ci-gate` job requires `[docs-and-policy, rust-checks, worktree-windows, tui-checks]`.
     - Evaluates results via `scripts/ci/check_gate.py` ensuring fail-closed aggregation.
2. **CodeQL Detection Update (`.github/workflows/codeql.yml`)**:
   - Updated Rust detection step to activate CodeQL Rust analysis when either root `Cargo.toml` or `prototypes/tui/Cargo.toml` exists.
3. **Workspace Detection & Gate Scripts (`scripts/ci/`)**:
   - `scripts/ci/check_tui.py`: Detects standalone TUI workspace and enforces committed `Cargo.lock`.
   - `scripts/ci/check_gate.py`: Fails closed if any required job is not `success`.
4. **Automated Unit Tests (`tests/ci/test_tui_gate.py`)**:
   - Verified detection of absent workspace, valid standalone workspace, partial workspace errors, and gate exit codes across all status combinations.
5. **Cross-PR Reproducibility (Task B)**:
   - Verified in disposable local worktree (`.worktrees/codex/tui-validation`):
     - PR #9 (`86f2211`), PR #11 (`4a2a8d2`), and PR #13 (`54b0d03`).
     - Root workspace crates (`lidb-core`, `lidb-protocol`, `lidash`, `lidashd`) compile and pass all 14 tests.
     - Standalone TUI prototype compiles and passes all 20 tests.
     - Zero workspace conflicts or duplicate dependencies.
6. **Adapter Architecture Review (Task C)**:
   - Documented in `docs/P0_INTEGRATION_REVIEW.md`:
     - `Disabled` must not automatically become `SimulatedFixture`.
     - `NotProbed`, `Unsupported`, and `PermissionDenied` must maintain distinct UI badges.
     - Missing telemetry must never become numeric zero.
     - Telemetry observation provenance (`source`, `monotonic_ns`) must be exposed.
     - IPC framing must not be confused with authenticated socket transport.

## Verification Evidence

- `python -m unittest discover -s tests/ci -p 'test_*.py' -v`: Passed (24 tests passed).
- `python scripts/check_docs.py`: Passed (all markdown links valid).
- Standalone TUI verification (Antigravity commit `54b0d03`):
  - `cargo fmt --check`: Passed.
  - `cargo clippy --all-targets --locked -- -D warnings`: Passed (0 warnings).
  - `cargo test --locked --all-targets`: Passed (20 passed).
  - `--headless-test` & `--smoke-test`: Passed.
