# LiDashBoard — Architecture, Product Requirements & Agent Implementation Plan

**Status:** Architecture proposal / implementation blueprint (not an implementation)  
**Edition:** v1.0 research-backed design  
**Date:** 2026-10-08  
**Primary platforms:** Linux x86_64 and Linux aarch64, with first-class NVIDIA DGX Spark (GB10) support  
**Core languages:** Rust (user space, TUI, state, diagnostics, services); C (kernel eBPF programs and narrowly scoped native interop)  
**Interface:** terminal only — interactive TUI, CLI, local IPC; no browser required  
**Philosophy:** read-only, low overhead, verifiable evidence, graceful fallback, opt-in control

> **Research scope.** This document synthesizes official kernel/NVIDIA/framework documentation and a representative selection of open-source reference implementations (links in §23). It is not a claim to have read every paper ever published on these topics. Facts and known limitations are distinguished from proposed design, estimations, and unvalidated performance goals.

---

## 0. Executive decision

Build **an AI-infrastructure-aware Linux observability and diagnosis platform**, not a generic network monitor, hardware manager, replacement TCP/IP stack, or autonomous cluster scheduler. LiDashBoard answers:

1. **What is happening?** Host, processes, containers, sockets, NICs, RDMA, GPU, inference jobs, model-serving engines, and distributed ranks.
2. **Where is the bottleneck?** CPU scheduling, memory pressure, GPU saturation, I/O, network congestion, communication collectives, model queuing, or the configuration between them.
3. **What evidence supports the diagnosis?** Timestamped, sourced telemetry and discriminating tests; no fabricated certainty.
4. **What should the operator do?** Prioritized, safe suggestions, supported diagnostics, reviewable and reversible changes only when explicitly enabled.

**Key differentiator:** the *correlation layer*: user request → inference engine/model → process/container → GPU/rank → transport/NCCL → NIC/fabric → peer node, shown in a fast keyboard-only TUI over SSH.

### Core non-negotiables

- `lidash` starts immediately in a basic **non-root** mode; missing vendor/kernel functionality must show `unsupported` or `permission_required`, never become a startup crash.
- The interactive TUI does **not** run as root. An optional, narrow privileged collector/helper mediates access.
- **No packet contents, prompts, outputs, secrets, TLS keys, or model weights are collected by default.** Primarily metadata/aggregates.
- **DGX Spark inter-device fabric is ConnectX-7 200GbE Ethernet (with RoCE support), *not external NVLink*.** NVLink-C2C is the GB10 CPU↔GPU coherent on-package interconnect. Generic NVLink/NVSwitch support is a separate hardware adapter for platforms that actually expose it. See NVIDIA ConnectX-7 networking, DGX Spark playbooks and NVLink-C2C disclosures [S01–S04, S30].
- Do not infer *dedicated VRAM* usage on DGX Spark from unsupported `nvidia-smi` fields; model unified memory and system pressure instead [S05].
- Do not claim a distributed job is load-balanced merely because GPU utilization appears equal: **rank critical-path latency, queue time, collective wait, effective throughput, and stragglers** decide.
- Phase 1 is observability/diagnostics, not controlling firewalls, changing routing, or moving running jobs.

## 1. Product boundaries and personas

### Personas

| Persona | Real problem | LiDashBoard's answer |
|---|---|---|
| Single-VPS Linux developer | Why is FastAPI/nginx slow despite low CPU? | Correlate request latency, TCP retransmissions, DNS/TLS, socket queues, CPU/IO PSI, container path |
| Local AI developer | Why are tokens/s or TTFT unstable? | Relate serving queue, KV cache, GPU load, UMA pressure, CPU scheduling and network |
| 2–4 DGX Spark operator | How is computation distributed; is the ConnectX fabric healthy? | Physical/logical NIC mapping, per-node heat map, NCCL ranks/collectives, RoCE counters and straggler hypotheses |
| DevOps / SRE | Can I diagnose a remote host over SSH without a browser? | Responsive TUI, explainable incidents, local/remote agent, copy/export evidence |
| Researcher | Which topology/model-parallelism strategy fits a cluster? | Explicit-capability topology explorer, benchmark baselines, **advisory** placement simulation |

### Out of scope for v1

A full HTTP reverse proxy, full flow packet reassembly, a generic distributed training orchestration platform, turnkey STUN/TURN service, production-changing load balancer, or vendor-independent GPU profiler. These may be integrated as **external systems** when they exist.

## 2. Correct Linux network model

`FastAPI → HTTP → nginx → application → socket → TCP/UDP/QUIC → kernel → conntrack → nftables → route → namespace → veth → NIC` is a useful learning list, **not an invariant packet traversal order**.

- **Application layer:** FastAPI/Uvicorn, nginx, vLLM/Triton, gRPC; TLS can terminate at different processes; HTTP/3/QUIC is usually UDP in user space and not a kernel TCP protocol.
- **Kernel:** sockets, L4 protocols, Netfilter hooks at distinct positions (and hook priorities), routing policy, interfaces/qdisc, neighbors, bridges, container namespaces and NIC drivers. `conntrack` is invoked through hooks when enabled, not necessarily for every packet.
- **Ingress:** wire → NIC driver / optional XDP → skb path / tc ingress → optional bridge/network namespace contexts → Netfilter PREROUTING / applicable connection tracking → routing decision → INPUT or FORWARD → local socket or onward egress (simplified).
- **Egress:** application socket → kernel stack → route selection and applicable LOCAL_OUT / POSTROUTING hooks → qdisc/tc egress → NIC. Container/veth paths may cross namespaces and hooks multiple times.
- **Observation points differ:** XDP may run before skb construction; tc/eBPF, socket tracepoints, conntrack events and packet capture each see different stages. Do **not** pretend a capture on one hook proves where packets disappeared elsewhere.
- **Control plane:** rtnetlink, generic netlink (ethtool/devlink, WireGuard where available), nftables/netfilter, cgroups, network namespace discovery.
- **Data plane:** existing Linux kernel and NIC features. LiDashBoard does not forward application traffic by default.

**Required networking objects:** interfaces, addresses, neighbors/ARP/ND, routes and policy rules, qdiscs, conntrack (when available), socket states, TCP retransmits/RTT, UDP errors, MTU, VLAN/bridge/veth, tunnels, physical NIC telemetry, DNS resolver state, firewall ruleset metadata, container links.

**Specialty features:** TUN/TAP, WireGuard, NAT, STUN/TURN/ICE, QUIC/HTTP3, TLS/mTLS, eBPF/XDP, netlink, epoll and io_uring are **inspectors or protocol-specific diagnostics**, not mandatory components of one packet pipeline.

## 3. Architecture: processes, planes, trust boundaries

