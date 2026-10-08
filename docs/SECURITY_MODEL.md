# Security architecture and threat model

## Security posture

LiDB is an open-source diagnostic product running on sensitive hosts. Treat both **kernel event streams and runtime adapters as untrusted input**. The threat model covers: privilege misuse, kernel/verifier instability, malicious or faulty plugins, secret leakage, excessive sampling, socket impersonation, supply-chain compromise and unsafe operator automation.

## Required trust boundaries

1. `lidash` TUI runs unprivileged.
2. `lidashd` gathers baseline metrics with minimal permissions over a local Unix socket. Validate peer credentials and restrict socket filesystem mode; never expose an unauthenticated TCP listener.
3. `lidash-helper` is opt-in for restricted kernel/eBPF operations. Linux capabilities depend on kernel and operation (e.g. CAP_BPF, CAP_PERFMON, CAP_NET_ADMIN or other specific requirements); never claim one fixed set universally.
4. Third-party adapters run as supervised non-root processes with explicit capability allowlists, deadlines, data size and resource bounds.
5. Inter-node trust and authentication are **not** part of P0. Future remote telemetry requires mutual authentication, authorization, encrypted transport, key rotation, replay protection, and a threat-model review before enabling.

## Privacy and data handling

No packet body, TLS secret, HTTP header, DNS query, model prompt/completion, GPU memory content, SSH token or environment-variable secret collection by default. Filter command-line and process metadata before display/export. Make advanced capture exceptional, scoped, expiring and visibly active. No automatic outbound telemetry, update checker or cloud account.

## Unsafe actions that agents must not perform

Never execute arbitrary commands as root, disable a firewall, reconfigure NICs/routes, unload in-use kernel modules, alter NCCL/runtime parameters on production hosts, or send a workload without an operator-approved plan. Unauthenticated local network probes are not assumed harmless; scope and rate-limit any active diagnostics.

## Engineering controls

- Memory-safe Rust for parsing and state handling; C limited to documented kernel boundaries with targeted sanitizers/tests.
- Fuzz untrusted parsers, bounds-check event records and strings, and reject unknown protocol versions safely.
- No secrets in logs, CI output, snapshots or test fixtures. Include SAST, dependency vulnerability checks, license review, reproducible release builds and SBOM.
- Fail closed for privileged operations; fail visibly and degrade for missing optional metric sources.
- Introduce security review for new privilege, network listener, plugin, export, or cross-node feature.

## Incident handling

Security disclosures follow [../SECURITY.md](../SECURITY.md). Fixes should include regression tests, affected-version analysis, release notes and minimized public disclosure while coordinating a patch.
