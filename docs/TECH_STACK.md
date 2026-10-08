# Tech stack and dependency policy

## Implementation boundaries

- **Rust:** domain models, application state, CLI/TUI, local IPC, storage, integration adapters, correlation engine, error handling.
- **C:** Linux CO-RE eBPF programs and only necessary stable native shims. Avoid C application business logic where Rust suffices.
- **Linux kernel interfaces:** procfs, sysfs, netlink/rtnetlink, netfilter/nftables metadata, socket diagnostics, cgroups, namespaces, PSI, ethtool/devlink where supported.
- **Terminal:** Ratatui + Crossterm. Prefer a functional headless CLI output path for agents and CI.
- **eBPF:** libbpf-compatible CO-RE tooling; optional collector and explicit kernel capability probing.
- **Asynchronous IO:** Tokio/epoll baseline where appropriate; do not require io_uring initially. Investigate io_uring only after measurements identify a meaningful bottleneck.
- **NVIDIA:** optional NVML/DCGM integrations (where applicable), published NVIDIA tools and exported runtime metrics; no bundled proprietary GPU runtime or undisclosed vendor redistribution.
- **Model serving:** integrate read-only, documented metrics endpoints, with secrets sourced from local approved config and never logged.

## Compatibility rules

Support Linux x86_64 and aarch64 using declared minimum kernel and distro baselines chosen during P0 via tests. Verify toolchain and dependency versions when implementation begins; do not pin hypothetical future crate versions in docs. Prefer stable language features and established dependencies. A module without validation on target hardware must remain experimental or unsupported.

## Source, supply chain and licensing

License project-owned source under MIT. Assess license compatibility of all dependencies, probes and vendor SDKs before adding them. Commit lockfiles for binaries and versioned application crates; record reproducible build instructions; CI should scan dependency advisories and produce an SBOM for release. Never commit GPU SDK binaries, model weights, credentials or test dumps containing personal data.

## Process isolation

`lidash` (no special privileges) communicates with `lidashd` over a credential-checked local Unix domain socket; `lidash-helper` exists only if a specific operation cannot be performed without additional Linux capabilities. Make all privilege boundaries explicit in reviewable code. The UI never parses untrusted kernel events as control instructions.

## Configuration

Human-readable, schema-validated config with safe defaults and environment-based overrides. Operator secrets must use strict permissions or platform secret providers rather than plaintext checked-in files. Changes that could cause data collection or outbound access should be opt-in and auditable.
