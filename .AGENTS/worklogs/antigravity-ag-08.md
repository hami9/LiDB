# Task AG-08 — Phase 4: Optional read-only eBPF instrumentation and unprivileged fallback

- Task ID: `AG-08`
- Owner: `agent/antigravity/ag-08-ebpf`
- Worktree: `.worktrees/antigravity/ag-08-ebpf`
- Branch: `agent/antigravity/ag-08-ebpf`
- Base commit: `e70d6d8` (`origin/main`)
- Scope: `crates/lidb-core/src/ebpf.rs`, `crates/lidb-collect/bpf/tcp_lifecycle.bpf.c`, `crates/lidb-collect/src/ebpf.rs`, `apps/lidash/src/config.rs`, `apps/lidash/src/lib.rs`, `apps/lidash/src/output.rs`, `apps/lidash/tests/cli.rs`, `docs/DECISIONS.md`, `docs/CORE.md`, `crates/lidb-collect/README.md`, `apps/lidash/README.md`
- Status: `ready_for_review`

## Acceptance criteria

- Pure platform-agnostic eBPF contracts in `crates/lidb-core`:
  - `EbpfEventType`, `EbpfLifecycleEvent`, `EbpfReport`, `EbpfError`.
  - Fixed capacity bound `MAX_EBPF_EVENTS = 2048` and `dropped_events` tracking.
  - Serialization to JSON schema `0.1`.
- Safe C CO-RE program:
  - `crates/lidb-collect/bpf/tcp_lifecycle.bpf.c` hooking tracepoint `sock:inet_sock_set_state`.
  - 256KB ringbuf event delivery.
  - Metadata-only extraction: socket endpoints, family, state transitions, duration.
- Userspace collector & fallback:
  - `LinuxCollector::ebpf` and `lidb_collect::ebpf::collect_ebpf`.
  - Capability probing for Linux BTF (`/sys/kernel/btf/vmlinux`) and `CAP_BPF`/`CAP_SYS_ADMIN`.
  - Clean typed fallback: `SourceAvailability::unsupported` (missing BTF / non-Linux) and `SourceAvailability::permission_denied` (unprivileged).
  - Deterministic fixture replay for CI and non-Linux test environments.
- CLI Integration in `apps/lidash`:
  - `Command::Ebpf` with aliases `ebpf`, `bpf`, `events`.
  - `--ebpf` flag and `--json` export support.
  - Formatted text output (`output::ebpf_text`).
- Documentation & ADR:
  - ADR-031 in `docs/DECISIONS.md`.
  - Updated `docs/CORE.md`, `crates/lidb-collect/README.md`, and `apps/lidash/README.md`.

## Verification

- `cargo fmt --workspace --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo test --workspace --locked --all-targets`: PASS (114 tests passed across workspace)
- `python scripts/check_docs.py`: PASS
- `python -m unittest discover -s tests/ci -p "test_*.py" -v`: PASS
