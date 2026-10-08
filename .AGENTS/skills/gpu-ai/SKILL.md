---
name: lidb-gpu-ai
description: NVIDIA GPU, DGX Spark GB10 unified memory and inference workload metrics.
---
# GPU and AI skill

**Use when:** GPU device sensing, memory accounting, model-serving adapter or inference RCA.

- Runtime-detect actual NVIDIA API/sensors. No hard link-time dependency for baseline Linux operation.
- GB10 unified-memory semantics differ from discrete VRAM. Mark unreliable/unavailable counters honestly; integrate Linux pressure and process memory evidence.
- CPU↔GPU NVLink-C2C is internal to Spark. Multi-Spark transport is ConnectX networking; never use one label for both.
- Keep inference latency units clear (TTFT, TPOT, end-to-end latency) with precise quantile aggregation and request scope.
- Compare timestamp windows, sampling bias and queue length against GPU/utilization indicators without claiming causality automatically.
- Do not access model inputs, outputs, weights or sensitive request payloads. Endpoint authentication is read-only, scoped and secret-safe.
- Fixture tests are not hardware validation; require real GB10 evidence for Spark support claims.
