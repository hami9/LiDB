---
name: lidb-ci-cd
description: GitHub Actions security, cross-architecture CI, semantic releases and protected branches.
---
# CI/CD and release engineering skill

**Use when:** modifying workflows, build artifacts, release scripts, version intent, status checks, rulesets, CODEOWNERS or dependency updates.

## Operational model

- Work on feature branches and PRs; never push direct to protected main.
- Use Conventional Commit PR titles: `fix:`, `feat:`, `feat!:` (or BREAKING CHANGE), and explicit `Release-Bump:` trailers when necessary.
- Before creating releases, run all reusable checks and verify real x86_64/aarch64 binaries, checksums and available GitHub provenance attestations.
- Version tags are immutable. Reject retries that would modify an existing version.
- A workflow file requires least-privilege token scopes, appropriate concurrency, bounded timeouts and safe event contexts. No publish token for fork PRs.
- Security scans, lint/tests, privacy/security review and maintainer approval remain necessary even when automation passes.
- The fixed required check is `CI Gate`. Preserve its job name or update the active GitHub branch ruleset **in a coordinated administrative operation**.
- Branch/tag rulesets must be applied by a maintainer with GitHub Administration write permission. Do not treat JSON as an applied setting.
- Document exact CI run URLs and real test outcomes in `.AGENTS/WORKLOG.md`.

## References

- [CI/CD policy](../../../docs/CI_CD.md)
- [Branch protection activation](../../../docs/BRANCH_PROTECTION.md)
- [Agent quality gates](../../QUALITY_GATES.md)