```text
                           UNPRIVILEGED TERMINAL
             +---------------------------------------------+
             | lidash (Rust CLI + Ratatui UI)              |
             | Overview / AI / Cluster / Network / RCA     |
             | search, time-window, evidence, export       |
             +---------------------+-----------------------+
                                   |
                       Local authenticated Unix socket
                        versioned protocol; snapshots
                                   |
             +---------------------v-----------------------+
             | lidashd (Rust local node daemon)           |
             |                                              |
             | inventory | adapters | pollers | events      |
             | normalization + clock alignment             |
             | bounded aggregation + correlation engine    |
             | histograms + ringbuffer + SQLite (optional) |
             | SLOs / alerts / evidence provenance          |
             | capability matrix & degradation behavior    |
             +---------------------+-----------------------+
                                   |
                       narrow broker API (optional)
                                   |
             +---------------------v-----------------------+
             | lidash-helper (privileged *separate* service)|
             | allowlisted read ops / eBPF loader / ioctl  |
             | capabilities, seccomp, bounded resource use |
             +------+----------+------------+--------------+
                    |          |            |
     +--------------v-+  +-----v------+ +---v----------------+
     | Linux collectors|  | AI adapters| | Fabric adapters    |
     | proc/sys/cgroup |  | NVML/DCGM  | | ConnectX ethtool   |
     | netlink/nftables|  | vLLM/Triton| | RDMA/RoCE counters |
     | C eBPF CO-RE    |  | Ollama etc.| | NCCL RAS + logs     |
     +-----------------+  +------------+ | NVLink-capable GPUs|
                                        +--------------------+
                         Optional authenticated inter-node transport
             +--------------------------+-----------------------+
             | cluster coordinator in lidashd (not mandatory)    |
             | inventory, topology, job graph, time sync quality |
             | remote snapshots, diagnostics, placement advice  |
             +--------------------------+-----------------------+
                                     1..N peer lidashd nodes
```

**Three operating modes**

- **Local:** `lidash` reads through local `lidashd`; standalone direct unprivileged snapshot mode is available if daemon is absent.
- **Remote view:** `lidash --connect unix:/...` local, or a secure SSH-forwarded Unix socket / authenticated peer channel. No unauthenticated web service.
- **Cluster:** selected `lidashd` coordinates a set of trusted node agents; read-only federated telemetry as the default, and no single point of failure for *local* monitoring. Coordinator is not a job scheduler.

### Responsibilities and ownership

- `lidash` owns UI only, no privileged logic, no blocking driver calls on render thread.
- `lidashd` owns collector lifecycle, retention, discovery, normalization, validation, correlations and diagnostics.
- `lidash-helper` owns minimal privileged operations; runs with restricted identity/capabilities (capability availability depends on kernel/security policy), no arbitrary shell execution or user-defined code.
- Connector plugins run as versioned, time-limited adapters; isolate vendor crashes/unavailable libraries. C FFI confined to wrappers audited/tested for memory safety.
- Cluster transport owns mutual authentication, per-node authorization, replay defense, sequence numbers, reconnect and throttling; no implicit discovery across public networks.

## 4. Hardware/topology semantics: Spark vs NVLink

### 4.1 Categorize connections by evidence, never by marketing name

| Edge kind | Example | Inspect via | Interpretation |
|---|---|---|---|
| `CPU_GPU_COHERENT` | GB10 NVLink-C2C | vendor platform metadata / capability checks | **Inside one device**; not a cable between Spark nodes |
| `NIC_ETHERNET` | Spark ConnectX-7 QSFP | sysfs + PCI topology + netlink + ethtool | Physical Ethernet inter-node path |
| `RDMA_ROCE` | Supported RoCE over ConnectX | RDMA sysfs / ibverbs discovery / counters | GPU-related workloads may use RDMA transport |
| `GPU_NVLINK` | Compatible multi-GPU host | NVML/DCGM as supported | Actual peer-GPU links only |
| `GPU_NVSWITCH` | NVSwitch-capable DGX/HGX | supported NVIDIA APIs/DCGM | Switched intra/inter-server NVLink fabric on supported platforms |
| `TCP_SOCKET` | fallback communicator/bootstrap | socket/process/NCCL config | Not equivalent to RDMA bandwidth |
| `VETH_TUNNEL` | Docker/overlay/WireGuard | netlink, namespace, route | Virtual path, may overlay physical network |

**DGX Spark-specific inventory:**

- Map physical left/right QSFP to the Linux logical Ethernet and RoCE interfaces; NVIDIA documents that each QSFP port corresponds to **two Linux Ethernet interfaces and their RoCE counterparts** due to the PCIe topology. Avoid treating those logical interfaces as four independent physical QSFP cables [S02].
- Two QSFP physical ports, each **up to 200 Gb/s** depending on physical setup and cable; **never** present 400 Gb/s as measured available application throughput simply by summing two nominal port rates [S02].
- NVIDIA's documented scenarios include two directly connected nodes, a three-node direct setup, and four via a switch [S03, S04]. Other sizes/topologies are exploratory **only after hardware and driver validation**. No automatic assumption that arbitrary counts of Sparks form one shared-memory computer.
- The cluster is **distributed-memory across nodes**. Aggregate system DRAM across boxes is not transparent unified memory; model parallelism/runtime must explicitly partition state and communicate tensors.
- Preserve management and compute networks as separate roles even when a fabric serves both; discover actual chosen NCCL/RDMA interface.
- Monitor NIC link state/speed/duplex, errors, discard counters, PFC/ECN/CNP metrics **only if exposed by driver/switch**, link flap and thermal indicators **when supported**.

### 4.2 Cluster fabric graph

```
Node A ─[physical QSFP, ConnectX-7, Ethernet/RoCE]─ Node B
   ├─ GB10 CPU --[NVLink-C2C]-- GPU A
   └─ NIC ↔ PCIe ↔ SoC
Node B ─ GB10 CPU --[NVLink-C2C]-- GPU B

NCCL job: rank 0 on GPU A ⇄ rank 1 on GPU B
Transport: selected NIC/interface ↔ Ethernet/RoCE fabric
```

**On platforms with real GPU-to-GPU NVLink/NVSwitch**, the adapter reports per-link counters/topology when vendor APIs actually support them; otherwise `unsupported`, not zero traffic. Do not attribute such per-link NVLink counters to Spark QSFP.

## 5. End-to-end AI workload model

```text
Client request
 → ingress: nginx / API / gRPC / OpenAI-compatible server
 → model-serving request & queue
 → scheduler/batch/prefill/decode/KV-cache
 → process → cgroup/container → GPU/device
 → distributed job / parallel group / rank
 → communication operation (NCCL / other backend)
 → transport selection / network interface / route / RDMA queues
 → physical link / peer node
 → output tokens / latency / error rate
```

**Data contract objects:**

