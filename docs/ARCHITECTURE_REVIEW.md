# Public Core architecture review

Final review baseline: main `842f877`, including merged PR #25 (`cc2fcff`, maintainer ARM flag/bootstrap corrections) and PR #34 (CPU/Memory tab). The earlier reading used `ec1d307`; after resumed-session discovery the branch was merged with current main without rewriting published history. Delivery: `agent/codex/agent-handoff`, issue #26 / PR #35. This is a source/dependency and regression review by the implementing coordinator, not an independent human approval or a guarantee that every defect is absent. Unimplemented networking, replay, eBPF and release phases were assessed as future boundaries, not tested features.

AG-01 follow-up on 2026-10-16: `agent/codex/collector-shutdown` is explicitly stacked on PR #35 head `5360ace`. It implements the lifecycle policy below, with independent read-only agent source review and deterministic blocked-callback/PTY evidence. Integration and native results are tracked in [issue #43 evidence](../.AGENTS/worklogs/issue-43.md), separately from the original guidance review.

## Dependency and responsibility boundaries

Actual workspace metadata reports an acyclic dependency graph:

```mermaid
flowchart LR
    CLI[lidash: config / text / JSON / terminal] --> Collector[lidb-collect: reads / parsing / rates]
    CLI --> Core[lidb-core: typed contracts / history]
    Collector --> Core
    Protocol[lidb-protocol: pure version negotiation]
```

Core and protocol depend on neither OS collection nor presentation. Collection owns procfs reads, parsing, failure isolation and counter baselines; the application owns sampling lifecycle, terminal state and rendering. One replaceable mailbox bounds queued snapshots. The protocol crate is a pure compatibility library, not a running transport/security boundary.

## Verified findings and changes

| Finding | Evidence / consequence | Resolution |
| --- | --- | --- |
| Wrong ARM64 final-component no-follow flag | `read.rs` used `0x20000` for both targets; Linux arm64 defines `O_NOFOLLOW` as `0x8000`, while `0x20000` is `O_LARGEFILE`. Static-link preinspection concealed the old open-time defect; current main has correct flags. | Already fixed by maintainer `ec50747` and included in main. Preserve that implementation; PR #27 separately owns the direct-open regression, with native before/fix CI evidence. Do not duplicate its work in this PR. |
| Stale product registry inside pure Core | Public `capability::bootstrap_registry` had no consumer and returned hard-coded disabled Linux areas with an obsolete P0 reason. | Already removed by the maintainer and integrated with PR #25. Generic Capability/Registry APIs remain; live doctor derives source status from observations. This PR documents that boundary rather than redoing the removal. |
| Architecture/implementation drift | The source table suggested packet/error/drop exports and broader PSI counters; the collector exports RX/TX bytes/rates and `some avg10`. ADR-007 suggested default Tokio/epoll; the baseline uses a standard thread. | Correct source claims and record [ADR-022](DECISIONS.md#adr-022--implemented-core-boundaries-and-abi-validation-2026-10-12). |
| Missing persistent Antigravity continuation scope | The existing native rule applied only by model decision to concurrent work; there was no ordered task/resume contract. | Add always-on startup guidance, shared skill discovery and the task/dependency/acceptance queue. No runtime behavior changes. |
| Unbounded join during interactive shutdown | A blocked-sampler regression waited for a 2s rescue and returned success, rather than timing out; an actual read could stall indefinitely. | AG-01 adds a 100ms joining grace, 5s batch watchdog, explicit errors, post-cancel publication suppression and permanent admission latch after abandonment; see ADR-023. |
| Racing late-batch failure classification | Independent review found the batch could time out between the UI's deadline check and its finished-worker check, producing a generic error. | Recheck collection outcome after observing completion and restore the terminal before explicit shutdown. |

The ABI was verified against Linux v6.18 primary sources: [arm64 fcntl.h](https://github.com/torvalds/linux/blob/v6.18/arch/arm64/include/uapi/asm/fcntl.h) and [generic fcntl.h](https://github.com/torvalds/linux/blob/v6.18/include/uapi/asm-generic/fcntl.h). Supported live targets remain Linux x86_64/aarch64; other targets report unsupported before collection. No new dependency, unsafe code, service or runtime network connection was added. The current CPU/Memory tab consumes the same validated Snapshot; its gauges are presentation and add no source reads. Its narrow/unavailable/tab tests were included in final validation.

## Five-axis assessment

- **Linux accounting and sources:** CPU guest ticks avoid double-counting; idle/iowait, MemAvailable, 512-byte disk sectors and per-device rates remain documented. No layered-device host total is invented. Kernel fields/PSI and namespace visibility are checked per source.
- **Telemetry integrity:** Export preserves exact unsigned counters, finite values, units, source and collector-local time; unavailable states retain reasons. Histories reject changed metric/source/unit and regressed timestamps. Batch time is the start of sequential reads, not an atomic host-wide observation. Clock-domain identity is caller-controlled and must be addressed before replay/import.
- **Privacy and permissions:** Read-only metadata collection, fixed sanitized I/O reasons, validated bounded identifiers and explicit live/fixture labels. Files are bounded at 1 MiB, devices at 64 per source, snapshots at 2048. No process arguments/environments/payloads or listener. Final-component no-follow does not eliminate intermediate-directory races; fixtures must remain trusted/static.
- **Reliability and resource use:** Source/device bounds yield at most 538 observations; names fit Core's metadata bounds and parsed quantities satisfy Snapshot insertion. The latest-slot queue bounds backlog while paused. AG-01 bounds interactive waiting with a 5s batch watchdog and 100ms shutdown grace, and permits at most one abandoned worker with no replacement. Blocked callbacks and captured-destructor stalls are tested; arbitrary kernel read cancellation and hard real-time process exit remain unclaimed. Production CPU/RSS/latency were not measured by this task.
- **Product and extensibility:** Text/JSON/doctor/TUI share one snapshot; doctor means collection availability, not host health. Routes, endpoints and interface metadata need structured records and source-specific policies before presentation. The continuation queue supplies those boundaries without reinstating vendor/AI/cluster scope.

## Remaining engineering limits

| Priority | Limit | Next slice |
| --- | --- | --- |
| Before claiming hard OS/read deadlines | AG-01 bounds interactive waiting but cannot cancel a kernel read; headless snapshot/doctor still have no deadline | Preserve ADR-023 limits; further source/process isolation needs its own justified slice |
| Before P2 structured network export | No typed route/endpoint/interface report or source-specific sysfs policy | AG-02 then AG-03/04/05 |
| Before persistence/replay or cross-collector correlation | No durable clock-domain identity or global multi-series retention/byte budget | AG-06 |
| Before stable release/performance/support commitments | Prerelease v0.2.0 is published; full notices/SBOM, measured overhead and wider distribution/container matrix remain incomplete | AG-07 |

See the [Antigravity queue](../.AGENTS/ANTIGRAVITY.md) for ownership, dependencies and acceptance criteria. These limits are not waived or represented as completed. Optional privileged/eBPF/active diagnostics stay separately authorized.

## Compatibility and verification

Snapshot JSON 0.1 and protocol negotiation are unchanged. The removed helper was pre-stable and unconsumed inside the workspace; any external caller should use `CapabilityRegistry::new()` and register its own actually checked capabilities.

Exact commands, environment, results and PR/check links are recorded in [WORKLOG.md](../.AGENTS/WORKLOG.md) and the [issue #26 task note](../.AGENTS/worklogs/issue-26.md). Source/dependency review is distinct from live local/native CI evidence. PR #27 owns the direct open regression; its source and native evidence were inspected, while final validation here covers current main and the installed-skill tooling.
