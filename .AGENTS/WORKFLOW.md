# Standard agent delivery workflow

## 1. Discover

Read project context and current phase. Inspect Git branch, status, current PRs and unfinished work. Identify capability flags and API constraints. Research vendor/kernel interfaces through primary sources where needed; record source version and any uncertainty.

## 2. Specify

Use [TASK_TEMPLATE.md](TASK_TEMPLATE.md) for one vertical slice. Include user story, assumptions, privilege boundaries, explicit `not supported` behavior, expected cost and reproducible tests. Resolve ambiguity before making irreversible changes.

## 3. Implement

Prefer pure domain logic behind stable traits and fixtures, then platform adapters. Keep initial change sets reviewable. Feature flags must default off for experimental or privileged integrations. Avoid unrelated cleanup.

## 4. Validate

Run formatting, linting, tests and appropriate integration checks. For hardware claims, collect reproducible reports on actual labeled devices; otherwise mark NOT VALIDATED. Record regression and negative-path tests as well as happy paths.

## 5. Review

Use [REVIEW_5_AXES.md](REVIEW_5_AXES.md). Security-sensitive and API-breaking changes need independent human/domain review; agents should not treat self-review as approval.

## 6. Handoff

Update [WORKLOG.md](WORKLOG.md) and [STATE.md](STATE.md), docs, tests and changelog. Fill out [HANDOFF_TEMPLATE.md](HANDOFF_TEMPLATE.md); open a PR with targeted scope. Do not merge or trigger release actions unless explicitly authorized.

## Parallel work

Allocate non-overlapping modules/branches to separate agents. One integrator owns shared schemas and merges. Agents must not concurrently rewrite the worklog or `STATE.md` without coordination. If collaboration is asynchronous, consolidate logs during handoff with source branch/commit references.
