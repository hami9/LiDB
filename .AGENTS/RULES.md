# Mandatory engineering and agent rules

## Correctness

R01. Never invent metric values, benchmark outcomes, platform evidence or completed tasks.

R02. Do not model conntrack, nftables, route lookup and namespaces as a single unconditional packet-processing sequence; distinguish ingress, egress, forwarding, hooks and priorities.

R03. Preserve general Linux scope. Treat CPU activity, load, memory availability, cumulative counters and PSI according to their distinct source semantics; do not add vendor-runtime or distributed-workload commitments.

R04. Separate source data, derived data, hypotheses and verified diagnosis with provenance and uncertainty. A missing counter does not mean zero.

## Security, privacy and availability

R05. Non-root CLI/TUI and read-only baseline. Use a privileged helper only for an explicitly authorized and narrowly audited operation.

R06. Do not execute arbitrary remote code, run invasive diagnostics, change firewall/routes/network parameters, install kernel modules or alter running workloads without explicit operator permission.

R07. Never capture packet payloads, decrypted TLS, process environments, command-line arguments or secrets by default. Scrub support bundles and logging.

R08. Validate untrusted kernel events, RPC messages and plugin input. Bound memory, queues, timeouts and execution overhead.

R09. Do not add a remote network listener, telemetry export, update agent or dynamic native plugin loader without threat-model and approval.

R10. Local discovery cannot assume eBPF, container runtimes, broad procfs access, privileges or external services.

## Product and collaboration

R11. All source material in the repository is English.

R12. Each feature has a capability declaration, test suite, help/docs, failure behavior, configuration/flag strategy and target compatibility record.

R13. Preserve existing files and manually-authored work. No force-push, mass overwrite, history rewrite or self-approval.

R14. No fabricated test evidence. Record commands, environments, exit statuses, skipped cases and actual known issues.

R15. Update `WORKLOG.md` and `STATE.md` for each merged feature slice. Do not claim release readiness before gates pass.

R16. Never silently install external skills/plugins/dependencies. Check provenance, license, security impact, permissions and maintainer policy.

R17. Stable API/schema changes follow SemVer and migrations; architectural boundary changes require an ADR.

R18. Optimize measurement overhead, but never hide dropped events, collection gaps or throttling.

R19. Hardware-specific claims must be confirmed on real hardware, explicitly identified; mock tests are not hardware validation.

R20. When rules conflict or a required approval is missing, stop, document the conflict and ask a maintainer.

## Automation requirements

R21. PR titles follow Conventional Commits. Do not turn a PATCH into MINOR or MAJOR based on the subjective size of the diff; declare feature and compatibility effects truthfully.

R22. CI must execute in minimal-privilege context for PRs; only main-only release publication may request `contents: write`. Never run untrusted PR code with privileged tokens.

R23. Never bypass failed checks, mutate existing release tags, or invent release artifacts. Real release binaries must compile on supported architectures with checksum and provenance checks.

R24. The proposed GitHub rulesets are not active until a maintainer verifies administration-level activation. Do not state or infer enforcement from a committed JSON policy alone.

## Parallel worktree invariants

R25. Every concurrently active coding agent owns one issue, one branch and one local worktree (or its own remote clone). No shared writable directories.

R26. Never push another agent's branch, edit uncommitted work, force-remove a worktree or bypass the protected-PR process.

R27. Central `STATE.md`, central `WORKLOG.md`, shared interface schema and CI files have one designated integrator/owner at a time. Task evidence stays in distinct PRs or scoped logs.

R28. Git worktrees only isolate **local checkouts**; do not claim they synchronize cloud machines or replace issue-level task ownership.

R29. All branch creation/removal scripts must reject traversal, unexpected pre-existing paths and dirty directories. No automatic branch deletion or force push.
