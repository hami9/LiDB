# Telemetry and evidence contract

## Implemented observation identity

`MetricObservation<T>` carries a bounded metric name and source, a typed `Unit`, collector-local `monotonic_ns` and `MetricState<T>`. Available zero is a valid measurement. Unsupported, disabled, denied, temporary, stale and error states carry reasons and do not expose a fabricated value.

Snapshot schema `0.1` carries `clock_source: "collector_monotonic"`, batch `monotonic_ns`, `collection_duration_ns`, `mode: "live" | "fixture"`, `metrics` and `dropped_metrics`. Each metric carries `name`, `source`, `unit`, `monotonic_ns` and `state`. Snapshot output preserves source, units and availability. JSON is a pre-stable, documented machine interface; contract changes need explicit compatibility notes. In JSON, `state` is `{ "status": "available", "value": <number> }` for an observed value. For an unavailable state, `value` is its reason string; consumers must inspect `status` before interpreting it. Exact integer counters serialize as unsigned integers; derived values are finite numbers. There is no public deserializer accepting unchecked snapshots. Identifiers are bounded and contain no command lines, credentials or private payloads. Monotonic timestamps share a domain only within the same collector/boot context; they are not Unix wall-clock time.

## Derived values

CPU utilization and device rates use two compatible observations and elapsed monotonic time. First samples, time regression, zero elapsed time, reset counters and missing input create visible gaps. Cumulative interface/disk counts remain labeled cumulative. PSI averages and load averages have their own kernel semantics; neither is automatically a CPU-utilization percentage.

A capability being available means its source passed the applicable checks. It does not imply the subsystem is healthy. A stale observation's freshness and its original availability are independent dimensions.

## Evidence hierarchy

- **Observed:** a supported source value with units, timestamp and provenance.
- **Derived:** a documented calculation from compatible observed values.
- **Correlated:** signals moving together in a stated local time window.
- **Hypothesis:** a possible explanation with supporting and contrary evidence.
- **Verified:** a repeatable controlled test or validated deterministic cause.

The current baseline displays observations; it does not claim an automated root-cause engine. Never turn missing data or one threshold crossing into a verified cause.

## Collection and retention

Collection is metadata-only and local. No payloads, secrets, process environments or command lines are collected. Source files are capped at 1 MiB and snapshots at 2048 observations. Capacity omissions are reported as `dropped_metrics`; this is not a lost-kernel-event count. Source input sizes and sample/device counts are bounded, and polling intervals have an allowed range. History primitives bound sample count, not arbitrary bytes; see [history contract](P0_TELEMETRY_HISTORY.md). No persistence or outbound export is enabled by ordinary startup.

Operator-requested text/JSON can contain host/device activity metadata. Sharing it is an operator action. Future support bundles, stored history or exporters need explicit privacy/retention review and documented controls.
