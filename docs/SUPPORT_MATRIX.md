# Platform and Kernel Compatibility Matrix

This document defines the supported runtime environments, hardware architectures, operating system versions, and container compatibility boundaries for LiDB and LiDashBoard (`lidash`).

## Hardware architecture tiers

| Architecture | Tier | Status | Binaries & Verification |
| --- | --- | --- | --- |
| Linux `x86_64` (`amd64`) | **Tier 1** | Primary release target | Precompiled binary (`lidb-x86_64`), automated CI gate, native tests |
| Linux `aarch64` (`arm64`) | **Tier 1** | Primary release target | Precompiled binary (`lidb-aarch64`), automated CI gate, cross-build tests |
| Windows `x86_64` | **Tier 3** | Diagnostic / CLI only | Compiles cleanly; returns typed `SourceAvailability::unsupported` |
| macOS `x86_64` / `arm64` | **Tier 3** | Diagnostic / CLI only | Compiles cleanly; returns typed `SourceAvailability::unsupported` |

Tier 1 targets receive precompiled standalone binary builds, automated cryptographic checksums, and continuous integration gating on every commit and pull request.

## Linux kernel compatibility matrix

LiDB reads Linux kernel procfs, sysfs, and optional tracepoints directly without third-party kernel modules. Different diagnostic features activate depending on the host kernel version:

| Feature Subsystem | Minimum Kernel | Source Path | Fallback State when Unavailable |
| --- | --- | --- | --- |
| CPU utilization & ticks | Linux 2.6+ | `/proc/stat` | `SourceAvailability::error` |
| Memory & Swap counters | Linux 2.6+ | `/proc/meminfo` | `SourceAvailability::unsupported` for missing swap |
| System Load averages | Linux 2.6+ | `/proc/loadavg` | `SourceAvailability::error` |
| System Uptime | Linux 2.6+ | `/proc/uptime` | `SourceAvailability::error` |
| Disk I/O counters & rates | Linux 2.6+ | `/proc/diskstats` | `SourceAvailability::error` |
| Network interface traffic | Linux 2.6+ | `/proc/net/dev` | `SourceAvailability::error` |
| Network interface metadata | Linux 2.6.32+ | `/sys/class/net` | `SourceAvailability::unsupported` |
| Network namespace identity | Linux 2.6.32+ | `/proc/self/ns/net` | `NetworkNamespace::Host` fallback |
| IPv4 & IPv6 routing tables | Linux 2.6+ | `/proc/net/route`, `/proc/net/ipv6_route` | `SourceAvailability::unsupported` |
| TCP & UDP socket endpoints | Linux 2.6+ | `/proc/net/{tcp,tcp6,udp,udp6}` | `SourceAvailability::unsupported` |
| Pressure Stall Info (PSI) | Linux 4.20+ | `/proc/pressure/{cpu,memory,io}` | `SourceAvailability::unsupported` |
| eBPF TCP lifecycle events | Linux 4.16+ | `sock:inet_sock_set_state` via BTF | `SourceAvailability::unsupported` or `PermissionDenied` |

When an older kernel does not provide a feature (such as PSI on Linux 3.10, or BTF eBPF on Linux 4.14), `lidash` degrades gracefully:
- It reports the exact source status as `unsupported` with a clear explanation.
- It **never** panics, crashes, or fabricates synthetic zero values.
- All remaining available metrics and features continue to function normally.

## Container and virtualization environments

| Environment | Supported | Operational Notes |
| --- | --- | --- |
| **Bare-metal Linux** | YES | Full visibility into host CPU, memory, disk, network, and eBPF. |
| **KVM / QEMU / Proxmox VM** | YES | Full visibility within the virtual machine boundary. |
| **Docker / Podman (root)** | YES | Telemetry scoped to container namespace; eBPF requires `CAP_BPF` or `--privileged`. |
| **Rootless Containers** | YES | Full non-root procfs and sysfs diagnostics; eBPF degrades cleanly to `PermissionDenied`. |
| **Kubernetes Pods** | YES | Network interfaces, routes, and sockets accurately reflect the Pod network namespace. |
| **WSL2 (Windows Subsystem for Linux)** | YES | Uses standard Linux 5.15+ kernel; all procfs and network diagnostics fully supported. |

## Resource and performance guarantees

`lidash` is engineered for zero-impact production monitoring on constrained and heavily loaded systems:

- **CPU Usage:** Consumes < 0.5% CPU core utilization during continuous 1-second terminal refresh.
- **Memory Footprint:** Resident Set Size (RSS) strictly remains under 15 MB.
- **History Retention:** The interactive terminal history ring buffer is capped at `MAX_VIEW_HISTORY = 120` updates (~700 KB RAM), guaranteeing no memory leaks over indefinite execution.
- **Disk I/O:** Exactly 0 disk writes occur during live streaming or one-shot commands, unless an operator explicitly specifies `--output PATH`.
- **Network I/O:** Exactly 0 network packets are transmitted, 0 listeners are created, and 0 outbound connections are initiated.
- **Privilege Requirements:** Defaults to unprivileged non-root user (`UID > 0`). Zero `sudo` or root requirements for baseline diagnostics.
