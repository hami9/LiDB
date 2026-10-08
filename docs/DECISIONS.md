# Architecture decision records (ADR)

Decisions are **proposed** unless explicitly accepted by maintainers. Changing one requires an ADR entry with context, alternatives, tradeoffs, security effects and migration.

| ID | Status | Decision | Reason |
| --- | --- | --- | --- |
| ADR-001 | proposed | Rust userspace + C eBPF | Safe orchestration; native CO-RE integration |
| ADR-002 | proposed | Terminal-first, no web UI | Fast SSH workflows, low dependencies |
| ADR-003 | proposed | Unprivileged TUI; separate optional privileged helper | Reduce blast radius |
| ADR-004 | proposed | Read-only, metadata-only baseline | Operator trust and privacy |
| ADR-005 | proposed | Capability-gated integrations | Hardware/runtime support is highly variable |
| ADR-006 | proposed | Monotonic timestamps locally; quantify cross-node time uncertainty | Avoid false timeline correlations |
| ADR-007 | proposed | Default Tokio/epoll; optional later io_uring benchmark | Reduce initial complexity |
| ADR-008 | proposed | Community adapters out of process | Avoid unsafe native dynamic loading |
| ADR-009 | proposed | No autonomous remediation or job scheduler initially | Needs separate safety and reliability validation |
| ADR-010 | proposed | MIT for project-owned code | Permissive open-source contribution and redistribution |
| ADR-011 | proposed | Semantic versioning for operator-visible interfaces | Predictable community ecosystem |
| ADR-012 | proposed | Explicit unavailable state, never numeric zero fallback | Preserve correctness |

Decisions to settle during P0: Rust minimum supported version, minimum kernel baseline, supported distributions, deployment/packaging policy, IPC encoding, local storage format, plugin protocol initial version and build-time feature flags. Publish the concrete choices and test data before treating them as settled.

## Automation decisions (2026-10-08)

- **ADR-013, proposed:** Conventional Commits are the single automatic SemVer bump signal; explicit `Release-Bump:` commit footers may override. Non-feature commits default to PATCH.
- **ADR-014, proposed:** Release only after reusable CI and reproducible architecture-specific builds. Bootstrap docs-only commits must not produce a pretend binary.
- **ADR-015, proposed:** Protect `main` with a stable aggregated `CI Gate`, PR-only squash/rebase, no deletion/force-push. Ruleset activation is an administrative operation outside coding-agent authority.
- **ADR-016, proposed:** Public build archives include checksums and GitHub provenance attestations. This does not replace independent artifact verification or SBOM/license review.

## Multi-agent execution decisions (2026-10-08)

- **ADR-017, proposed:** Use real Git worktrees in an ignored `.worktrees/` folder for local agent isolation; no committed `.git` internals or copies of the repository.
- **ADR-018, proposed:** GitHub Issues + protected branch PRs are the cross-device ownership and integration control, not `git worktree` locking.
- **ADR-019, proposed:** The canonical agent policies remain `.AGENTS/` plus root `AGENTS.md`; integration shims `CLAUDE.md` and `.agents/rules/` are limited adapters.
- **ADR-020, proposed:** A single integrator owns central `.AGENTS/STATE.md`, `.AGENTS/WORKLOG.md`, shared schemas and CI while task agents edit disjoint modules.
