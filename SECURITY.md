# Security policy

## Supported versions

LiDashBoard is currently in **pre-release specification stage**, with no supported binary release or security patch schedule. When releases exist, this document will list supported versions and coordinated update policy.

## Reporting a vulnerability

**Do not disclose exploitable security issues in a public GitHub issue.** Use this repository's GitHub **Report a vulnerability** / private vulnerability reporting feature, if enabled. If unavailable, contact repository maintainers using a private channel published on the GitHub profile; do not upload secrets, private traces or exploit data publicly. No guaranteed SLA is offered at this bootstrap stage.

Useful report details: impacted commit/version, Linux/kernel/hardware environment, severity, exact boundary affected, sanitized reproduction, impact and mitigation. Do not probe systems you do not own or lack authorization to test.

## Areas of special concern

Privilege separation; eBPF attach/loader behavior; local Unix socket authentication; plugin/process isolation; untrusted kernel/provider data; parsing/fuzzing; secrets in support bundles; remote telemetry/cluster authentication (once introduced); supply-chain and packaged binaries.

Public fixes should credit reporters with consent, include regression tests, and disclose affected versions when relevant.
