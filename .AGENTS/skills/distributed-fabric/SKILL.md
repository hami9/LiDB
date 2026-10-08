---
name: lidb-distributed-fabric
description: Multi-node topology, ConnectX, RoCE/RDMA and NCCL correlation.
---
# Distributed fabric skill

**Use when:** fabric telemetry, cluster graph, rank-to-NIC mapping and collective communication analysis.

- Explicitly classify physical link, advertised capacity, measured throughput, error counters and transport.
- For multi-Spark use ConnectX Ethernet/RoCE terminology, not GPU-to-GPU NVLink. Detect actual NVLink/NVSwitch only on applicable platforms.
- Keep distinct concepts: RDMA capability, RoCE configuration, NCCL algorithm/transport choice and application-level collective performance.
- Link node ID, runtime rank and NIC identity with evidence and uncertainty; handle multi-NIC ambiguity.
- Cross-node timestamps require synchronization quality and uncertainty bounds before event correlation.
- Instrumentation and NCCL logs are opt-in; never alter job scheduling or runtime flags on behalf of the operator.
- Reproduce known communication slowdowns on approved nodes before diagnosing a fabric root cause.
