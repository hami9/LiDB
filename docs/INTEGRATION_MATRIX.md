# Integration and hardware capability matrix

All cells are **planned targets**, not verified implementation status. Support must be tested against versions and permissions recorded in CI/hardware reports.

| Environment | Baseline system | eBPF | NVIDIA GPU | Multi-node AI | Notes |
| --- | --- | --- | --- | --- | --- |
| Linux x86_64 VPS | required | optional | unavailable without hardware | optional | Headless read-only operation |
| Linux aarch64 host | required | optional | conditional | optional | Cross-build and native smoke tests |
| Docker/container host | required with host access | permission-dependent | conditional | conditional | Namespaces and cgroups vary |
| NVIDIA CUDA workstation | required | optional | probe NVML/DCGM | optional | Not every sensor supported |
| DGX Spark GB10 | required | optional | unified-memory-aware | via ConnectX | Validate using actual GB10 node |
| Two+ DGX Spark nodes | required/node | optional | per-node | runtime + fabric adapters | Ethernet/RoCE, not inter-node NVLink |
| NVLink/NVSwitch DGX systems | required | optional | platform-dependent | platform-dependent | Dedicated topology adapter |

## Adapters and primary sources

- **Linux:** procfs/sysfs/cgroup v2, PSI, rtnetlink, inet_diag, netfilter state. Prefer netlink for structured state; document polling limitations.
- **NVIDIA:** NVML/DCGM and supported CLI/API; metric list runtime-probed. Some unified-memory or GB10 counters may be missing or inapplicable. Do not infer memory metrics from discrete-GPU conventions.
- **Network fabric:** ethtool, devlink, IB/RDMA utilities and NIC counters where applicable. Report advertised link speed distinctly from observed throughput.
- **NCCL:** opt-in log/trace/instrumentation supplied by workload or operator; rank ID, transport, communicator, collective, duration and timestamps require meaningful correlation IDs.
- **Inference engines:** vLLM, NVIDIA Triton, TGI, Ollama and other integrations can be added using public metrics/protocols. Existence of an endpoint is not proof a metric is available.
- **Container/runtime context:** containerd or Docker metadata only when socket permissions are explicitly granted; do not require root-level Docker socket for baseline collection.
- **Distributed runtime:** PyTorch, Ray, Slurm, Kubernetes (future) via read-only integration adapters; no assumption that all users have an orchestrator.

## Unsupported behavior

Always show **capability / reason / documentation / available fallback**. Never label a cross-node ConnectX link `NVLink`. Never attribute a latency regression to GPU, NCCL or RDMA without observed supporting evidence and clock uncertainty assessment.
