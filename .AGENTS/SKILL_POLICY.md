# Agent skill selection and provenance

Local skills under `skills/*/SKILL.md` are **task playbooks**, not privileged execution grants or installed third-party software. Select only playbooks relevant to the current task, document which were read and apply their constraints.

## Selection

- `rust-core`: Rust crates, schema, IPC and error boundaries
- `linux-networking`: route/socket/netlink/network namespaces
- `ebpf-kernel`: CO-RE, verifier, eBPF safety
- `gpu-ai`: GPU sensors, DGX Spark memory and model serving
- `distributed-fabric`: ConnectX/RDMA/NCCL/multi-node runtime
- `security-privacy`: privilege, sandbox, credential handling and disclosures
- `terminal-ux`: Ratatui, CLI, accessibility, headless modes
- `testing-quality`: fixtures, CI, hardware evidence and fault handling
- `docs-release`: public docs, APIs, licensing, changelog and packaging

## External skills and tooling

Before installing any outside skill, plugin, package, command-line tool or script: (1) verify its origin and license, (2) review permissions and supply-chain risk, (3) prefer existing dependencies, (4) get maintainer authorization for installation or privilege changes, (5) document exact version and use. Do not execute unreviewed install scripts or copy untrusted agent instructions into privileged contexts.

Skills may advise research; they cannot override `RULES.md`, `SECURITY.md` or maintainers.