- `Host`, `GPU`, `CpuPackage`, `MemoryDomain {dedicated|unified|unknown}`, `NIC`, `LogicalInterface`, `RdmaPort`, `Link`, `NetworkNamespace`, `Cgroup`, `Container`, `Process`, `Socket`.
- `ModelServer`, `ModelInstance`, `InferenceRequestAggregate`, `DistributedJob`, `Rank`, `ParallelGroup {data|tensor|pipeline|expert|unknown}`, `Collective`, `TraceEvidence`, `Incident`, `DiagnosticHypothesis`.
- Each object includes stable ID (within its actual lifetime), node ID, capability set, time range and provenance.
- Separate `observed`, `derived`, `estimated`, `unavailable`, `redacted` in the schema. Unknown ≠ zero.

### 5.1 AI telemetry providers

| Provider | Collection | Important limits |
|---|---|---|
| NVML | GPU identity, activity, power/temp/clocks/health **where exposed** | Not all metrics are available on every GPU; GB10 UMA must use different memory semantics |
| DCGM / exporter | Supported per-GPU, health and profiling counters; on suitable devices NVLink link data | Conditional installation, NVIDIA permissions, MIG/platform support and profiling multiplexing [S07] |
| Linux `/proc`, `/sys`, PSI/cgroup v2 | DRAM, available/reclaimable, swap, page faults, OOM, CPU/memory/IO stalls, process resource usage | Does not magically identify model-exclusive GPU memory |
| vLLM metrics | TTFT, TPOT/ITL, tokens/s, requests queued/running, KV-cache hit/use, errors | Metric names and availability version-dependent; prefix labels bounded [S12] |
| NVIDIA Triton | request counts, queue/compute breakdown, batch/inference latency, GPU metrics when supported | Per-engine metadata; does not by itself prove root cause [S13] |
| Ollama / llama.cpp | process/model identification, engine/endpoint statistics where documented and enabled | No universal guaranteed per-token metric across engines; adapter degrades explicitly |
| NCCL | version/config, communicator and rank health, optional RAS, supported diagnostics | Collectives are not visible from kernel sockets alone; cooperation/log/RAS needed [S08, S09] |
| CUDA/Nsight external diagnostic | Optional profiling artifacts | Explicit on-demand action; no continuous invasive profiler |

### 5.2 DGX Spark unified memory correctness

Instead of `VRAM used/total`, use:

- `system_ram_total`, `MemAvailable`, active anonymous/file memory, resident/working sets, swap used and swap-in/out rates.
- PSI `memory.some/full`, cgroup `memory.current`, `memory.events`, `memory.swap.current` where available, relevant OOM events, major faults.
- GPU utilization and memory-related GPU/driver statistics **when supported**, labeled independently from DRAM.
- `memory_available_for_new_workload` is *not* a directly observed CUDA fact; if modeled, show it as an uncertainty-bounded estimate. NVIDIA specifically documents that `cudaMemGetInfo` may not account for reclaimable RAM from swap on DGX Spark [S05].
- No fake addition of “RAM + VRAM” on unified-memory systems and no double counting of mapped memory.

### 5.3 Distributed load metrics and straggler diagnosis

The question is not simply which GPU has a lower utilization %. Measure, for the same job and time window:

1. Per-rank iteration/step latency or token-processing rate, **only via supported application telemetry**.
2. `rank_wall_time` and idle/wait time, collective durations/bytes by operation where instrumented.
3. Per-node compute utilization, memory pressure, CPU PSI, context switches and queue depth.
4. NIC TX/RX bytes/s, interface selection, link state, retransmissions, RDMA errors and congestion counters when available.
5. Batch sizes, prefill/decode balance, KV cache occupancy/evictions and model parallel mapping where the serving stack exposes them.
6. Link throughput against an **experimentally recorded, topology-specific baseline**, not just advertised wire rate.
7. Relevant engine errors, NCCL warning/RAS signals, clock/version/config mismatches.

**Derived indicators with formulas and limitations:**

- `rank_compute_imbalance = max(per_rank_compute_time) / median(per_rank_compute_time)` (only for equivalent ranks and time windows; ratio has no meaning if roles differ).
- `rank_step_skew = p95(rank_step_duration) - p50(rank_step_duration)` where comparable.
- `fabric_utilization_observed = measured_bits_per_second / negotiated_line_rate` **per physical port**; not NCCL efficiency.
- `collective_efficiency = measured_algorithm_or_bus_bandwidth / same-test_baseline_bandwidth`; benchmark type/units must match.
- `request_latency = queue + compute_input + compute + compute_output + remaining_overheads` **only if same engine defines these components consistently**.
- `effective_cluster_throughput` from accepted request throughput/tokens per second or job steps/s; exclude failed/canceled work.

**Hypothesis example (not automatically truth):** TTFT rises + queue time rises + GPU busy + stable fabric → likely demand/capacity constraint. TTFT rises + NCCL collective wait rises + affected peer NIC drops + unchanged GPU compute → fabric issue candidate. GPU workload throughput falls + PSI memory spikes + swap-ins increase on a Spark → likely UMA pressure. Display every assertion with quality/confidence and corroborating data.

## 6. Placement and load distribution: what LiDashBoard can and cannot do

**Release 1:** display/discover current work placement. **Release 2:** explain inefficiencies and make **recommendations**. **Future opt-in extension:** *request* workload modifications through a supported backend (Ray/vLLM deployment manager/Kubernetes APIs), never kill/migrate unseen in-flight state.

**Advisory planner inputs:** available and reservable DRAM per node; GPU compute family/capability; CUDA, driver and NCCL versions; model weights (size/quantization); context length; KV-cache budget; selected tensor/pipeline/data/expert parallel strategy; communication sizes/frequency; measured link bandwidth/latency; topology and scheduler integration; desired p95 TTFT and throughput; NUMA and CPU limits.

**Planner decisions:**

- `data_parallel`: replicate eligible model shards on multiple nodes; improves request capacity for independently servable models, but requires *per-replica sufficient memory*.
- `tensor_parallel`: communicate activations/collectives frequently; fabric-sensitive.
- `pipeline_parallel`: distribute layers and manage bubble/stage imbalance; potential lower collective frequency but introduces cross-stage traffic.
- `expert_parallel`: conditional routing and all-to-all can be fabric-sensitive.
- `hybrid`: prefer runtime-supported combinations; avoid assigning unsupported settings.

**Algorithm (advisory):** reject infeasible configurations first; then score weighted normalized estimates of p95 latency, throughput, memory risk, inter-node communication volume, resilience, and operational complexity using **measured** baseline models. Output a ranked set of alternatives with trade-offs. All estimates must have confidence/inputs; when the engine doesn't expose the required data, return `insufficient_evidence`.

**Critical placement safety:** A faster interconnect does not automatically solve scheduler overhead, low batch efficiency, wrong sharding, slow storage, model cold starts, or UMA pressure. There is no single exact “% load balance” metric.

## 7. Event collection pipeline

