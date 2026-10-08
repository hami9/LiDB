# Mandatory engineering and agent rules

## Correctness

R01. Never invent metric values, benchmark outcomes, GPU capabilities, NCCL results or completed tasks.

R02. Do not model conntrack, nftables, route lookup and namespaces as a single unconditional packet-processing sequence; distinguish ingress, egress, forwarding, hooks and priorities.

R03. Treat Spark's CPU↔GPU NVLink-C2C as intra-node; DGX Spark inter-node traffic uses ConnectX Ethernet/RoCE. Separate real NVLink/NVSwitch fabrics on applicable hardware.

R04. Separate source data, derived data, hypotheses and verified diagnosis with provenance and uncertainty. A missing counter does not mean zero.

## Security, privacy and availability

R05. Non-root CLI/TUI and read-only baseline. Use a privileged helper only for an explicitly authorized and narrowly audited operation.

R06. Do not execute arbitrary remote code, run invasive diagnostics, change firewall/routes/network parameters, install kernel modules or alter AI jobs without explicit operator permission.

R07. Never capture packet payloads, decrypted TLS, prompts, completions, model weights or secrets by default. Scrub support bundles and logging.

R08. Validate untrusted kernel events, RPC messages and plugin input. Bound memory, queues, timeouts and execution overhead.

R09. Do not add a remote network listener, telemetry export, update agent or dynamic native plugin loader without threat-model and approval.

R10. Local discovery cannot assume NVIDIA/CUDA, eBPF, container runtimes, permissions or a cluster.

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
