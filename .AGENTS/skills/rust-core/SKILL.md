---
name: lidb-rust-core
description: Rust architecture, async orchestration, typed telemetry and IPC.
---
# Rust core skill

**Use when:** creating Rust crates, binary entrypoints, domain models, config, local IPC or async collectors.

- Separate domain DTO/schema, collector traits, scheduler, storage and presentation; keep devices optional.
- Require explicit units, time bases, missing-data enum, provenance and typed errors.
- Use bounded channels, cancellation, backpressure and deterministic shutdown; never block UI on a collector.
- Avoid `unsafe` unless justified, isolated, documented and reviewed. Validate all C/FFI boundaries.
- Keep schema evolution additive where possible; tests for unknown fields/version mismatch.
- CLI and JSON headless outputs must work without a running GPU or helper.
- Verify with `cargo fmt`, `cargo clippy`, `cargo test` once a workspace exists; report actual execution.
- For native compilation decisions, record distro/toolchain/kernel versions in an ADR.
