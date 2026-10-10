# Linux compatibility and source matrix

Support claims follow actual test evidence. Intended targets and successful fixture tests are not the same as a validated native platform.

| Environment | Baseline expectation | Validation status |
| --- | --- | --- |
| Linux x86_64 with readable procfs | Local CPU/memory/load/uptime/disk/interface observations | Local Debian 13.6 and native Ubuntu 24.04 CI passed; exact evidence below |
| Linux aarch64 | Same Rust/procfs baseline | Native Ubuntu 24.04 CI passed; exact evidence below |
| Restricted Linux container | Visible namespace/procfs subset | Capability and per-field fallback required; no host-wide visibility promise |
| Non-root SSH session | Same local collector and terminal UI | No privilege escalation or daemon requirement |
| Linux without readable PSI | Other sources continue; pressure marked unavailable | PSI fallback tested with fixtures |
| Non-Linux host | No live Linux collection claim | Unsupported live baseline; pure core contract code is separate |

## Native validation evidence

On 2026-10-09, [CI run 38004433311](https://github.com/hami9/LiDB/actions/runs/38004433311) passed on delivery commit `c1dd00f263142c924830ca0b98c32ac8e6fa5189`. Both native Ubuntu 24.04 x86_64 and aarch64 jobs ran Rust 1.85.0 formatting, locked Clippy with warnings denied, 49 Rust tests, the doctest command, unprivileged live snapshot/doctor JSON checks and PTY lifecycle checks. The Windows job validates worktree tooling only, not the Linux application.

[CodeQL run 38004433296](https://github.com/hami9/LiDB/actions/runs/38004433296) passed Actions and Rust analysis on the same commit; C analysis was skipped because no C source exists. [Documentation checks](https://github.com/hami9/LiDB/actions/runs/38004433345) also passed. The local environment and installation/package results are recorded in the [worklog](../.AGENTS/WORKLOG.md). Check [PR #25](https://github.com/hami9/LiDB/pull/25) for checks on later revisions.

These results validate the baseline on the named environments. They do not establish every kernel/distribution/container combination, production overhead, or public release readiness.

## Sources

`/proc/stat`, `/proc/meminfo`, `/proc/loadavg`, `/proc/uptime`, `/proc/net/dev`, `/proc/diskstats` and optional `/proc/pressure/*` are read-only sources. Mount restrictions, kernel configuration and process namespaces affect coverage. Custom procfs directories are useful fixtures and must be identified as such.

The current baseline integrates no external runtime or SDK. There is no mandatory container runtime, eBPF program, hardware driver, cluster service or network endpoint. Optional future Linux networking/eBPF support must declare tested versions, permissions and fallback separately.

## Unsupported behavior

Expose the affected capability, reason and remaining useful sources. Do not infer source support from the kernel version alone, turn missing counters into zero, or claim a rate without valid counter deltas and elapsed time. Installation/build compatibility and actual live collection evidence should be tracked separately.
