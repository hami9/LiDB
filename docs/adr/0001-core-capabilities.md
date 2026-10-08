# ADR 0001: Present Core Capabilities in the TUI

- Date: 2026-10-08
- Status: proposed; implemented in the Issue #17 slice, pending review
- Phase: P0

## Context

The independent TUI has fixture models and unprobed host telemetry, while `lidb-core` owns the capability registration contract. Copying that contract into another enum risks conflating disabled collectors, absent registration and observed hardware support.

## Decision

The TUI keeps its independent Cargo workspace and uses a local path dependency on `crates/lidb-core`. A small presentation adapter owns a clone of the GPU capability registration. It retains the core `CapabilityState` and reason directly; an absent registration is displayed as `NotProbed`.

The GPU panel presents capability registration and telemetry sample state separately. `Available` is not a live sample, `Disabled` is not a simulated fixture, and missing registration is not a hardware rejection. Bootstrap data comes from `bootstrap_registry`, with a constructor accepting another registry for tests and future integration. This is an immutable registration snapshot, not ongoing capability discovery.

## Consequences

The prototype now requires the accepted core workspace alongside it. Its `[workspace]` boundary and separate lockfile remain; it does not become a member of the root workspace. No protocol, serialization, daemon, collector, privileged operation or hardware dependency is added. The local core dependency has no external crate dependencies.

Telemetry-to-UI value conversion, provenance timestamps, units and freshness remain a subsequent slice. Fixtures keep their own explicit labels and can only be selected through the existing demo toggle. The adapter must not manufacture values while those collectors are absent.

## Alternatives

- Duplicating core availability enums was rejected because it weakens the single domain contract.
- Moving the whole prototype into the root workspace was deferred to preserve the owner's package boundary and keep this change independently reviewable.
- Rendering live metrics or implementing IPC was deferred because capability registration alone provides neither observations nor authenticated transport.
