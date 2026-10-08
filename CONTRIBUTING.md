# Contributing to LiDashBoard

Thanks for helping build an open-source infrastructure tool. This project is in the architecture/bootstrap phase.

## Before contributing

Read [README.md](README.md), [AGENTS.md](AGENTS.md), [ROADMAP.md](ROADMAP.md), [SECURITY.md](SECURITY.md), and [docs/CAPABILITY_MODEL.md](docs/CAPABILITY_MODEL.md). Discuss substantial feature or architecture changes in an issue before implementation.

## Pull requests

- Keep PRs small, focused and scoped to an existing roadmap phase.
- Explain user problem, implementation, alternatives, security/privilege changes and fallback behavior.
- Update documentation, tests, changelog when externally visible and [.AGENTS/WORKLOG.md](.AGENTS/WORKLOG.md) for agent-generated work.
- Include exact test commands and environment; distinguish simulated and real hardware.
- Do not claim GPU/NCCL/NVLink behavior tested on hardware without test evidence.
- Use English across code, docs, reviews and commit messages.
- Preferred commits: `docs(scope): description`, `feat(scope): description`, `fix(scope): description`, `test(scope): description`.
- External contributors retain their copyright under the project's MIT license; no CLA currently required.

## Review expectations

Reviews consider kernel correctness, AI/hardware semantics, security/privacy, performance/reliability and product UX/compatibility. Sensitive changes require a separate security review. Maintainers may reject premature features or out-of-scope changes.

## Development expectations

Never run privileged kernel/network diagnostics on shared or production systems without express authorization. Use reproducible fixtures and designated disposable test environments. See [docs/TEST_STRATEGY.md](docs/TEST_STRATEGY.md).

## Conduct

Follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md); report sensitive conduct issues privately to maintainers.
