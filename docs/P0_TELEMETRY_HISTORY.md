# P0 Bounded Telemetry History & Freshness

Task: [#19](https://github.com/hami9/LiDB/issues/19). Parent Rust Core foundation: [PR #9](https://github.com/hami9/LiDB/pull/9).

## Purpose

Provide pure, deterministic telemetry history primitives needed for future CLI/TUI adapters and collector implementations. This slice does **not** enable a daemon, GPU probe, procfs/sysfs reads, eBPF, IPC or live measurements.

## Contract

- `TelemetryHistory<T>` retains up to a caller-selected **1–4096 observations** and evicts the oldest when full. This bounds the sample count, **not total bytes**. Future producers must separately bound payload sizes and label cardinality.
- One history holds a single metric name/source established by its first observation. Mixed-series input is rejected without mutating the history.
- A monotonic timestamp older than the latest stored observation is rejected; equal timestamps may represent samples from the same collection batch.
- `evicted_count` counts capacity evictions only. It is **not** a dropped-kernel-event, queue overflow or lost-network-message count.
- `assess_freshness` compares sample time with a caller-provided monotonic time and maximum age. A timestamp in the future returns `ClockMovedBackwards`; stale age and available sample state are different dimensions.
- `latest_freshness` does **not** turn `MetricState::Unsupported` or `PermissionDenied` into a good value. The consumer must check both freshness **and** `MetricState<T>`.
- Observations must originate from **one node, same boot, same clock domain**. Name/source equality alone does not prove clock comparability. Future normalization should attach explicit node, boot and clock-domain identifiers.

## Security/privacy

No filesystem, privileged resources, collectors, clocks, external dependencies or network listeners are accessed. All inputs come from callers. Sensitive process commands, prompts, memory data and credentials must not enter metric names, source identifiers or metric payloads by default.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
```

GitHub Actions should validate Linux x86_64 and aarch64. The tests intentionally use only deterministic synthetic samples and do **not** constitute real-host or GPU hardware validation.

## Integration

After PR #9 merges, retarget the stacked task PR to `main` following standard review. No root Workspace, TUI, Codex adapter or central project phase-file changes are required.
