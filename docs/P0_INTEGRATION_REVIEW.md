# P0 Integration Review

Reviewed on 2026-10-08 by Codex for [Issue #14](https://github.com/hami9/LiDB/issues/14). This is a source and CI integration review, not approval to merge or a completed P0 milestone.

## Reviewed snapshots

| PR | Owner | Exact head | Integration boundary |
| --- | --- | --- | --- |
| [#9](https://github.com/hami9/LiDB/pull/9) | ChatGPT | `86f22115a85633f46a0ad32d2ef63e3d23ad3947` | Root workspace, typed core contracts, CLI/daemon scaffolds |
| [#11](https://github.com/hami9/LiDB/pull/11) | ChatGPT | `4a2a8d234721be701395c34308f7454e207eb234` | Bounded byte framing; PR base is the core branch |
| [#13](https://github.com/hami9/LiDB/pull/13) | Antigravity | `54b0d03c6ed5585cec6f2666936801244fe2a798` | Independent `prototypes/tui` workspace and fixtures |

Existing CI reported success on these heads. However, the old Rust jobs skipped PR #13 because it had no root manifest; CodeQL Rust also skipped it. That green gate was not TUI compilation evidence. The CI slice adds required native TUI checks and standalone CodeQL detection without editing the owners' runtime files.

## Integration order and readiness

1. Land the CI slice through maintainer review, then rerun the updated checks on PR #13's current head.
2. Review and land #9 before #11. After #9 lands, update #11's target/base through the owning agent's normal workflow and rerun checks against main.
3. #13's explicit `[workspace]` keeps it outside the root workspace. It can coexist with the core/framing code, but it does not consume those contracts. A later adapter task must preserve provenance, explicit missing-data states and units; no duplicate domain schema should become a stable API by accident.
4. Keep P0 open until the supported toolchain/kernel decision, CLI doctor/demo acceptance, actual daemon transport and UI/core integration are reviewed and tested. Native ARM compilation is not DGX Spark validation.

## Findings and limits

- Framing rejects zero-length and oversized payloads, caps input chunks, detects incomplete EOF and poisons malformed streams. It carries opaque bytes only. Callers must still bound their queues and implement schema validation and Unix peer authentication in a later transport slice.
- The prototype defaults GPU state to `NotProbed`; fixture mode separates NVLink-C2C inside a node from ConnectX between nodes. Its fixture constants and labels remain synthetic, including bandwidth and power values. No hardware accuracy claim follows from render tests.
- The CLI's `--headless-test` renders all seven tabs; modal and small-terminal rendering are covered by the existing test suite. Real terminal restoration on panic, setup failure and interrupted sessions remains an operator test. The guard is created after entering the alternate screen; a setup error after enabling raw mode deserves a focused owner review.
- Root workspace Rust 1.82 is declared in #9, but current CI uses stable. This slice does not prove the minimum supported Rust version for the separate TUI dependencies.
- Main's bootstrap documentation and security status still describe specification-stage software. They must be reconciled when runtime PRs land. The architecture phase table differs from `.AGENTS/PHASES.md` after P0; the maintainer must resolve that discrepancy before subsequent phases. This slice uses the shared P0 foundation scope.

## Verification

Local Python policy tests, document links and whitespace checks are recorded in the [task worklog](../.AGENTS/worklogs/issue-14.md). Neither Windows nor the checked Ubuntu WSL environment has Cargo.

The [TUI-only CI run](https://github.com/hami9/LiDB/actions/runs/37776716589) passed at `0985b30580525ce346b5c1a596896b36f4517089`, proving the standalone checks execute without a root manifest. The [combined CI run](https://github.com/hami9/LiDB/actions/runs/37776757450) passed at `76aa72c3adc939edca15204b4656a10b91e4027d`, combining the three reviewed heads and CI implementation `3210e66`. Both runs executed the full TUI checks on native x86_64/aarch64. The combined run also executed the root Rust checks: 14 root and 20 TUI tests per architecture passed; documentation commands passed with zero cases. These results establish build/test coexistence, not functional UI/core integration. Stable Rust resolved to 1.99.0, not an MSRV test.

The validation snapshots are isolated on `agent/codex/tui-validation`; the feature [PR #15](https://github.com/hami9/LiDB/pull/15) remains free of other owners' runtime changes. The [standalone CodeQL run](https://github.com/hami9/LiDB/actions/runs/37776710115) passed at the TUI-only snapshot: Actions, detection and Rust analysis succeeded; C analysis correctly skipped absent code.

## Five-axis review

- Kernel/networking: CI and source review only; no system configuration, collectors or privileged tests.
- AI/GPU/fabric: fixture-only UI checks; GPU, GB10, ConnectX and NCCL validation pending.
- Security/privacy: read-only PR workflow token, credentials not persisted, no listeners or payload collection introduced; authenticated IPC pending in the owner roadmap.
- Reliability: matrix jobs retain independent results; non-success required jobs block the gate. Timeouts bound all new jobs. Existing fixture/render tests execute when code is present.
- Product/ecosystem: docs-only bootstrap remains supported; standalone prototype is visible to CI; actual UI/core adapter and terminal lifecycle acceptance pending.

Codex independently reviewed the other owners' source but self-reviewed its own CI changes. Maintainer review is still required before merge. No releases, branch rulesets or published tags were changed.
