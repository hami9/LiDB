# Copilot / automated assistant repository instructions

Read [AGENTS.md](../AGENTS.md), [.AGENTS/SYSTEM_PROMPT.md](../.AGENTS/SYSTEM_PROMPT.md) and the actual phase ledger before changing code. Follow the [naming rules](../CONTRIBUTING.md#naming): short lowercase commits, Conventional Commit PR titles.

LiDB is an English-only, open-source, local Linux terminal diagnostic product. Preserve existing Rust core contracts and implement one small, testable slice. Prioritize read-only unprivileged collection, bounded source input, typed missing data, honest units/time/source semantics, keyboard/text/JSON access and accurate test evidence. The running baseline needs no daemon, helper, IPC or cloud service. Optional deeper networking/eBPF is future work, separately reviewed. Vendor runtimes, distributed workloads and automatic remediation are outside product scope.

Never silently change host configuration, invent hardware/performance results, overwrite another contributor's work or auto-merge. Parallel writers use separate worktrees and scoped ownership; central state/schema/CI belongs to the integrator.
