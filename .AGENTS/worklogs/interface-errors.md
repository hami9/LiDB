# Interface Error Counters

Date: 2026-10-10. Owner: Codex. Branch: `agent/codex/interface-errors`.
Phase: one P2 networking slice; phase acceptance remains a maintainer decision.
Tracking: [issue #32](https://github.com/hami9/LiDB/issues/32).

## Task and acceptance

Operators need per-interface error and drop counts alongside throughput. Read
the existing `/proc/net/dev` source once per sample and expose
`network.<interface>.{rx,tx}.{errors,drops}` as exact cumulative `count` values.
Use the existing source, batch timestamp, name encoding and availability types.
Keep the core APIs, JSON schema 0.1 and existing byte/rate behavior unchanged.

Acceptance criteria:

- Preserve distinct RX/TX field positions, available zeros and full `u64` precision.
- Publish raw counters on the first sample and after decreases; do not invent
  rates, percentages, thresholds or a diagnosis.
- A malformed, missing or denied source retains the existing `network.state`
  failure without fabricating per-interface observations; other sources survive.
- Verify text/JSON output, source/unit/time provenance, encoded names and the
  maximum accepted batch size (794 observations with 64 interfaces and disks).
- Run pinned-toolchain format/lint/tests and native Linux CI; record actual
  evidence separately from fixtures and unsupported-platform checks.

## Source and boundaries

The [Linux interface statistics reference](https://docs.kernel.org/networking/statistics.html)
documents procfs aggregation: RX `drop` includes missed packets and is not
identical to netlink `rx_dropped`. Report the procfs column as observed, without
re-summing counters or inferring end-to-end packet loss. Error counts can include
events also represented by drop counters; they must not be summed into a total.
The selected procfs network namespace determines interface visibility.

This adds no reads, dependencies, privileges, active probes, sockets, process
attribution or packet contents. The existing 1 MiB/source, 64-device and
100-byte encoded-name limits apply. No system/network configuration changes.
Central state, central worklog and CI remain coordinator-owned and untouched.

## Validation and review

Implemented four additive metrics per interface. Existing byte/rate baselines
retain only transfer counters; error/drop counts are read from the same parsed
sample and do not persist across failures. Generic terminal presentation and
doctor counts consume the same observations. No core or protocol files changed.

Local environment: Windows x86_64, Rust 1.85.0 MSVC. Commands completed with
exit status 0:

- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --locked --all-targets`: 38 tests passed, including
  strict field mapping/overflow tests and unsupported-platform behavior.
- `cargo test --workspace --locked --doc`: passed, zero doctest cases.
- `python scripts/check_docs.py`: 140 local Markdown links passed.
- `git diff --check`: passed.

Linux-only source, permission, capacity and CLI fixture tests are not exercised
by Windows. Native x86_64/aarch64 PR CI results will be recorded in the PR.
No network namespace configuration or physical NIC error injection was done;
fixtures do not establish hardware error reporting or performance measurements.

Five-axis self-review (not independent approval):

- Linux/source: directional columns 2/3 and 10/11 retained exactly; procfs RX
  drop aggregation and namespace visibility documented with a kernel reference.
- Telemetry: first-sample/raw decrease behavior, exact integers, real zeros,
  source/time/unit and unchanged byte/rate semantics covered by focused tests.
- Security/privacy: no new reads or privileges; existing validation, failure
  isolation, label encoding and privacy boundaries retained.
- Reliability: 64-device and source-byte caps unchanged; a fixture asserts all
  794 observations fit, including maximum-length labels. Temporary parsing maps
  are bounded by the same device limit; overhead is not benchmarked.
- UX/compatibility: text/JSON tests cover the four names and metadata; existing
  generic terminal listing needs no new controls. Schema remains 0.1; only metric
  names are added. Consumers must tolerate additive metrics.

Handoff: maintainer review and merge through the normal CI gate. Central state
and worklog consolidation belongs to the coordinator after merge, as requested.
PR #29 also changes the device-limit path in the parser; this slice deliberately
retains main's current limits and does not implement that separate behavior.
