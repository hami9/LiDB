# Task AG-10 — Phase 5: Public release hardening, standalone installer, and packaging documentation

- Task ID: `AG-10`
- Owner: `agent/antigravity/ag-10-hardening`
- Worktree: `.worktrees/antigravity/ag-10-hardening`
- Branch: `agent/antigravity/ag-10-hardening`
- Base commit: `e9ae7ae` (`origin/main`)
- Scope: `scripts/install.sh`, `tests/ci/test_install_script.py`, `docs/PACKAGING.md`, `docs/SUPPORT_MATRIX.md`, `docs/THIRD_PARTY_LICENSES.md`, `docs/DECISIONS.md`, `README.md`
- Status: `ready_for_review`

## Acceptance criteria

- Standalone installer and uninstaller script:
  - `scripts/install.sh` POSIX-compliant, supports CPU architecture detection (`x86_64` and `aarch64`), SHA-256 cryptographic checksum verification, unprivileged user install (`~/.local/bin`) or system install (`/usr/local/bin`), `--uninstall`, `--tag`, and `--dry-run`.
  - Integration tests in `tests/ci/test_install_script.py` validating options, usage text, and dry-run output.
- Packaging & installation documentation:
  - `docs/PACKAGING.md` detailing standalone binaries, installer script usage, manual download and checksum verification, system requirements, and clean removal.
- Platform support matrix:
  - `docs/SUPPORT_MATRIX.md` defining hardware architecture tiers (`x86_64` and `aarch64` Tier 1), kernel compatibility matrix (from Linux 2.6+ through Linux 4.16+ eBPF), container/virtualization environments, and resource envelopes (< 15 MB RSS, < 0.5% CPU, 0 disk/network writes).
- Software Bill of Materials & license attribution:
  - `docs/THIRD_PARTY_LICENSES.md` auditing all 22 compiled dependencies and confirming 100% license compliance with permissive licenses (MIT, Apache-2.0, BSD-3-Clause).
- Decision Record:
  - ADR-032 in `docs/DECISIONS.md`.
- Closed invalid obsolete Dependabot pull requests (#41, #38, #12, #5).

## Verification

- `cargo fmt --workspace --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS
- `cargo test --workspace --locked --all-targets`: PASS (114 tests)
- `python scripts/check_docs.py`: PASS (157 local Markdown links checked)
- `python -m unittest discover -s tests/ci -p "test_*.py" -v`: PASS (22 tests)
