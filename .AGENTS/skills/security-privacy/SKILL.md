---
name: lidb-security-privacy
description: Least privilege, local authentication, sensitive telemetry and adapter isolation.
---
# Security and privacy skill

**Use when:** any collector reads sensitive data, adds IPC, invokes helpers, starts active probes, loads plugins or exports diagnostics.

- Maintain trust boundary between non-root TUI, local daemon, constrained helper and supervised external adapters.
- Review socket credentials, filesystem modes, timeouts, input validation and malicious/local user scenarios.
- Default to metadata-only with bounded retention; redact credentials, headers, environment data, command lines and model prompts.
- No self-authorized privilege escalation, system mutations, network services or remote telemetry.
- Document threats, attack preconditions, logging and rollback. Fail closed on privilege errors; optional signals degrade visibly.
- Test malformed events, oversized frames, denial of service, race conditions and plugin crashes.
- Require maintainer review for new privileged or network-exposed surfaces.
