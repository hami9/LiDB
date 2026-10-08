# LiDB agent workspace

This **version-controlled workspace** coordinates multiple AI coding agents. It is documentation and policy: it does not grant a tool or model capabilities that are not available in the actual runtime.

## Navigation

| File | Role |
| --- | --- |
| [SYSTEM_PROMPT.md](SYSTEM_PROMPT.md) | Project agent role and operating constraints |
| [RULES.md](RULES.md) | Immutable correctness, privacy, security and collaboration rules |
| [START_HERE.md](START_HERE.md) | Ready-to-use agent kickoff instructions |
| [WORKFLOW.md](WORKFLOW.md) | Branch/task/implementation/review/handoff loop |
| [STATE.md](STATE.md) | Authoritative latest confirmed phase and blockers |
| [PHASES.md](PHASES.md) | Executable phase decomposition and dependency gates |
| [QUALITY_GATES.md](QUALITY_GATES.md) | Definition of done and gate criteria |
| [WORKLOG.md](WORKLOG.md) | Append-only factual task record |
| [TASK_TEMPLATE.md](TASK_TEMPLATE.md) | Template for each agent-owned task |
| [HANDOFF_TEMPLATE.md](HANDOFF_TEMPLATE.md) | Handoff report template |
| [REVIEW_5_AXES.md](REVIEW_5_AXES.md) | Five domain reviews before phase completion |
| [SKILL_POLICY.md](SKILL_POLICY.md) | How to choose and validate skills |
| [skills/README.md](skills/README.md) | Local topic-specific agent playbooks |

## Canonical source of truth

Source code and tests prove behavior. [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) is the intended design, [STATE.md](STATE.md) shows progress, and [WORKLOG.md](WORKLOG.md) contains dated evidence. **If these disagree, do not silently choose whichever is most convenient.** Record the discrepancy and request human resolution.

## Current status

P0 has not been started and no runtime code has been implemented. Agents must never infer otherwise from the existence of comprehensive documentation.

## Agents and responsibilities

Suggested roles (not separate permanent personalities): design reviewer, implementation engineer, test engineer, security reviewer, documentation/release reviewer. A single agent may fill multiple roles for simple tasks, but **must not call self-review an independent review**.

Every agent should keep contextual assumptions in its task description, pass explicit next steps during handoff and avoid accumulating unreviewed agent-generated code.
