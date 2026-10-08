---
name: lidb-testing-quality
description: Reproducible CI, negative-path tests, hardware validation and release gates.
---
# Testing and quality skill

**Use when:** writing tests, CI workflows, benchmarks, QA reports or accepting a phase.

- Unit-test models, units, parser limits and typed errors; use deterministic injected clocks.
- Integration-test kernel behavior only in disposable isolated environments with explicit permission.
- Collect adversarial cases: missing GPU, low privileges, stale/contradictory data, throttling, dropped events, adapter crashes.
- Separate emulation, simulation, CI virtualization and verified hardware measurements in all reports.
- Define measurement workload, kernel, driver, node count, tool version and confidence interval for benchmarks.
- No test-result claims without exact commands, exit status and relevant logs/CI artifacts.
- Respect quality gates and stop when untestable critical requirements block a phase.
