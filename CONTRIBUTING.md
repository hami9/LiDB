# Contributing to LiDashBoard

LiDB is an open-source, local Linux terminal diagnostic product in core development.

## Before contributing

Read [README.md](README.md), [AGENTS.md](AGENTS.md), [ROADMAP.md](ROADMAP.md), [SECURITY.md](SECURITY.md) and [capability model](docs/CAPABILITY_MODEL.md). Discuss substantial scope or architecture changes in an issue first. Keep contributions useful to ordinary Linux operators and preserve the read-only unprivileged baseline.

## Development

Use Rust 1.85 or newer and the committed Cargo lockfile. Run the format, lint, test and link checks shown in the README. Test actual Linux sources separately from deterministic fixtures and record the architecture. See [test strategy](docs/TEST_STRATEGY.md).

## Pull requests

- Keep one independently testable issue and focused file scope per PR.
- Describe the operator problem, resulting behavior, compatibility, privacy/permissions and fallback.
- Update meaningful tests and operator documentation; record exact commands and real results.
- Use English in source, documentation, reviews and commits.
- Use Conventional Commit PR titles, for example `feat(core): add pressure observations` or `fix(cli): handle redirected output`.
- Write concise imperative commit subjects, such as `Add Pressure Observations` or `Handle Redirected Output`. Squash merging preserves the PR's release intent; use an explicit `Release-Bump:` footer when individual commits are rebased onto main.
- Do not claim unrun platform, kernel, terminal or benchmark evidence.
- Retain copyright notices under the MIT license; no CLA is currently required.

Agent work follows [.AGENTS/WORKFLOW.md](.AGENTS/WORKFLOW.md). Parallel authors use separate [worktrees](.AGENTS/WORKTREES.md); the coordinator owns shared state/worklog/schema/CI integration. No force-push, unrelated overwrite or unauthorized self-merge.

## Reviews and safety

Review Linux correctness, telemetry semantics, security/privacy, performance/reliability and operator UX. Sensitive changes require independent review. Use disposable environments for optional privileged kernel/network testing and obtain actual operator authorization before changing any system configuration. Core collection itself must remain read-only.

## Version and automation

PR titles determine automatic SemVer intent once release gates are enabled: `feat:` means minor, breaking changes mean major, and routine fixes/docs/CI default to patch. A documented `Release-Bump: major|minor|patch` merge-commit footer can override the default. See [CI/CD policy](docs/CI_CD.md) and [CI/CD skill](.AGENTS/skills/ci-cd/SKILL.md). The required `CI Gate` is enforced only after repository rulesets are administratively activated. No public release is implied by a successful local build.

Follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Report vulnerabilities via [SECURITY.md](SECURITY.md), rather than posting sensitive reproduction data in a public issue.
