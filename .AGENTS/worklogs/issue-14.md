# Issue 14: TUI CI Gate

- Owner: Codex; P0 foundation CI slice.
- Branch: `agent/codex/tui-gate`; worktree: `.worktrees/codex/tui-gate`.
- Acceptance: standalone TUI detection without root Cargo, locked native x86_64/aarch64 checks, headless/smoke execution, fail-closed CI Gate, negative regression coverage, accurate docs and integration review.
- Scope: CI and CodeQL detection, Python helpers/tests, CI docs, changelog and serialized coordinator state/worklog. Other agents' runtime files and branches remain untouched.
- 2026-10-08: Windows Python 3.14, `python -m unittest discover -s tests/ci -p 'test_*.py' -v`: exit 0, 24 tests passed. `python scripts/check_docs.py`: exit 0, 125 local Markdown links passed before adding this report. `git diff --check`: exit 0.
- Local Cargo checks: NOT RUN; Cargo is absent on Windows and the checked Ubuntu WSL environment. Native Linux runtime CI evidence pending on the integration snapshot.
- No privileged/system changes, dependency installation, hardware tests, release publication or merge performed.
- Review findings and owner handoff: [P0 integration review](../../docs/P0_INTEGRATION_REVIEW.md).
