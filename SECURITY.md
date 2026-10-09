# Security policy

## Supported versions

LiDashBoard currently has a **pre-release Linux Core implementation**, with no published supported binary release or security patch schedule. When releases exist, this document will list supported versions and coordinated update policy.

## Reporting a vulnerability

**Do not disclose exploitable security issues in a public GitHub issue.** Use this repository's GitHub **Report a vulnerability** / private vulnerability reporting feature, if enabled. If unavailable, contact repository maintainers using a private channel published on the GitHub profile; do not upload secrets, private traces or exploit data publicly. No guaranteed SLA is offered at this bootstrap stage.

Useful report details: impacted commit/version, Linux/kernel/hardware environment, severity, exact boundary affected, sanitized reproduction, impact and mitigation. Do not probe systems you do not own or lack authorization to test.

## Areas of special concern

Bounded procfs parsing, input validation, availability and counter integrity, terminal control handling, local privacy, supply-chain dependencies and packaged binaries. The current Core is a local read-only process; it installs no daemon, opens no network listener and collects no packet contents or process arguments. Future privileged collectors, persistence or external integrations need a separate design and review before implementation.

Public fixes should credit reporters with consent, include regression tests, and disclose affected versions when relevant.
