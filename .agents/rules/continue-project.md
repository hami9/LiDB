---
trigger: always_on
description: "LiDB startup and continuation: load shared GitHub style, resume the persistent Antigravity task queue, verify real tests and preserve worktree ownership."
---

# Persistent LiDB delivery instructions

For every LiDB session, read [AGENTS.md](../../AGENTS.md), the [shared GitHub style skill](../skills/github-style/SKILL.md), [.AGENTS/STATE.md](../../.AGENTS/STATE.md) and [.AGENTS/ANTIGRAVITY.md](../../.AGENTS/ANTIGRAVITY.md). Follow the startup/resume loop and select the first eligible task there; the maintainer has requested this continuation plan so routine task selection does not need another prompt.

Inspect actual code, branch, task notes and GitHub status before trusting a checklist. Resume your existing owned task before starting another. Use a dedicated Antigravity worktree and a scoped task note. Implement, test, update docs, commit, push, open a focused PR and verify available CI. Continue an eligible independent task when another PR is under review; preserve dependencies and record a stacked PR's base explicitly.

Ask only when required information, conflicting ownership or explicit authorization blocks a concrete action. Ordinary implementation choices, reversible fixes and test execution within the queue are already in scope. Merge, publishing, repository administration, privileged operations and active probes still require explicit authorization. Never interpret a pending review or unavailable test as approval or success.

This rule is persistent context, not a scheduler: it cannot launch an agent, grant tools or keep a closed session running. Leave exact progress and the next eligible task in your task note before the session ends.
