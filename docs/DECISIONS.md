# Architecture decision records (ADR)

Decisions are **proposed** unless explicitly accepted by maintainers. Changing one requires an ADR entry with context, alternatives, tradeoffs, security effects and migration.

| ID | Status | Decision | Reason |
| --- | --- | --- | --- |
| ADR-001 | proposed | Rust userspace + C eBPF | Safe orchestration; native CO-RE integration |
| ADR-002 | proposed | Terminal-first, no web UI | Fast SSH workflows, low dependencies |
| ADR-003 | proposed | Unprivileged TUI; separate optional privileged helper | Reduce blast radius |
| ADR-004 | proposed | Read-only, metadata-only baseline | Operator trust and privacy |
| ADR-005 | proposed | Capability-gated integrations | Hardware/runtime support is highly variable |
| ADR-006 | proposed | Collector-local monotonic timestamps; no cross-domain comparability | Avoid false timeline correlations |
| ADR-007 | proposed | Default Tokio/epoll; optional later io_uring benchmark | Reduce initial complexity |
| ADR-008 | proposed | Defer external adapters; no native dynamic loading | Avoid unsafe native dynamic loading |
| ADR-009 | proposed | No automatic remediation or job scheduler | Needs separate safety and reliability validation |
| ADR-010 | proposed | MIT for project-owned code | Permissive open-source contribution and redistribution |
| ADR-011 | proposed | Semantic versioning for operator-visible interfaces | Predictable community ecosystem |
| ADR-012 | proposed | Explicit unavailable state, never numeric zero fallback | Preserve correctness |

Rust 1.85 is the selected minimum toolchain for the runnable foundation. Record native platform/toolchain evidence before claiming validation. Kernel fields are capability-detected; persistence, IPC and external adapters are deferred until an actual product need justifies a separate decision.

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

## ADR-021 — public Linux product scope (2026-10-09)

**Status:** accepted direction under the maintainer's instruction to remove non-general-product scope and complete Core. Implementation acceptance and release approval remain separate gates.

**Context:** The repository already has typed Rust core/protocol contracts and bounded telemetry history. The earlier architecture expanded into specialized vendor hardware, runtime integrations and distributed operations before an ordinary Linux operator had a useful baseline.

**Decision:** Keep and extend the existing core. Deliver one local read-only Linux application: bounded procfs collection → typed snapshot → text/JSON and terminal UI. Focus on CPU, memory, load, uptime, disk/interface counters and optional PSI. Retain the pure `lidb-protocol` compatibility library; remove inactive daemon scope and require no helper/IPC/service for startup. Optional deeper Linux networking/eBPF remains later, explicitly scoped work. The selected userspace minimum is Rust 1.85.

**Alternatives:** A daemon-first architecture adds lifecycle/authentication cost to ordinary procfs reads. Vendor-first integration or distributed control does not serve the requested general Linux baseline. Replacing the existing typed core would discard useful tested contracts. These alternatives are not chosen for this delivery.

**Consequences:** Active product docs, roadmap and agent guidance adopt the public scope. Existing history and protocol contracts are preserved, and snapshot JSON remains pre-stable. No privileged operation, outbound runtime network request, persistent store or remote listener is introduced. Local x86_64 and pending native aarch64 evidence remain distinct; runnable code is not production or phase-exit approval.

**Migration:** The CLI presents `snapshot`, `doctor` and `tui`; legacy scaffold `status`/`capabilities` remain compatibility aliases. The internal workspace version remains pre-release. There is no published stable release to migrate, and no installed daemon service is removed by application startup. Historical task/worklog records remain intact.
