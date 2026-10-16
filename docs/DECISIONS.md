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
| ADR-007 | superseded in Core candidate by ADR-022 | No required async runtime; standard collection worker | Match the implemented bounded local baseline |
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

## ADR-022 — implemented Core boundaries and ABI validation (2026-10-12)

**Status:** implementation proposal in the issue #26 follow-up; maintainer integration/phase acceptance remains pending.

**Context:** The runnable baseline has acyclic pure-Core → collector → application layering, a standard collection thread and one bounded latest-snapshot slot. Earlier proposals suggested Tokio/epoll without an implemented need. A shared numeric Linux open flag was wrong for arm64; static symlink preinspection tests did not exercise the actual open defense. Pure Core also retained an unused product bootstrap registry with obsolete disabled declarations.

**Decision:** Keep the existing in-process layers and standard worker; add no async runtime or daemon for current procfs reads. Preserve the maintainer's already-integrated architecture-specific `O_NOFOLLOW`/`O_NONBLOCK` implementation; PR #27 owns direct final-component symlink regression coverage on both native architectures. Preserve the maintainer's removal of the unused product bootstrap helper from pure Core; source availability belongs to actual collector observations, while generic capability construction remains platform-independent.

**Alternatives:** Adding libc solely for two constants is unnecessary for the existing two-target baseline; sharing the wrong ABI value is incorrect. An async wrapper alone cannot cancel a synchronous kernel read. A daemon/transport or invented timeout guarantee adds cost without resolving that boundary. Lifecycle isolation/timeout design is explicitly tracked in AG-01.

**Consequences and migration:** Snapshot JSON 0.1 and protocol negotiation are unchanged. The pre-stable, unused `capability::bootstrap_registry` helper was already removed in merged PR #25; callers can use an empty `CapabilityRegistry` and explicitly checked registrations. Intermediate fixture directory races, synchronous read stalls, clock-domain replay identity and release evidence remain documented limits. No unsafe code, new dependency, host configuration or privilege is introduced.

## ADR-023 — bounded interactive collector lifecycle (2026-10-16)

**Status:** implemented proposal for AG-01 / issue #43, pending maintainer integration. This refines ADR-022's lifecycle policy without changing source or snapshot contracts.

**Context:** Refresh cancellation woke the interval wait, but `Worker::shutdown` unconditionally joined a synchronous sample. A blocked read could therefore retain the terminal application indefinitely. One snapshot slot and file byte limits bounded memory, not read time. An in-process completion message would not prove that thread-local/captured destructors had finished.

**Decision:** Keep one standard thread and one latest-snapshot mailbox. Acquire process-wide collector admission before spawning, release it only after an actual join or spawn failure, and permanently close admission until process exit after any shutdown timeout. Serialize cancellation, batch admission and publication through the mailbox mutex; hold no mailbox lock across source reads. A batch admitted before cancellation may finish remaining reads if resumed, but cannot publish or start another batch.

The TUI checks a fixed 5-second collection-batch deadline every event-loop iteration (normally up to 50ms input polling), including while paused and before a first sample. The refresh wait is outside that deadline. An overdue or late-completed batch is discarded and reported as `io::ErrorKind::TimedOut`, not fabricated per-source readings. The event loop also rechecks the deadline after observing a finished worker so a racing late completion retains its timeout reason.

Restore the terminal before worker cleanup on every return path. Cancellation wakes interval waiting; shutdown checks `JoinHandle::is_finished` for at most a 100ms userspace grace, joining only a finished thread. On timeout, detach the one unfinished worker, return an explicit `TimedOut`, consume its handle and cache the outcome so repeated shutdown/Drop does not repeat the wait. The CLI reports runtime failure (exit 2); callers cannot start a replacement in this process even if the detached worker later returns.

**Alternatives:** Unconditional join can hang. Releasing admission from the callback or a completion notification is premature because capture/TLS destruction may still block. Repeated detached replacement workers can exhaust resources. An async wrapper cannot cancel a synchronous kernel read. A separate helper process could improve isolation but adds transport/lifecycle cost and does not guarantee termination of an uninterruptible kernel task; no new helper, daemon, dependency or privilege is warranted for this slice.

**Limits and migration:** This bounds application waiting on samples under ordinary scheduling, not OS scheduling, terminal I/O, pathological thread teardown or hard real-time process death. An arbitrary kernel read remains uncancellable; snapshot/doctor remain synchronous with no read deadline. Process restart is required after a shutdown timeout. Snapshot JSON 0.1, source availability reasons, polling flags and pure Core/protocol APIs are unchanged. Fixtures must remain trusted/static. No new unsafe code, dependency, runtime network access or host configuration is introduced.

**Verification:** Deterministic blocked callbacks exercise explicit timeout, repeated shutdown/Drop, permanent admission, late publication suppression, deadline boundary, captured-destructor stalls and panic cleanup. Linux PTY subprocesses exercise q/raw Ctrl-C/SIGINT/SIGTERM and the no-input watchdog at a 60000ms interval, requiring exact termios/cursor/alternate-screen restoration before diagnostics. These callbacks are simulations, not uninterruptible-kernel or performance validation. Actual commands, independent review and native CI belong to [issue #43 evidence](../.AGENTS/worklogs/issue-43.md).
