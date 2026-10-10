# Core security architecture and threat model

## Posture and boundaries

LiDB runs locally with the operator's existing identity. The baseline reads Linux procfs and presents text, JSON or a terminal view through in-process libraries. It opens no daemon socket, remote listener or external endpoint, requests no privileges and changes no host settings.

Trust boundaries are the source text entering the collector, parsed observations entering core contracts, and text entering terminal/JSON output. Kernel-derived files and custom fixture roots may be incomplete, malformed, large or intentionally hostile. Capability state represents usable evidence, not trust in arbitrary contents.

## Risks and controls

- Bound file reads, source/device counts, retained samples and polling frequency. A denied, absent or malformed source degrades its own observations visibly.
- Reject invalid numeric fields, non-finite values and unsafe counter arithmetic. Preserve reset and first-sample gaps rather than inventing rates.
- Sanitize terminal control characters in untrusted identifiers; JSON output must escape strings correctly.
- Collect no process environments, command-line arguments, packet bodies, memory contents or credentials. Host counters can still describe activity; operators control whether to share output.
- Do not persist history, upload telemetry, inspect secrets, run shell probes or enable services as part of ordinary startup.
- Restore terminal state on quit and handled errors. Collection failures must not trigger uncontrolled retries or privilege prompts.
- Review locked dependencies and build tooling; never run an unreviewed install script.

## Future features

Network probes, eBPF, stored/exported history, IPC, helpers and listeners each require a separate scoped threat-model update and concrete operator need. Passive baseline operation must remain useful without them. No arbitrary command executor or automatic remediation belongs to this product.

## Incident handling

Use [SECURITY.md](../SECURITY.md) for vulnerability reporting. Public issues should include minimal, redacted reproduction data. Review [test strategy](TEST_STRATEGY.md) for hostile-input, permission and resource tests; don't claim controls are verified until the applicable tests have run.
