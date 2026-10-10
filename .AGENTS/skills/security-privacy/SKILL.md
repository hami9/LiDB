---
name: lidb-security-privacy
description: Read-only collection, hostile input, privacy and explicit future trust boundaries.
---
# Security and privacy skill

**Use when:** source parsing, diagnostics/output, dependencies, persistence, privilege or network access changes.

- Preserve the non-root, local, read-only baseline; collection and presentation are in-process.
- Bound source reads, identifiers, metric counts, polling and retained samples. Validate malformed/non-finite values and terminal-control text.
- Collect no credentials, packet bodies, process environments or command-line arguments. Requested outputs can contain activity metadata; minimize and document them.
- No self-authorized privilege escalation, host mutation, listener, active probe or outbound telemetry.
- Adding IPC/helpers, persistence or optional eBPF requires its own threat-model update, need, explicit scope and safe fallback.
- Test denial, missing sources, malicious identifiers, oversized files, resource limits and output escaping.
- Require independent review for new privilege or network-exposed boundaries; report actual evidence and unresolved limits.