```text
[source: proc/sys | netlink | eBPF | NVML | DCGM | inference | NCCL]
    → CollectorAdapter {capability, cost_class, sampling_policy}
    → source-specific parsing + validation (explicit units/monotonic time)
    → bounded per-source queue; drop accounting; priority lanes
    → normalized event/metric model (schema version, source, quality)
    → time-window aggregation (histograms, sketches, low-cardinality keys)
    → correlation graph / rule engine / incident evidence
    → in-memory ring + optional SQLite/WAL rollups + bounded retention
    → local UDS subscriptions / cluster read-only snapshots
    → TUI/CLI and optional localhost telemetry export
```

- Default sampling: host network/CPU ~1 s, GPU ~1–2 s subject to API limits, RDMA per-port ~2 s, cluster snapshots ~2–5 s, deep tracing **off**; tune after profiling. These are *initial design defaults*, not vendor limits.
- Adaptive updates: 1–5 Hz TUI refresh depending on terminal; do **not** force 5 Hz driver polling. Coalesce render updates, cap costly queries.
- Bounded queue/drop budget with visible `collector_dropped_events`/`collector_last_success` counters. Never backpressure network data path through dashboard user space.
- Metrics label discipline: avoid per-request ID, full IP, PID as unbounded Prometheus label cardinality. Internal inspectable records have TTL/caps.
- Time: local monotonic durations; wall-clock UTC for storage; record clock-sync status/offset uncertainty for **cross-host** comparisons. Never subtract independent monotonic clocks on different nodes.
- Retention: in-memory live window by default; optional SQLite WAL event metadata + aggregated history; quotas and secure rotation. Do not save raw packet payload by default.

## 8. eBPF/C vs Rust responsibilities

**C**: CO-RE BPF probes (tracepoints/fentry where stable/supported, socket and TCP observation, optional XDP/tc counters), packed ABI event structures, minimal native wrappers required by vendor APIs. C code must be small, verifier-friendly, no unbounded loops, bounded map sizes. C is *not* an excuse to re-implement a TCP stack.

**Rust**: libbpf-rs + libbpf-cargo loader/bindings, collectors, netlink access, async orchestration, schema and event buses, snapshots, correlation engine, security policy, storage, terminal TUI, diagnostics and tests. Use safe wrappers/`unsafe` isolation, sanitize every FFI boundary.

**Kernel compatibility:** target modern BTF-enabled Linux (initial validation against e.g. Ubuntu/DGX OS and supported x86_64/aarch64 kernels); probe per-feature support at runtime; provide proc/netlink fallback on unsupported kernels. Avoid blanket assertions that a kernel version alone guarantees all BPF hooks.

**Performance choice:** Tokio/epoll-backed I/O is the baseline. `io_uring` is an **optional, benchmark-gated experimental backend**, never an assumed general win; no compulsory use of AF_XDP or packet capture. XDP observability must be opt-in where it changes attach state or driver behavior [S19, S20].

## 9. Network observability modules

| Module | Baseline telemetry / output | Advanced / opt-in |
|---|---|---|
| `net.interfaces` | netlink enumeration, speed, carrier, MTU, stats | devlink health, selected vendor NIC counters |
| `net.routes` | per-namespace routes, rules, neighbor, route-get | dry-run what-if route evaluation |
| `net.sockets` | proc/inet_diag socket state, process best-effort join, RTT/retrans | eBPF PID/cgroup attribution |
| `net.netfilter` | nftables ruleset read-only, conntrack pressure, drop evidence | Netfilter hook/TC trace, explicit plan only |
| `net.containers` | namespace↔veth↔bridge↔container mapping | orchestrator annotations |
| `net.tunnels` | WireGuard metadata, TUN/TAP status, MTU/route | STUN/ICE diagnostic only with explicit consent |
| `net.dns` | resolver config, query errors/timing on explicit probe | comparative configured resolver tests |
| `net.tls_http` | endpoint handshake timing, protocol negotiation with consent | optional app-side instrumentation, never TLS decryption by default |
| `net.quic` | UDP/QUIC endpoint/flow metadata; application QUIC metrics | engine integration; no claim of blind QUIC L7 visibility |
| `net.packetpath` | explain expected path from route/firewall/topology | scoped tracing of drops and hook stages |
| `net.nic` | ethtool/driver, IRQ/queue/softnet drop indicators | RDMA counters, RoCE congestion, XDP queues |
| `net.capture` | **disabled** | opt-in bounded filter/snaplen/time, restricted export, access audit |

**Network path explorer** returns `observed|inferred|unobservable` at every hop, and includes source, namespace, chain/hook, routing table and interface. It must not claim universal packet visibility.

## 10. DGX Spark/AI specialized modules

1. **Platform Detector**: identify GB10/aarch64 and software/driver stack; use capability probes not brand alone.
2. **UMA Pressure View**: aggregate Linux memory pressure with GPU activity; flags swapping/OOM and differentiated DRAM vs unsupported dedicated VRAM.
3. **Spark Port Map**: physical QSFP ↔ PCIe functions ↔ Linux Ethernet ↔ RoCE netdevices; distinguish `physical_port_id` from `ifindex`.
4. **Fabric Graph**: known peer edges from trusted config + active links, with a `discovery_confidence` field; never blindly infer cable endpoints from names alone.
5. **NCCL Inspector**: versions, rank ↔ GPU ↔ node ↔ job mapping; RAS status if available; selected interface/transport; unknowns marked.
6. **Collective Analysis**: AllReduce/AllGather/ReduceScatter/AllToAll timings and sizes when instrumented; baseline/variance, no kernel-only invented counters.
7. **Straggler Analyzer**: find comparable slow ranks, correlate stalls, pressure, NIC errors and config skew; show ranked hypotheses.
8. **Serving Inspector**: vLLM/Triton specific TTFT/TPOT/queue/KV metrics; optional OpenAI-compatible app metrics adapter if exported.
9. **Model Placement Advisor**: advisory simulation and constraints; no unsanctioned workload control.
10. **Cross-host Incident Timeline**: time-sync quality shown alongside probable cause; no false submillisecond precision.
11. **Power/Thermal Watch**: temperature/clocks/power when vendor API supports; identify throttling candidates, never assume measurements from TDP specifications.
12. **Fallback/Bootstrap Path Inspector**: identify accidentally selected management or slower TCP interfaces instead of high-speed compute fabric.
13. **Baseline Recorder**: controlled, explicit benchmark metadata for NCCL/latency/throughput; never run disruptive bandwidth tests automatically.

### Real NVLink / NVSwitch adapter

- Detect GPU-to-GPU NVLink or NVSwitch topology only via supported APIs.
- Report peer link-up, negotiated properties and TX/RX/error metrics when **available on that GPU/driver**; capability-gated.
- For DGX Spark, show `NVLink-C2C: CPU↔GPU intra-device`; show `inter-node: ConnectX-7 Ethernet/RoCE`, not fictitious NVLink between devices.
- NVIDIA DCGM supports per-link NVLink profiling on supported platforms; its mere existence does not imply DGX Spark exposes those counters [S07].

## 11. Root-cause reasoning engine (RCA)

