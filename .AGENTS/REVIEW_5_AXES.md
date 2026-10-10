# Required five-angle technical review

Use at phase boundaries and when crossing a trust or API boundary. Record concrete findings, tests and unresolved risks. Self-review does not count as independent approval.

## A — Linux and source correctness

Are procfs units, namespaces, kernel optional fields and counter semantics correct? Are reads bounded and unavailable/denied states tested? Future networking must use correct routes, hooks and namespace identities.

## B — Telemetry and contract integrity

Are observed and derived values distinct, with source, units and local clock domain? Are first samples, resets, malformed values and stale/absent observations handled honestly? Do snapshot/history caps and compatibility match their documentation?

## C — Security and privacy

Does the baseline remain non-root, local and read-only? Are hostile source text, terminal escapes, sensitive metadata, dependencies and output privacy addressed? Any new helper/socket/probe/persistence needs explicit separate review.

## D — Performance and reliability

Are source sizes, metric counts and polling bounded? Can partial collection fail without corrupting unrelated evidence? Does terminal cleanup work on exit/error? Are overhead claims measured on a named environment?

## E — Operator experience and compatibility

Are help, errors, text/JSON and keyboard workflows understandable over SSH? Are fixture data and support limits visible? Are narrow/non-TTY behavior, installation/removal and schema changes documented?

## Review record

```text
Change / commit:
Reviewer(s) and independence:
A: PASS | CONCERNS | BLOCK — evidence, action
B: PASS | CONCERNS | BLOCK — evidence, action
C: PASS | CONCERNS | BLOCK — evidence, action
D: PASS | CONCERNS | BLOCK — evidence, action
E: PASS | CONCERNS | BLOCK — evidence, action
Platform / fixture evidence:
Unresolved blockers:
Maintainer decision:
```
