# Preserved bounded telemetry history and freshness

The pure history implementation from [issue #19](https://github.com/hami9/LiDB/issues/19) / [PR #20](https://github.com/hami9/LiDB/pull/20) builds on [PR #9](https://github.com/hami9/LiDB/pull/9) and remains part of `lidb-core`.

## Contract

- `TelemetryHistory<T>` retains a caller-selected **1–4096 observations**, evicting the oldest when full. This bounds sample count, **not total bytes**; producers separately bound payloads and identifiers.
- The first observation establishes one metric name/source. Mixed series and regressing timestamps are rejected without mutation; equal timestamps are allowed.
- `evicted_count` counts capacity evictions, not lost kernel events, collection errors or dropped transport messages.
- `assess_freshness` compares sample time to caller-supplied local monotonic time and maximum age. A future sample returns `ClockMovedBackwards`.
- Consumers check both freshness and `MetricState<T>`; a current unsupported value remains unsupported.
- One history must use the same local collector, boot and clock domain. Metric/source equality alone does not prove time comparability.

## Integration and privacy

The library accesses no files, clocks, collectors, privileged resources or network endpoints. It accepts caller-provided observations and does not persist them. Live sampling belongs to the in-process Linux collector; the library cannot establish that fixture data is an actual host measurement.

Metric names, sources and payloads must not contain credentials, packet contents, process environments or command lines. CLI/TUI consumers must preserve explicit unavailability and separately enforce retention/payload bounds.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
```

History tests use deterministic synthetic samples. They establish pure contract behavior, not native architecture, real-host collection or performance support. Current integrated test evidence belongs in [.AGENTS/WORKLOG.md](../.AGENTS/WORKLOG.md).
