# Quality gates — enforced for each feature/PR

## Functional and architecture

- [ ] User story and acceptance criteria documented
- [ ] Correct layer and domain boundaries, no duplicated kernel implementation
- [ ] Feature discoverability, config defaults, capability flags and fallback behavior
- [ ] Typed error and unavailable states, units/provenance/clock semantics
- [ ] Stable external contracts and migration or ADR where applicable

## Product safety

- [ ] Non-root baseline preserved; any privileged operation explicitly scoped
- [ ] Threat and privacy assessment updated, no new unintended network access
- [ ] No secret/payload capture by default; logs and support exports redact
- [ ] Third-party input bounded, failures isolated and observability overhead controlled
- [ ] License and dependency implications checked

## Verification

- [ ] Formatter/lint/unit checks passed **with actual commands and exit results**
- [ ] Negative/degraded/permission denied conditions tested
- [ ] Relevant integration and hardware validation correctly labeled
- [ ] Performance impact measured where pertinent, including dropped events
- [ ] CLI/TUI/headless and docs/test examples updated

## Delivery

- [ ] Five review axes recorded with findings, not just checkboxes
- [ ] Small scoped PR/commit; no unrelated churn
- [ ] Worklog entry includes evidence and known gaps
- [ ] STATE.md reflects facts (not desired state)
- [ ] Maintainer approval recorded for security/privilege/API/release changes

**Failing any required gate means the task remains pending.** Items requiring unavailable hardware are explicitly blocked, not waived by an agent.
