---
name: lidb-rust-core
description: Rust domain contracts, bounded snapshots, typed telemetry and in-process collection.
---
# Rust core skill

**Use when:** Rust crates, entrypoints, telemetry/config contracts or collection change.

- Preserve existing core observation/capability/history and pure protocol-negotiation APIs.
- Separate domain contracts, source collection, sampling and presentation. No service/IPC layer is required for the baseline.
- Require explicit units, local time domain, availability, provenance and typed errors.
- Bound files, metric counts, payloads and history; report gaps and resets. Collection must not block terminal input indefinitely.
- Avoid unsafe code unless justified, isolated, documented and reviewed.
- Keep output schema evolution explicit and compatible where possible; pure protocol compatibility is independent from snapshot schema.
- Text/JSON and terminal modes work without privileges or external services.
- Run formatting, Clippy and meaningful tests with the lockfile; verify Rust 1.85 minimum independently of latest stable.