Use transparent **rules + evidence graph first**; optional LLM summarizer can come later but may not create unsupported claims.

**Diagnosis record**

```json
{
  "hypothesis_id": "H-FABRIC-003",
  "label": "Inter-node communication degraded",
  "assessment": "possible",
  "confidence": "medium",
  "window": {"start_utc": "...", "end_utc": "...", "clock_quality": "synced"},
  "evidence": [
    {"metric": "rdma.port.discards", "source": "sysfs", "status": "observed"},
    {"metric": "nccl.collective.wait_p95", "source": "engine_adapter", "status": "observed"}
  ],
  "counterevidence": ["..."],
  "missing": ["switch_queue_drops"],
  "recommended_checks": ["verify chosen transport", "compare same-size NCCL baseline"],
  "automated_change": false
}
```

**Diagnosis rules** must avoid causation from simple correlation, consider alternate explanations, rate-limit alerts, and make suggestions proportional to evidence and blast radius. Export evidence bundles stripped of sensitive data by default.

## 12. Product UX: terminal layout, navigation and accessibility

### TUI views

| Key | View | Content |
|---|---|---|
| `1` | Overview | host health, CPU/IO PSI, RAM/UMA, GPU, network, top incidents |
| `2` | AI Workloads | engines, models, requests, TTFT/TPOT, tokens/s, KV cache, queue |
| `3` | GPU/Memory | GPU utilization where supported, energy/temp/clocks, UMA pressure and swap |
| `4` | Cluster | nodes, logical & physical graph, rank map, sync quality, fabric health |
| `5` | NCCL/Fabric | collective results, RDMA counters, chosen interfaces, stragglers |
| `6` | Network | interfaces/sockets/routes, container path, DNS/TLS, packet drop evidence |
| `7` | Diagnostics | ranked hypotheses, contrary evidence, explanations, safe playbooks |
| `8` | Timeline | correlated events, annotations, history/replay, exports |
| `9` | Settings | capabilities, collector cost, retention, permissions, integrations |

`/` search, `?` shortcuts, `Tab` focus, `Enter` drill down, `Esc` back, `q` quit, `e` evidence export, `p` pause display (not collection), `t` time window, `g` graph view, `c` compare nodes. All keyboard-only; 80×24 usable fallback, 120×35 preferred for cluster views. ANSI color is supplementary, readable without color. No flicker, no blocking telemetry on input events.

### Example (illustrative, not live measurements)

```text
LiDashBoard  [CLUSTER]  3 nodes   fabric: watch   TS sync: ±3 ms  [read only]
NODE         CPU PSI   UMA pressure  GPU activity  NCCL rank   FABRIC
spark-a      low       medium        busy          rank0       200GbE up
spark-b      low       high          moderate      rank1       200GbE up
spark-c      medium    low           busy          rank2       200GbE up

INCIDENT CANDIDATE: rank1 straggler
  observed: high memory pressure + swap-in activity on spark-b
  observed: p95 stage latency larger than comparable ranks
  unknown : operator-visible NCCL collective duration (adapter absent)
  Next: inspect vLLM queue / confirm comparable ranks / check fabric drops
```

## 13. Storage, schemas and API contract

**Canonical Rust data envelope:**

```rust
struct Observation<T> {
    schema_version: u16,
    node_id: NodeId,
    source_id: SourceId,
    observed_at_utc_ns: i128,
    monotonic_elapsed_ns: u64, // local only
    quality: Quality,          // Observed, Estimated, Unsupported, PermissionDenied, Stale
    value: T,
}
```

- Snapshot query + bounded streaming subscribe via **versioned Unix domain socket** (`protobuf`/`prost` or another documented typed versioned schema); restrict peer via OS credentials/permissions. Define payload limits and deadlines.
- Use monotonic increasing local sequence numbers and per-stream drop accounting.
- Historical aggregation: optional embedded SQLite/WAL with migrations, retention and safe schema upgrade; always retain source and units.
- Cluster protocol: pinned/cert-authenticated mTLS **only when network transport enabled**; SSH-tunneled transport acceptable for small setups. Verify identities, roles and node pairing; reject untrusted agent endpoints.
- Optional exporters: Prometheus localhost only and JSONL/CSV offline export, not mandatory web dashboard. When exports include IP/PID/model IDs, apply privacy controls.
- `FeatureSupport` typed status: `{available, unsupported_hardware, unsupported_kernel, missing_dependency, permission_required, disabled, stale}`. Feature detection is cached and periodically rechecked.

## 14. Security and privacy design

- **Read-only default** across all hosts; no `sudo lidash` and no globally permissive Unix socket.
- Distinct roles: viewer, diagnostic_operator, administrator. An `operator` can request bounded diagnostics; only admin with explicit two-step confirmation may apply changes in *future phases*.
- Restrict privileged helper API to typed allowlisted operations. No `sh -c`, arbitrary executable, arbitrary path writes, wildcard BPF programs or arbitrary sysctl.
- Drop privileges after initialization where feasible; minimize Linux capabilities, seccomp, systemd sandboxing (ProtectSystem, NoNewPrivileges where compatible), resource limits and audit events.
- User/session authorization for socket and peer data; consider multi-user hosts and process privacy. Do not expose other users' command lines, environments, request bodies or secret-bearing logs by default.
- No always-on packet sniffing, MITM, TLS decryption, or unrequested traffic generation. Scope capture or NCCL benchmarks explicitly and warn about performance and privacy cost.
- Remote agent enrollment uses authenticated approval and scoped node identities; enforce mTLS/cert rotation or SSH tunnel policy; no broadcast secrets, hardcoded passwords, hidden service exposure.
- Store minimal identity metadata, redact tokens and paths on export, state retention and purge guarantees.
- No production firewall, routing, network namespace, kernel driver, RDMA, NCCL or service changes outside an explicit approved change plan with dry-run, scoped validity and rollback.

## 15. Performance and reliability engineering

### Budgets (engineering **targets**, not established benchmark results)

| Measure | Proposed verification target | Measurement protocol |
|---|---|---|
| Basic read-only CLI start | fast perceived startup (< 2s on reference host) | time with/without daemon |
| Default node monitoring overhead | aim < 1–2% CPU on reference idle host and modest fixed memory | compare baseline vs active, repeat across hardware |
| TUI latency | p95 key-to-render < 100 ms with synthetic stream | instrument event/render loop |
| Collector event loss | zero in normal baseline, prominently report under forced overload | synthetic 100k+ events/s load test and drop accounting |
| eBPF map allocation | strict configured upper bound | inspect memlock/map accounting per profile |
| Agent crash effect | no workload interruption and no stuck XDP attachment | terminate/restart with workloads running |
| Remote resilience | local host view remains functional if cluster coordinator unavailable | fault injection/network partition tests |
| Probe overhead | gated per probe and opt-in invasive features | A/B p50/p95 serving throughput & tail latency |

