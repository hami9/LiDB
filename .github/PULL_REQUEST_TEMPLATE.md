## Release intent

Conventional Commit PR title: patch (`fix:`, `docs:`, `ci:`) | minor (`feat:`) | major (`feat!:`, BREAKING CHANGE). Document any `Release-Bump:` override. No release will occur before binaries pass CI.

## User need and roadmap phase

Describe the problem, active phase and bounded scope.

## Implementation

Changed modules, alternatives and compatibility / migration effects.

## Capabilities and safety

Feature flag, fallback, privilege requirement, privacy classification, resource cost and license/dependency changes.

## Tests actually executed

Commands, environments, exit codes and reproducible evidence. Clearly label NOT RUN and simulated vs real GPU/multi-node verification.

## Five-axis review

- Kernel/networking:
- AI/GPU/fabric:
- Security/privacy:
- Performance/reliability:
- Product/UX/extensibility:

## Documentation and handoff

- [ ] Updated docs/help and relevant ADR
- [ ] Updated `.AGENTS/WORKLOG.md`
- [ ] Updated `.AGENTS/STATE.md` to actual verified state
- [ ] No unsupported claims or secrets in diff
- [ ] Maintainer approval requested for sensitive changes

## Parallel-agent ownership

- Worktree agent / task / branch:
- GitHub issue:
- Owned file scope (and files explicitly excluded):
- Cross-agent dependencies and integration order:
- Coordinator aware of shared-schema or CI changes: Yes / No
