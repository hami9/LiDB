# Linux compatibility and source matrix

Support claims follow actual test evidence. Intended targets and successful fixture tests are not the same as a validated native platform.

| Environment | Baseline expectation | Validation status |
| --- | --- | --- |
| Linux x86_64 with readable procfs | Local CPU/memory/load/uptime/disk/interface observations | Local development target; record exact test commands in the worklog |
| Linux aarch64 | Same Rust/procfs baseline | Native CI validation pending until successful evidence is recorded |
| Restricted Linux container | Visible namespace/procfs subset | Capability and per-field fallback required; no host-wide visibility promise |
| Non-root SSH session | Same local collector and terminal UI | No privilege escalation or daemon requirement |
| Linux without readable PSI | Other sources continue; pressure marked unavailable | PSI fallback tested with fixtures |
| Non-Linux host | No live Linux collection claim | Unsupported live baseline; pure core contract code is separate |

## Sources

`/proc/stat`, `/proc/meminfo`, `/proc/loadavg`, `/proc/uptime`, `/proc/net/dev`, `/proc/diskstats` and optional `/proc/pressure/*` are read-only sources. Mount restrictions, kernel configuration and process namespaces affect coverage. Custom procfs directories are useful fixtures and must be identified as such.

The current baseline integrates no external runtime or SDK. There is no mandatory container runtime, eBPF program, hardware driver, cluster service or network endpoint. Optional future Linux networking/eBPF support must declare tested versions, permissions and fallback separately.

## Unsupported behavior

Expose the affected capability, reason and remaining useful sources. Do not infer source support from the kernel version alone, turn missing counters into zero, or claim a rate without valid counter deltas and elapsed time. Installation/build compatibility and actual live collection evidence should be tracked separately.