Actual thresholds must be rebaselined on DGX Spark, x86 Linux, idle/loaded inference and high-NIC-throughput environments. Avoid benchmarking on a machine while a production job is vulnerable to latency spikes.

### Failure isolation

Collector deadlines, circuit-breakers on bad vendor drivers, backoff/retry and last-good-value staleness, signal-safe TUI restore, WAL corruption recovery, clean eBPF unload, helper crash containment, peer restart handling, partial network partition states. High telemetry volume never blocks computation or the TUI.

## 16. Delivery matrix and phased roadmap

| Phase | Deliverables | Required acceptance gate |
|---|---|---|
| **P0** | Product/ADR docs, capabilities schema, Rust workspace, CI, mock data + TUI skeleton | compiles aarch64/x86_64; tests; UI works in narrow terminal |
| **P1** | local Linux CPU/memory/PSI/NIC/process collectors; unprivileged mode; UDS | truthful metrics, permissions fallback, no sudo UI |
| **P2** | Network map: routes, sockets, namespaces, Docker, nftables read-only, baseline diagnostics | integration netns/veth/iptables-nft/nft fixtures; route explanations |
| **P3** | C eBPF CO-RE optional sensors, event transport and loss accounting | verifier checks, bounded overhead, graceful non-BTF fallback |
| **P4** | AI adapters: NVML, DGX Spark UMA, vLLM, Triton (and capability-gated DCGM) | tests on mocked UMA and, when available, physical GB10; no fake VRAM |
| **P5** | Cluster: auth, coordinator, clock uncertainty, node graph, Spark QSFP logical/physical mapping | 2-node lab validation, simulate 3/4, partition recovery |
| **P6** | NCCL/RDMA/RoCE telemetry, RAS adapter, collectives and rank mapping | repeatable NCCL test fixture + real supported Spark case; real vs absent data states |
| **P7** | RCA with evidence/provenance, incident timeline, calibrated recommendations | adversarial counterexample tests, no unsupported cause assertions |
| **P8** | Placement advisor/what-if, baseline recorder, secure export, versioned API | planner rejects impossible memory/topology, no live workload control |
| **P9** | Production hardening, package/releases, stress, operator docs | security review, rollback, SLO regression, signed/artifact provenance |

Do **not** collapse P0–P9 into one giant AI-generated PR. Each phase is broken into small features, tested, reviewed, and merged independently. Linux x86_64 provides accessible early validation; GB10 aarch64 hardware gates remain explicit, not hand-waved as passing without hardware.

## 17. Suggested repository structure

```text
LiDashBoard/
├── Cargo.toml                   # workspace
├── Cargo.lock
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
├── ARCHITECTURE.md              # this document
├── AGENTS.md                    # agent instructions
├── ROADMAP.md
├── docs/
│   ├── adr/                    # decisions and alternatives
│   ├── kernel-networking.md
│   ├── dgx-spark.md
│   ├── schema.md
│   ├── privacy-threat-model.md
│   ├── performance-tests.md
│   └── runbooks/
├── crates/
│   ├── lidash-cli/             # CLI binary / Ratatui presentation
│   ├── lidash-daemon/          # background daemon
│   ├── lidash-helper/          # optional privileged binary
│   ├── lidash-core/            # domain models + capabilities
│   ├── lidash-protocol/        # UDS / remote wire messages
│   ├── lidash-collect-linux/   # proc, sys, netlink, cgroup, PSI
│   ├── lidash-collect-bpf/     # optional libbpf-rs frontend
│   ├── lidash-collect-gpu/     # NVML, DCGM capability wrappers
│   ├── lidash-collect-ai/      # vLLM, Triton, optional adapters
│   ├── lidash-collect-nccl/    # RAS and job evidence parsing
│   ├── lidash-fabric/          # Ethernet/RDMA/NVLink adapters
│   ├── lidash-topology/        # physical/logical graph
│   ├── lidash-correlate/       # diagnoser / rule engine
│   ├── lidash-placement/       # advisory, no execution
│   ├── lidash-store/           # bounded ring + SQLite
│   └── lidash-testkit/         # fixtures and simulation
├── bpf/
│   ├── include/                # stable C ABI events
│   ├── src/                    # narrowly scoped *.bpf.c
│   └── tests/
├── proto/                       # versioned schemas if protobuf
├── configs/                     # secure defaults + example profiles
├── packaging/                   # systemd units/deb/rpm scripts
├── tests/
│   ├── integration/
│   ├── fuzz/
│   ├── fixtures/
│   ├── simulated-cluster/
│   └── performance/
├── scripts/                     # only documented build/test utilities
└── .github/workflows/           # fmt, lint, test, build, audit, release
```

## 18. Five mandatory architecture reviews — outcomes

Each review uses a different failure lens. These are **design assessments**, not claims that implementation tests were run.

### Review A — Kernel/networking correctness

**Risks identified:** linearizing Netfilter hooks; conflating XDP/TC/socket stages; exposing containers' paths without namespace; inferring packet drops from absent samples; mixing protocol stacks.

**Changes adopted:** three-plane architecture, namespace-aware path engine, hook/source provenance, `observed/inferred/unobservable` classifications, optional tracepoints rather than magical complete visibility. **Gate:** trace path through veth + bridge + DNAT fixture and accurately state invisible portions.

### Review B — AI/GPU & distributed compute correctness

**Risks identified:** naming Spark inter-node Ethernet “NVLink”; summing DRAM into transparent pooled memory; reliance on unsupported dedicated VRAM stats; equating equal GPU usage with equal performance; interpreting NCCL counters without instrumented job context.

**Changes adopted:** explicit topology edge types, UMA-specific schema, NCCL RAS/engine adapters, comparable-rank analysis, model parallelism constraints, `unavailable` as first-class. **Gate:** tests reject the invented Spark-to-Spark NVLink edge and do not render unsupported VRAM %.

### Review C — Security / remote operations / privacy

**Risks identified:** root TUI, access to peer secrets, exposed unauthenticated metrics port, arbitrary shell helper, silent capture, unsafe firewall change, implicit SSH trust.

**Changes adopted:** least-privilege split, typed helper, UDS identity checks, explicit pairing, opt-in capture and probes, audit/redaction, read-only default, future change approval + rollback. **Gate:** hostile-local-client permission tests and untrusted-peer rejection.

### Review D — Performance & reliability

**Risks identified:** tracing overhead masks true model throughput; unbounded event streams; vendor call stalls rendering; `io_uring` chosen by fashion not measurements; remote coordinator downtime interrupts local observability.

**Changes adopted:** bounded queues, collector cost classes, sparse baseline samples, epoll baseline, benchmark-gated `io_uring`, source circuit breakers, independent local daemon, graceful cleanup. **Gate:** on/off monitoring A/B tests including p95 TTFT and tokens/s, plus crash/partition tests.

### Review E — Product / UX / extensibility

**Risks identified:** an overwhelming tool with 40 disconnected charts; 'AI dashboard' without actionable diagnosis; no usable non-GPU mode; proprietary-only adapters; useless alert noise.

