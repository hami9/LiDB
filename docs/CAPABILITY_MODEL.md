# Capability and availability model

## Implemented foundation

`lidb-core` provides `Capability`, `CapabilityRegistry` and `CapabilityState`. Identifiers use bounded lowercase `org.lidb.*` names. Registration is duplicate-safe, and iteration is deterministic. Every unavailable capability requires an explanatory reason; available capabilities must not carry an unavailability reason.

The library describes capability state. Only a real collector can establish that its source is available. File existence alone does not prove readability, valid content or support for every metric.

## States

| State | Meaning |
| --- | --- |
| `available` | Source or value passed the collector's applicable checks |
| `unsupported` | Platform or source does not expose the feature |
| `disabled` | Intentionally inactive or not implemented |
| `permission_denied` | Current identity cannot read the source |
| `temporarily_unavailable` | Source or sample is not usable now |
| `stale` | Prior observation is too old to present as current |
| `error` | Parsing, validation or unexpected collection failure |

`MetricState<T>` distinguishes these states from a measured value, including a valid zero. Never label missing data as healthy. `doctor` presents source/capability status and actionable reasons without attempting privilege escalation or active probes.

## Public core domains

Host CPU, memory, load/uptime, network-interface counters, disk counters and PSI are independently inspectable. The absence of one source must not disable unrelated observations. Refresh and fixture-root configuration apply to the in-process collector, not to a service protocol.

## Future feature lifecycle

`planned → implemented → experimental → validated → stable → deprecated → removed`. Documentation does not promote a feature automatically. Optional network or eBPF features declare data sources, permissions, resource bounds, privacy, fallback and platform evidence before activation.

There is no current plugin manager, dynamic loader, vendor adapter or remote capability service. The pure `lidb-protocol` negotiation library is retained; it establishes no runtime transport or authorization. Any later extension boundary needs its own reviewed versioned contract.
