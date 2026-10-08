---
name: lidb-ebpf-kernel
description: Safe optional C CO-RE eBPF probes and userspace loader.
---
# eBPF kernel skill

**Use when:** adding tracepoints, socket probes, XDP/TC observation or eBPF loader integration.

- eBPF is a separate optional capability; probe kernel support and permissions before use.
- Write concise C CO-RE programs with verifier-safe bounded access; keep userspace event parsing in Rust.
- Use stable hooks and tracepoints when practical; document kernel-specific differences and map ABI.
- Define event size/version, ring buffer bounds, lost events, attach/detach lifecycle and failure fallback.
- Privileged loader/helper must not share process with TUI; no unrequested XDP/TC link changes or packet modification.
- Test verifier and cleanup on supported kernels, measure overhead and document unavailable tests.
- Review kernel capabilities case by case; never assume root is always needed or one capability set suffices.
