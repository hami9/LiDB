# Feature architecture, capability negotiation and extension policy

## Principle

Every feature is a **separately testable module**. It declares prerequisites, permissions, data semantics, operational cost and compatible API versions. Local baseline must continue working if an optional module fails or cannot load.

## Capability manifest (conceptual schema)

```yaml
id: org.lidb.gpu.nvidia
api_version: "1.0"
implementation: builtin               # builtin | supervised-adapter
status: experimental                   # stable | experimental | disabled
supported_arches: [x86_64, aarch64]
requires:
  os: linux
  libraries: [libnvidia-ml.so]         # detected at runtime, optional
  privileges: []                      # explicit separate helper permissions if any
provides: [gpu.utilization, gpu.power, gpu.memory.model]
cost_profile: low                      # low | moderate | high
privacy_class: host-metadata
fallback: unavailable-with-reason
default_enabled: false
```

The manifest format is illustrative and will be formalized with validated schemas in P0. Do **not** assume library presence implies metrics are supported: probe individual capabilities.

## Data availability is explicit

A telemetry field must carry one of: `available`, `unsupported`, `disabled`, `permission_denied`, `temporarily_unavailable`, `stale` or `error`. Unavailable values must not be substituted with 0. Missing data is not evidence that a subsystem is healthy.

## Extension boundary

- **First-party core:** Rust crates, compile-time features, well-reviewed code and strict domain APIs.
- **External community integrations (future):** separate supervised processes using a versioned local protocol. No untrusted native `dlopen` into the privileged daemon.
- Keep third-party adapters non-root by default, with opt-in explicit permissions and read-only interfaces.
- Reject unsupported protocol major versions. Negotiate supported minor capabilities; document deprecations.
- Declare OS and architecture support; third-party adapters must specify resource limits, timeouts, privacy policy and reproducibility.
- No plugin manager should silently download and execute arbitrary code. Operator consents to installation, source, privileges and updates.
- Disable one failing plugin without terminating all collectors; bound queues and memory allocations.

## Feature flag lifecycle

`planned → implemented behind flag → experimental → validated → stable → deprecated → removed`. There is no automatic promotion between states. Removal requires documented migration and a SemVer-compliant release.

## Definition of done for a module

Manifest + safety assessment + tests including failed dependencies + resource cost bound + CLI/TUI fallback + operator docs + changelog + compatible schema + human review. The agent must not merge feature code if any requirement is unaddressed.