**Changes adopted:** opinionated Overview → Workload → Evidence drilldown, role-based views, explicit missing data, generic host support, modular vendor adapters, feature-by-feature delivery with success scenarios. **Gate:** a user identifies a simulated slow inference cause without external commands, and non-GPU VPS remains useful.

**Open review items (must be resolved during implementation):** hardware-specific NVML/DCGM coverage matrix on actual DGX Spark and other GPUs; NDA/driver API constraints; secure and practical peer authentication choice; real NCCL signal availability per version; container correlation robustness; performance baselines on both target architectures; legal/redistribution review of vendored SDKs and probe licenses.

## 19. Test matrix: do not fake hardware success

| Fixture / host | Test | Expected |
|---|---|---|
| x86_64 Linux VM | CPU/PSI/network, no NVIDIA libraries | clean full host TUI, GPU `unavailable` |
| non-root SSH login | start `lidash` | no privileged crash; explain gated features |
| simulated container namespace | veth/bridge/policy route | accurate topology and status |
| forced BPF disable | disabled kernel capability/BTF | proc/netlink fallback; no app crash |
| aarch64 native/cross build | workspace toolchain | builds or explicit platform gate |
| Spark GB10 | unified memory and vendor metrics | no false VRAM; correct pressure display |
| two real Sparks | logical/physical QSFP mapping + 200GbE connection | one physical link per actual port, correctly mapped logical interfaces |
| three-node simulated ring | node drop and clock skew | node offline, uncertain cross-node timings |
| four-node switch topology | link discovery + NCCL interface selection | differentiated switch/peer edges |
| supported NVLink GPU system | peer-NVLink metrics when available | distinct graph path and metrics from Ethernet |
| vLLM/Triton mock | TTFT, queue and compute telemetry | units/labels and correct model mapping |
| NCCL RAS mock/real | rank timeout and config mismatch | error state with evidence; no invented cause |
| high telemetry volume | queue backpressure | dashboard protected; drops visible |
| agent restart or broken socket | reconnect | current local view eventually recovers; no workload impact |
| export containing possible tokens | privacy sanitizer | redaction and file permissions enforced |

**Cross-host clock test:** simulate 100 ms offset: suppress precise stage-to-stage latency claims while continuing to display independent per-host monotonic durations.

## 20. Acceptance scenarios phrased as product stories

**Scenario 1 — FastAPI slow:** nginx request p95 grows; discover which process/namespace and socket, compare CPU/IO PSI, retransmission and app metrics; describe likely cause and data gaps, without treating network speed as definitive.

**Scenario 2 — Spark out-of-memory behavior:** serving slows as memory pressure/swap increases; show coherent UMA-specific evidence and estimated reclaimability, not a fictitious dedicated-GPU memory percentage.

**Scenario 3 — Two Sparks not scaling:** one model spread across two nodes; compare baseline and observed tokens/s, rank mapping, job parallel strategy, selected ConnectX interface and NCCL/RDMA signals; explain why doubling systems does not guarantee double tokens/s.

**Scenario 4 — NIC mapping error:** logical Ethernet interfaces from one physical QSFP appear as separate names; correctly attach them to the physical port and avoid false link count.

**Scenario 5 — Mixed NVLink/ConnectX fleet:** show actual GPU NVLink/NVSwitch fabric *only* where hardware supports it and ConnectX Ethernet across Spark peers. Do not merge them.

**Scenario 6 — No high privilege:** unprivileged user receives informative, genuinely functional dashboard with precise capability status.

## 21. Implementation instructions to hand to an AI coding agent

> **Agent role:** Act as principal Rust/Linux systems engineer, C/eBPF developer, NVIDIA AI infrastructure specialist, performance engineer and security reviewer. Implement LiDashBoard incrementally according to this document. Never treat a proposal as already deployed or measured.

### Required operating procedure

1. Inventory the actual repository, existing files, toolchains and operating system before editing. Do not overwrite unrelated user work.
2. Read this architecture fully. Produce `ROADMAP.md`, `AGENTS.md`, ADRs and a small feature backlog that maps to **P0–P9**. Identify dependencies, CPU/GPU hardware test blockers, and exact acceptance tests.
3. Start with P0 only. Build a *working vertical slice*: reliable terminal UI → typed mock snapshot → local real proc/sys collector → correct fallback. Do not create a mountain of empty crates without running code.
4. Before any new technology/library call, inspect its current API and official docs. Favor pinned compatible Rust crates and system package versions; record `Cargo.lock`, supported kernel matrix, build prerequisites and reasons in an ADR.
5. Use Rust for host app/daemon/IPC, C for CO-RE eBPF programs and minimum needed FFI. Keep ownership/lifetimes clear. `unsafe` code needs documented invariants and tests.
6. Implement a capability registry and typed unsupported/denied status **before** vendor-specific integrations. Avoid panics/unwrap in error-prone collection paths.
7. Separate collection from UI by IPC. Avoid global root privileges, arbitrary shell, hidden listeners or automatic system config changes. Do not inject or bypass credentials.
8. For each phase: (a) plan small stories, (b) implement, (c) format/lint/test, (d) run focused integration/perf/fault tests where possible, (e) self-review from all five lenses, (f) update documentation, (g) prepare atomic commits/PR. **Stop and report actual blockers** rather than pretending a physical Spark test passed.
9. Use mock sources + deterministic fixtures for absent hardware, and a `requires-hardware` test label. If native DGX Spark/ConnectX tests are unavailable, mark those gates **pending**.
10. After each feature, report: changed files, design decision, exact commands run/results, observed tests vs simulations, risk, known limitations, next feature. Keep CLI help and docs synchronized.
11. Start diagnostics rules with evidence-proven heuristics; no LLM-generated factual diagnosis without supporting telemetry. If adding an optional LLM explainer later, local/off by default and sanitize input.
12. Never migrate, stop, kill, alter NCCL tuning, routes, firewall, NIC, performance governor or running inference jobs automatically. Only read-only observation and explicitly approved, scoped non-disruptive tests in MVP.
13. Do not claim performance superiority over another dashboard without a reproducible benchmark protocol, stated environment and measured results.
14. Enforce project conventions: Clippy and rustfmt, meaningful unit+integration tests, kernel probe verifier checks, CI x86_64/aarch64 builds, threat model, license checks, changelog and versioned interfaces.

### Immediate P0 acceptance criteria

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass in a supported development environment (or report the exact failure and reason).
- `cargo run -p lidash-cli -- --demo` draws an accessible TUI from deterministic mock data and exits/restores terminal cleanly.
- `lidash --version`, `lidash --help`, and `lidash doctor` report capabilities without privilege escalation; `doctor` never runs invasive tests unless flagged.
- There is at least one integration test for missing GPU APIs and one for inaccessible eBPF.
- No uncontrolled background daemon installation. Running scripts requires documented user intent.

