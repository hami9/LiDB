# Technology and dependency policy

## Current implementation boundaries

- **Rust 1.85 or newer, edition 2021:** core types, collection, CLI and terminal presentation.
- **`lidb-core`:** typed capabilities/observations, snapshots and bounded history.
- **`lidb-collect`:** safe, bounded procfs reads and parsers; no shell-command scraping.
- **`lidash`:** headless text/JSON commands and terminal dashboard.
- **`lidb-protocol`:** retained pure version negotiation; no active socket transport.

The baseline is in-process and requires no daemon, helper, database, browser, Python runtime, GPU library or external service. Keep dependencies limited to the implemented need; lock binary dependencies in `Cargo.lock` and inspect current APIs before use.

## Future instrumentation

Linux netlink/sysfs and optional C CO-RE eBPF can follow a concrete operator need. eBPF is never required for startup and needs explicit activation, kernel/permission checks, reviewed resource bounds and fallback. Introduce asynchronous runtimes, IPC or persistence only when the measured use case justifies them.

## Compatibility

Linux x86_64 and aarch64 are intended targets. Record native test evidence before asserting support. The core reads specific procfs interfaces and reports optional/missing fields; there is no unsupported blanket minimum-kernel promise. Rust minimum version must match the manifest and be validated independently of latest-stable testing.

## Supply chain and licensing

Project-owned code is MIT licensed. Review dependency license compatibility, provenance and security before introduction; retain notices. Release builds use the committed lockfile and documented toolchain. Dependency advisory review, SBOM and artifact provenance are release gates, not claims established by adding workflow files.

## Configuration

Core options cover sampling interval and source-root selection. No secret-bearing configuration is required. A future option that adds persistence, outbound access, privilege or additional sensitive collection needs explicit documentation and a reviewed default. Do not commit credentials, private host dumps or generated build artifacts.
