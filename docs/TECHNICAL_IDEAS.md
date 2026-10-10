# General Linux product backlog

These are candidate improvements, not implemented capability claims. Prefer a small user-visible slice with reproducible evidence over speculative subsystems.

| Idea | Operator benefit | Required gate |
| --- | --- | --- |
| Process/socket/route view | Explain service connectivity from observed Linux metadata | Namespace identity, permission fallback and process privacy |
| Evidence-linked pressure view | Compare CPU, memory, I/O and interface symptoms | Shared local time window and explicit causality limits |
| Counter delta/rate view | Compare activity between compatible samples | Reset/wrap, first-sample and timing tests |
| Bounded local replay | Compare before/after observations offline | Retention/payload bounds and schema compatibility |
| Safe support snapshot | Share reproducible host evidence | Explicit export, minimal metadata and redaction review |
| Accessible terminal views | Operate over SSH and small terminals | Keyboard, no-color, narrow-terminal and restore tests |
| Collector cost budget | Avoid obscuring the workload being inspected | Measured read/refresh cost and truthful throttling |
| Optional eBPF evidence | Observe a specific Linux event missing from procfs | Opt-in scope, verifier tests, permission fallback and overhead |

No vendor-specific runtime, distributed placement, automatic host changes or external telemetry service is planned. A metric spike is an observation, not proof of its cause. Any diagnosis must show supporting evidence, alternative explanations and missing data.