### Suggested baseline tools (validate versions at implementation time)

Rust workspace + Cargo, Tokio, Ratatui/Crossterm, tracing, serde and typed error handling, netlink family library/API, procfs/sysfs access, SQLite optional, protobuf if needed; Clang/LLVM/libbpf/libbpf-rs/libbpf-cargo for optional C eBPF; NVIDIA NVML/DCGM only when detected and permitted; typed collector adapters for vLLM/Triton/NCCL. Avoid unnecessary dependencies or a Python/FastAPI requirement for the core architecture.

## 22. Decisions / alternatives register

| Decision | Chosen | Rejected / postponed | Why |
|---|---|---|---|
| UI | Rust Ratatui | Electron/web dashboard | native SSH, smaller deployment footprint |
| Core | Rust | C-only entire app, Python-only collectors | safety + manageable async + C only where justified |
| BPF | C CO-RE/libbpf-rs optional | obligatory kernel probe or XDP dependency | hardware/kernel compatibility, safe fallback |
| I/O | Tokio/epoll | io_uring by default | benchmark first, complexity control |
| GPU | conditional NVML/DCGM | mandatory GPU runtime dependency | useful CPU-only network monitor |
| Spark memory | unified DRAM/PSI model | unsupported `VRAM %` | follows vendor limitation |
| Spark interconnect | ConnectX-7 Ethernet/RoCE | Spark-to-Spark NVLink | correct real hardware topology |
| Distributed insight | evidence + advice | automatic rescheduling in MVP | prevent operational surprises |
| Storage | ring + optional SQLite | mandatory external database | local-first and low setup |
| Transport | UDS + secure optional peer | public HTTP endpoint | least privilege and minimal attack surface |
| Diagnosis | auditable heuristics | generative guess as source of truth | explainability and precision |

## 23. Research bibliography (primary sources first)

Accessed for this proposal in October 2026. Sources are linked for an engineering agent to verify **live versions** before implementation. Hardware implementation and performance claims must still be field-tested.

### NVIDIA DGX Spark / hardware topology

- [S01] NVIDIA DGX Spark product and hardware specs: https://www.nvidia.com/en-us/products/workstations/dgx-spark/
- [S02] NVIDIA DGX Spark ConnectX-7 Networking, physical and logical QSFP topology: https://docs.nvidia.com/dgx/dgx-spark/spark-clustering.html
- [S03] NVIDIA Connect Two Sparks playbook: https://build.nvidia.com/spark/connect-two-sparks/overview
- [S04] NVIDIA NCCL for Multiple Sparks (2/3/4 nodes): https://build.nvidia.com/spark/nccl/overview
- [S05] NVIDIA DGX Spark known issues, unified memory and nvidia-smi caveats: https://docs.nvidia.com/dgx/dgx-spark/known-issues.html
- [S06] NVIDIA DGX Dashboard (existing baseline competitor): https://docs.nvidia.com/dgx/dgx-spark/dgx-dashboard.html
- [S07] NVIDIA DCGM profiling, NVLink per-link support and limitations: https://docs.nvidia.com/datacenter/dcgm/latest/learn/modules/profiling.html
- [S30] NVIDIA official CPU↔GPU NVLink-C2C description for GB10: https://investor.nvidia.com/news/press-release-details/2025/NVIDIA-Puts-Grace-Blackwell-on-Every-Desk-and-at-Every-AI-Developers-Fingertips/default.aspx

### Distributed GPU runtime, collective communications and networking

- [S08] NVIDIA NCCL overview and collectives: https://docs.nvidia.com/deeplearning/nccl/user-guide/docs/overview.html
- [S09] NCCL RAS and outlier/hang diagnostics: https://docs.nvidia.com/deeplearning/nccl/user-guide/docs/troubleshooting/ras.html
- [S10] NCCL network/RoCE troubleshooting: https://docs.nvidia.com/deeplearning/nccl/user-guide/docs/troubleshooting/networking_troubleshooting.html
- [S11] NCCL environment variables / interface selection: https://docs.nvidia.com/deeplearning/nccl/user-guide/docs/env.html
- [S12] vLLM metrics architecture: https://docs.vllm.ai/en/latest/design/metrics/
- [S13] Triton Inference Server metrics: https://docs.nvidia.com/deeplearning/triton-inference-server/user-guide/docs/user_guide/metrics.html
- [S14] Triton inference trace timing: https://docs.nvidia.com/deeplearning/triton-inference-server/user-guide/docs/user_guide/trace.html
- [S15] NVIDIA MLNX_OFED Ethernet/RoCE counters (check current driver variants): https://docs.nvidia.com/networking/display/mlnxofedv543400/ethernet%2Binterface

### Linux kernel, observability and architecture

- [S16] Linux netlink family specs (route/nftables/ethtool/devlink/conntrack): https://docs.kernel.org/netlink/specs/index.html
- [S17] Linux PSI (CPU/memory/IO pressure stalls): https://docs.kernel.org/accounting/psi.html
- [S18] Linux cgroup v2 memory/pressure accounting: https://docs.kernel.org/admin-guide/cgroup-v2.html
- [S19] Linux AF_XDP semantics and copy vs zero-copy: https://docs.kernel.org/networking/af_xdp.html
- [S20] Rust libbpf-rs / libbpf-cargo: https://github.com/libbpf/libbpf-rs
- [S21] BPF CO-RE bootstrap/examples: https://github.com/libbpf/libbpf-bootstrap
- [S22] Linux kernel ethtool Netlink: https://docs.kernel.org/networking/ethtool-netlink.html

### Similar products / reference implementations

- [S23] bandwhich, per-process terminal bandwidth: https://github.com/imsnif/bandwhich
- [S24] termshark, terminal packet analysis: https://github.com/gcla/termshark
- [S25] ntopng, flow/network analysis: https://github.com/ntop/ntopng
- [S26] pwru, kernel packet-path debugger: https://github.com/cilium/pwru
- [S27] BCC observability tooling and tutorials: https://github.com/iovisor/bcc
- [S28] Ratatui Rust terminal UI: https://github.com/ratatui/ratatui
- [S29] tokio-uring design and considerations: https://github.com/tokio-rs/tokio-uring/blob/master/DESIGN.md

## 24. Final architecture verdict

**Approve design, conditional on incremental implementation and hardware validation.** The architectural advantage is not an unsubstantiated claim of superior generic dashboard performance; it is an integrated, explainable **AI workload + kernel network + inter-node fabric diagnostic workflow** that generic single-domain tools commonly do not deliver as one SSH-native experience. DGX Spark support is a flagship adapter; the platform remains useful on conventional VPS and Linux AI GPU servers.

**Highest priority for an agent:** P0–P1 functional vertical slice, capability-safe GPU abstraction, source/provenance discipline, then Spark port topology + UMA and distributed NCCL/RDMA correlation. Do not begin with speculative automatic load balancing or a massive custom tracing engine.