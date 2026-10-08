# Telemetry, evidence and diagnosis contract

## Telemetry identity

A normalized sample should include: `schema_version`, `event_id`, `timestamp_monotonic_ns`, optional `timestamp_wall`, `clock_source`, `node_id`, `source`, `capability_id`, `metric_name`, `value`, `unit`, `status`, `sample_interval`, `collection_cost`, and optional tags.

Identifiers are opaque, bounded and privacy-filtered. Avoid high-cardinality labels by default. Treat timestamps from different hosts as not directly comparable without documented clock sync and uncertainty.

```json
{
  "schema_version": "0.1",
  "node_id": "node-local",
  "source": "linux.psi",
  "capability_id": "org.lidb.linux.pressure",
  "metric_name": "memory.pressure.some.avg10",
  "value": 1.4,
  "unit": "percent",
  "status": "available",
  "clock_source": "monotonic",
  "timestamp_monotonic_ns": 123456789
}
```

This is an **illustrative contract**, not real measured telemetry.

## RCA proof hierarchy

- **Observed:** directly read counter, event or supported API value, with unit and timestamp.
- **Correlated:** multiple signals move together in a bounded time window.
- **Hypothesis:** likely fault domain with supporting and contradicting evidence.
- **Verified:** a repeatable controlled test or validated deterministic cause.

Present causality limits. Never turn an unsupported counter, lack of data, or clock skew into a root-cause claim.

## Collection controls

- Default: metadata only, no payloads, model prompts/completions, user keys or weights.
- Sampling and aggregation have explicit CPU/memory limits, event queue caps, lost-event counters and operator-visible warnings.
- Persist local history only if configured; provide retention, delete/export, and redaction controls.
- Metrics exporters and support bundles must not expose local or host secrets; handle process command-lines carefully.

## Stability and diagnostics

Expose an additive, versioned schema with compatibility tests; incompatible changes require major-version handling. Format JSON output with documented units and stable names. Normalized errors include typed reason, source, last successful timestamp, and actionable next check.
