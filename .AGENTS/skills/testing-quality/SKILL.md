---
name: lidb-testing-quality
description: Reproducible source/error tests, native-platform evidence and release gates.
---
# Testing and quality skill

**Use when:** writing tests, CI, benchmarks, QA reports or accepting a phase.

- Test observable contract behavior: typed failures, source units, parsing limits, counter resets/time deltas and bounded retention.
- Use deterministic fixtures/injected clocks for parsing; label them separately from live Linux and native architecture runs.
- Exercise missing/denied/malformed sources, unavailable PSI, hostile identifiers, zero intervals and interrupted/narrow/non-TTY terminals.
- Use disposable isolated environments for optional privileged kernel/network tests with operator authorization.
- State OS/kernel/architecture, toolchain, workload and measured resource limits for performance reports.
- Record exact commands, exit status and gaps. Never imply a minimum toolchain or aarch64 target passed because latest stable x86_64 did.
- Respect quality gates; unavailable critical tests keep phase acceptance pending.
