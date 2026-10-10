# P0 Frame Codec Worklog — Issue 10

- Owner: ChatGPT GitHub integration.
- Branch: `agent/chatgpt/p0-framing`, based on `agent/chatgpt/p0-core`.
- Scope: pure local framing and negative-path tests; no socket or privilege boundaries opened.
- Files: `crates/lidb-protocol/src/framing.rs`, `crates/lidb-protocol/src/lib.rs`, `docs/ipc-framing.md`.
- TUI `prototypes/tui/` remains Antigravity-owned and untouched.
- Validation: see stacked PR CI, no claim of hardware validation or external runtime.
- Integration: merge PR #9 first, then retarget this stacked PR to `main` through a non-rewriting update and final checks.
