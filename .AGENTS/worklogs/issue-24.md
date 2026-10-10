# Issue 24 — Read-only public Linux host collector

Date: 2026-10-09. Owner: `agent/codex/public-collector`. Worktree:
`.worktrees/codex/public-collector`. Scope: `crates/lidb-collect/**`; central
state, shared schemas, lockfile, CLI, and top-level docs remain coordinator-owned.

## Acceptance and implementation

- Implement `LinuxCollector::new(proc_root)` and `sample() -> Snapshot` against
  the shared core schema. `with_fixture_mode()` supports explicit CLI fixture
  provenance, including an explicitly supplied `/proc` root.
- Collect actual aggregate CPU counters and busy deltas, memory/swap using
  `MemAvailable`, load averages, uptime, interface and disk byte counters/rates,
  and optional PSI some avg10.
- Preserve exact cumulative `u64` quantities and explicit state on missing,
  denied, malformed, overflowed, initial, nonadvancing, and reset observations.
  Independent source failure never invalidates unrelated observations.
- Enforce 1 MiB per source, 64 interfaces and 64 disks; entity overflow is a
  visible source failure. Encode device names reversibly into ASCII-safe metric
  labels, with encoded length at most 100 bytes and total names at most 128.
- Use a constructor-local monotonic clock with common batch timestamps and a
  measured collection duration. CPU and transfer rates require an advancing
  clock and reset their baseline after source failures/counter regression.
- Require no root, outbound network operation, process payload, or active probe.
  Reject terminal controls, invalid UTF-8, non-regular files, FIFO sources, final
  symlinks, and static fixture directory symlinks. Scope Linux open flags to
  x86_64/aarch64; other targets report unsupported without filesystem reads.

Checkpoint: `f486bb7` — `feat(collect): add bounded read-only Linux host telemetry`.
Follow-up records name compatibility, clock regression, portability, malformed
memory syntax, denied-file fixtures, and the operator-facing crate README.

## Actual validation

Environment: Rust/Cargo 1.85.0; Linux 6.18.44 x86_64; UID 1000. The actual
unprivileged mode-000 fixture was denied; no bypass message was emitted.

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Exit 0 |
| `cargo clippy -p lidb-collect --all-targets -- -D warnings` | Exit 0 after correcting the test permission literal to octal |
| `cargo test -p lidb-collect -- --nocapture` | Exit 0; 14 passed, 0 failed, 0 ignored; 0 doctests |
| `git diff --check` | Exit 0 |

The live smoke test reads real `/proc` in this Linux environment and observes an
available CPU delta on a second sample. Fixtures prove parser, units, source
isolation, initial/reset/gap rates, CPU guest/iowait accounting, byte overflow,
negative/nonfinite input, duplicate PSI fields/entities, Unicode/punctuation/%
encoding, maximum metric-name size, entity/byte limits, final/directory symlinks,
FIFO rejection, and actual permission denial. The `mkfifo` command creates only
a disposable test fixture.

An independent read-only collector reviewer identified CPU percentage rounding,
PSI duplicate-field validation, non-Linux warnings, and legitimate device-name
compatibility risks. Those have regression coverage or platform guards now.
Independent review remains evidence, not maintainer approval.

## Five review axes and limitations

1. Runtime/correctness: first-eight CPU accounting avoids double counting guest
   ticks; idle/iowait are non-busy. Counter decreases, including Linux's possible
   iowait decreases, produce an error and a new baseline. No atomic cross-source
   snapshot is claimed.
2. Networking: only local aggregate net/dev counters are read; no addresses,
   packets, peers, payloads, or connectivity probes. Punctuation names retain
   unrelated interfaces and encode without collisions.
3. GPU/AI/fabric: absent from this baseline. No GPU or hardware-specific support
   is claimed or fabricated.
4. Product/UX: stable units, fixed sanitized errors, source provenance, fixture
   labeling, and initial-rate states are exposed for the coordinator-owned CLI.
5. Security/privacy: no unsafe Rust or privilege boundary; byte/entity limits
   bound data volume. Fixture directories must be trusted/static because path
   checks do not prevent intermediate-directory replacement races. Synchronous
   regular-file reads have no wall-clock deadline on an unresponsive filesystem.

Not run: aarch64 runtime, non-Linux compilation (only x86_64 Linux Rust target
installed), measured overhead benchmark, GPU or specialized hardware tests.
Disk partitions and parent devices must not be summed together. Rates cannot
identify a hotplug that reuses a name while retaining increasing counters.

Coordinator next step: integrate commits, run complete workspace/CLI checks,
consolidate public scope and collector documentation, and update central state
and worklog. The worker did not push, open a PR, merge, or modify Cargo.lock in
its commit.
