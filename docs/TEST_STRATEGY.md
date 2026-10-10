# Core verification and release strategy

## Meaningful layers

1. **Domain contracts:** availability/reason validation, duplicate capabilities, timestamp/source identity, bounded history and pure protocol version mismatch.
2. **Collector parsing:** realistic procfs fixtures, malformed numbers/units, missing mandatory fields, truncated input, optional PSI and bounded large sources.
3. **Sampling:** consecutive counters, zero elapsed time, reset/regression and missing second samples. CPU activity and transfer rates must never be fabricated from one cumulative sample.
4. **CLI:** help/version, text/JSON snapshot, doctor reasons, invalid options and fixture-root behavior.
5. **Terminal:** explicit non-TTY behavior, keyboard quit, small dimensions, hostile identifiers and restored terminal modes on normal and error paths.
6. **Live Linux smoke:** real local procfs collection under a non-root identity; label the OS, kernel and architecture. A privileged development shell does not verify a non-root execution claim.

Fixtures are deterministic parsing/sampling evidence. They are not real-host, native architecture, kernel-probe or performance validation.

## Required commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
python3 scripts/check_docs.py
```

Verify the Rust 1.85 minimum independently of latest stable. CI should run native Linux x86_64 and aarch64; record successful runs before advertising both as tested. Existing Python automation tests remain applicable when those scripts change.

## Failure scenarios

Missing/restricted procfs; permission denied; malformed/oversized files; non-finite numbers; CPU/device counter resets; changing interface/device inventory; unavailable PSI; custom fixture roots; zero intervals; interrupted terminal sessions; narrow/non-interactive terminals; unsafe device-name control characters and malformed CLI arguments.

Tests must verify the operator-visible consequence, such as an explicit unavailable reason and continued unrelated collection, rather than only mirror implementation details.

## Release evidence

Public release requires reproducible native build/test reports, installation/uninstall checks, dependency/license review, verified checksums/provenance, compatibility documentation and measured resource use on labeled environments. Optional eBPF requires separate kernel verifier, privilege, detach and overhead evidence. No published binary release or production overhead claim currently exists.

## Test report

Record command, environment, exit code, observed result and known gaps in task logs and the central integrated worklog. Label `PASS`, `FAIL`, `NOT RUN` and `fixture` honestly. Keep output free of private host dumps and secrets.
