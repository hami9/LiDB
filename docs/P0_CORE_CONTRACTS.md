# Preserved P0 core contracts

The foundation from [PR #9](https://github.com/hami9/LiDB/pull/9), originally [issue #7](https://github.com/hami9/LiDB/issues/7), is retained by the general Linux product. This document describes library behavior; it does not independently declare a phase complete.

## Retained implementation

- Rust 2021 core with duplicate-safe `CapabilityRegistry` and typed availability reasons.
- Provenance-aware `MetricObservation<T>` with source, unit, collector-local monotonic timestamp and `MetricState<T>` distinguishing valid zero from absence.
- Pure `lidb-protocol::negotiate()` major/minor version compatibility and a declared frame-size constant. There is no socket transport enforcing that limit.
- Unit tests for duplicate IDs, reason validation, metric states and unsupported protocol major versions.

The public product connects these contracts to an in-process Linux collector and `apps/lidash` presentation. The inactive daemon scaffold and specialized bootstrap capability placeholders were removed from the running baseline scope. Consult [architecture](ARCHITECTURE.md) and [README](../README.md) for current commands.

## Contract limits

Core observations do not automatically authenticate sources, bind timestamps to a boot identifier or make different clock domains comparable. The retained protocol crate supplies no serialization, peer credentials, remote access, daemon handshake or service lifecycle. The workspace's internal `0.0.0` version is not a published release.

## Integration gates

Verify formatting, Clippy and tests with the committed lockfile. Record Linux x86_64 and aarch64 native evidence separately. Preserve existing typed contracts while adding bounded snapshots and real procfs observations. Any incompatible API change requires an explicit compatibility note and ADR.

## Security

The pure core/protocol libraries forbid unsafe code and perform no source reads, privileged operations or network access. Collection belongs to the Linux adapter. No code should manipulate firewall, routing, namespace or kernel settings as part of this baseline.
