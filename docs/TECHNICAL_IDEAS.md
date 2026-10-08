# Technical ideas and product differentiation

This document is a **prioritized hypothesis backlog**, not a promise that any capability exists today.

| ID | Idea | Operator benefit | Feasibility / dependency | Phase |
| --- | --- | --- | --- | --- |
| F01 | Evidence-linked root cause explorer | See *why* a diagnosis was proposed | Core trace + provenance | P4 |
| F02 | Process → socket → namespace → route map | Answer "why can't this service connect?" | Linux procfs/netlink | P2 |
| F03 | Fault-domain timeline | Align CPU, pressure, NIC, GPU and model symptoms | Monotonic timestamps + event core | P4–8 |
| F04 | GB10 unified-memory pressure panel | Avoid false discrete-VRAM conclusions | /proc, PSI, optional NVIDIA adapter | P5 |
| F05 | AI service queue vs GPU saturation detector | Distinguish batching/queueing from compute bottlenecks | Explicit serving metrics | P6 |
| F06 | Rank-to-node-to-NIC topology | Show incorrect placement or interface selection | Runtime-provided rank map | P7 |
| F07 | ConnectX/RoCE health explanation | Explain inter-node performance degradation | NIC support and permissions | P7 |
| F08 | NCCL collective correlation | Identify probable stragglers | NCCL instrumentation with opt-in | P8 |
| F09 | Multi-node counterfactual advisor | Offer safe placement suggestions, not autonomous scheduling | Validated topology/perf model | P8 |
| F10 | Offline incident replay | Compare before/after of an incident | Typed local event store | P4 |
| F11 | Reproducible diagnostic support bundle | Share redacted evidence without SSH access | Privacy-aware export | P4 |
| F12 | Stable third-party adapter protocol | Community hardware/runtime extensions | Versioned schemas + isolation | P9 |
| F13 | Adaptive collector budget | Avoid impact on loaded GPU machines | Overhead governor and backpressure | P4 |
| F14 | Secure change plan + diff (future) | Inspect configuration interventions | Threat model, explicit approval | after P10 |
| F15 | Accessible/headless CI snapshots | Automated tests and SSH usability | Non-ANSI JSON/text mode | P0–4 |

## Prioritization

**Baseline value first:** F02 and F15 are prerequisites to feature differentiation. **AI differentiators next:** F04–F08, gated by platform validation. **Trust and extensibility always:** F01, F11–F13.

## Design cautions

- A symptom is not a causal proof. Classify findings as observed, correlated, hypothesized or verified.
- Do not force all hardware into "GPU VRAM used %." On unified-memory systems this is misleading.
- NVLink-C2C (on-device) must never be presented as the Spark-to-Spark link.
- One metric spike is not enough to attribute fault to NCCL/RoCE.
- High-frequency tracing, payload capture and runtime instrumentation remain opt-in and budgeted.
- Community adapters must not gain arbitrary host privileges simply by being installed.
