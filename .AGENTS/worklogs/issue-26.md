# Issue #26 — shared delivery skill, Antigravity continuation and Core review

- Owner: Codex coordinator; no additional agents launched or delegated for this task.
- Branch/worktree: `agent/codex/agent-handoff`, `.worktrees/codex/agent-handoff`.
- Base: `ec1d307fc9fcfa0da390829205407923a222eb4a` on `agent/codex/public-core` (PR #25, still open when checked). Created a fresh worktree from main, then explicitly fast-forwarded only this new branch to the dependency candidate. Primary main and existing worker branches remain unchanged.
- Scope: maintainer-supplied skill/discovery/installation, persistent Antigravity queue, actual architecture report/ADR and bounded Core boundary fixes. All repository text is English.
- Provenance: skill instructions were read from the supplied temporary attachment before it disappeared between resumed sessions; the retained text is now tracked. Canonical source SHA-256: `46e57b8977c82a6e675ba3ee34deea84c7f7243e7a582388aaba80ceeda29e4b`. The digest identifies this installed repository copy, not an independently compared original attachment binary.
- Local installation: `python3 scripts/install_github_style.py --agent all` installed the skill in local Codex (modern user scope and configured CODEX_HOME), Claude and Antigravity IDE/CLI locations. A repeat reported matching copies already installed. Different existing files and final symlinks are refused. No other machine or account-wide plugin catalog was modified; actual native agent clients were not launched.
- Verified defects: arm64 O_NOFOLLOW mismatch confirmed against Linux v6.18 headers; fixed named flags and added a direct open regression. Removed unused bootstrap-only registry/test with obsolete availability claims. Corrected architecture table and async-runtime proposal drift. Snapshot schema/protocol version unchanged; pre-stable helper migration documented.
- Remaining limits: synchronous source reads and thread joins lack hard deadlines; intermediate fixture directory races require trusted static roots; replay clock identity, structured networking, measured overhead, notices/SBOM and wider compatibility remain future queue tasks. Source/dependency review is coordinator self-review, not independent/human approval.

## Initial candidate verification — 2026-10-12, before remote reconciliation

Environment: Debian 13.6 / Linux 6.18.44 x86_64 / Rust 1.85.0 / unprivileged UID 1000. Commands below exited 0 unless a result explicitly says otherwise.

| Command | Actual result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --locked --all-targets` | PASS: 49 (19 Core, 2 protocol, 15 collector, 13 app) |
| `cargo test --workspace --locked --doc` | PASS with zero cases |
| `python3 -m unittest discover -s tests/ci -p 'test_*.py' -v` | PASS: 21; includes three installer collision/idempotency/symlink tests |
| `python3 apps/lidash/tests/pty_smoke.py target/debug/lidash` | PASS: q/Ctrl-C/SIGINT/SIGTERM/resize/exact terminal restoration |
| `target/debug/lidash snapshot --json` / `doctor --json` | PASS: live schema 0.1, 146 observations, shared batch time; absent PSI yields degraded availability |
| `cargo metadata --locked --no-deps --format-version 1` | Actual acyclic workspace edges: collect→Core; app→collect/Core; protocol independent |
| Skill/rule frontmatter parsing with PyYAML | PASS: expected name/description and `trigger: always_on` |
| Installer repeat and installed SHA-256 | PASS: canonical content retained at all printed local destinations |
| `python3 scripts/check_docs.py` / `git diff --check` | PASS: 195 local Markdown links and whitespace |

Native GitHub results belong to the delivery PR/checks; they must be observed, not inferred. No public release build was required by this change and no release/tag/merge was performed. Resume by reading the final PR and `.AGENTS/ANTIGRAVITY.md` before taking AG-00/01; leave dependent phases under review until their own acceptance evidence exists.

## Reconciliation with current main — 2026-10-12

- Fresh fetch found PR #25 already merged with maintainer `ec50747` fixing ARM flags and removing bootstrap declarations, and PR #34 adding CPU/Memory UI. Current main is `842f877`; prereleases v0.1.0 and v0.2.0 are published with both native archives/checksums.
- Merge current main into this published branch without history rewriting. Preserve its runtime source, CLI regression and CPU/Memory tab. Withdraw our duplicate reader refactor/regression because PR #27 already owns direct-open coverage and contains native failing-before/passing-after evidence. No other owner's branch or PR was modified or merged.
- Retarget our PR #35 to main, update its scope to shared skills/persistent handoff and factual architecture documentation, and reconcile state/release claims. The original candidate-only local results above remain historical evidence; final main-based checks must be recorded separately.

Final main-based local validation exited 0: formatter, strict locked Clippy, 51 Rust tests (19 Core + 2 protocol + 14 collector + 16 app), doctest command (zero cases), PTY lifecycle and live snapshot/doctor (146 observations). The three new CPU/Memory/tab tests ran; current runtime files are identical to main. The 21 Python tests cover the unchanged installer/policy code; final Markdown links (195) and whitespace passed. Native CI/CodeQL results are on PR #35's exact final head. No pending owner PR was imported or closed.
