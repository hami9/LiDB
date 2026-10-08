# Required five-angle technical review

Use this review at phase boundaries and for changes that cross trust, device or API boundaries. Record concrete findings, file references, tests and unresolved risks. A self-review does not count as independent approval.

## A — Linux kernel and networking correctness
Questions: Does the code use correct ingress/egress paths, namespace identity, netlink and netfilter semantics? Are kernel/version capabilities probed? Are observability overhead and dropped events visible? Test privileged and missing-permission paths.

## B — AI/GPU and distributed computing semantics
Questions: Is GB10 unified memory modeled honestly? Is on-die NVLink-C2C distinct from multi-node ConnectX? Are advertised vs observed rates separated? Is rank mapping verified? Are inference/NCCL labels well defined and hardware results actually measured?

## C — Security, privacy and abuse resistance
Questions: Are local sockets authenticated, helper privileges minimized, plugins sandboxed, secrets redacted and sensitive collection opt-in? Are input bounds, supply-chain risks, privacy impact and consent explicit?

## D — Performance, reliability and failure behavior
Questions: Will collectors degrade under pressure, keep queues bounded, handle NIC/daemon/GPU failure and retain truthful counters? Does profiling reproduce latency/CPU/memory claims on labeled systems? Is there a rollback/disable path?

## E — User experience, operability and ecosystem
Questions: Does a new operator understand the finding? Is help and keyboard access viable over SSH? Are errors actionable and assumptions visible? Can external adapters integrate through versioned contracts without destabilizing core? Are migration and deprecation clear?

## Review record template

```text
Change / commit:
Reviewer(s) and independence:
A: PASS | CONCERNS | BLOCK — evidence, action
B: PASS | CONCERNS | BLOCK — evidence, action
C: PASS | CONCERNS | BLOCK — evidence, action
D: PASS | CONCERNS | BLOCK — evidence, action
E: PASS | CONCERNS | BLOCK — evidence, action
Hardware validation:
Unresolved blockers:
Maintainer decision:
```
