# Verification and release test strategy

## Testing layers

1. **Unit:** typed parsing, counters and units, availability states, RCA scoring, schema versioning, redaction.
2. **Contract:** JSON stability, capability negotiation, adapter timeouts, malformed input, plugin crash, version mismatch.
3. **Integration:** Linux host collection, netlink network namespace fixtures, cgroup v2/PSI, route/interface state; permissions-denied paths.
4. **eBPF:** compile and verifier tests on declared kernel baselines, unsupported-kernel fallback, attach/detach cleanup, dropped-event reporting.
5. **TUI/CLI:** deterministic terminal snapshots at narrow widths, keyboard interactions, non-ANSI output and accessibility/readability.
6. **AI:** simulated GPU, model-serving and NCCL fixtures plus explicitly labeled real-hardware validation.
7. **Security:** threat-model checks, fuzzing, resource caps, secret-scrubbing, local socket impersonation and privilege isolation tests.
8. **Performance:** instrument collector overhead and latency impact in controlled, repeatable runs.

## Hardware validation is not optional for hardware claims

Use explicit matrix labels: `simulated`, `CI virtual machine`, `tested on x86_64 host`, `tested on aarch64 host`, `tested on DGX Spark`, `tested on multi-node Spark`, `tested on NVLink GPU platform`. No unit test or fixture constitutes a measured DGX result.

## Representative failure scenarios

Missing NVIDIA libraries, non-root permissions, procfs access restriction, unsupported Linux kernel, stale samples, high cardinality, repeated adapter crashes, broken clocks between nodes, absent NCCL logs, counters reset or wrap, multiple NICs, conflicting routes, low terminal width, disconnected local daemon and disk full.

## CI required progression

P0: docs and format checks, Rust formatting/clippy/test when workspace exists, dependency and license checks. P1–4: Linux integration matrix + headless snapshots. P5–8: optional hardware runner reports. P9–10: release signing, SBOM, compatibility tests, reproducible benchmark methodology.

## Test report

For every changed module record commands, exit code, actual environment, meaningful test failures, measurements with workload/hardware details, untested paths and residual risks in [.AGENTS/WORKLOG.md](../.AGENTS/WORKLOG.md). Never say "all tests passed" when they were not run.
