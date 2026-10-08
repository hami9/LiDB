# CI/CD, release versioning and operational policy

**Status:** configured in GitHub workflows; software implementation is still P0/not started. Binary release publishing is gated on a working Cargo workspace and a real `lidash` binary.

## Workflows

| Workflow | Trigger | Responsibilities |
| --- | --- | --- |
| [CI](../.github/workflows/ci.yml) | PR, main push, merge queue, manual, reusable call | Docs/link checks; Python policy tests; PR naming; Linux x86_64 and aarch64 root and standalone TUI checks; stable `CI Gate` |
| [Documentation quality](../.github/workflows/docs.yml) | PR, main | Existing standalone docs link check |
| [CodeQL](../.github/workflows/codeql.yml) | PR, main, weekly | Actions analysis, and Rust/C analysis when sources exist |
| [Automatic Release](../.github/workflows/release.yml) | main push, manual dry-run | Re-runs gate, plans SemVer, compiles both architectures, packages, validates checksums and publishes GitHub Release |
| [Dependabot](../.github/dependabot.yml) | weekly | Dependency update PRs for GitHub Actions and Cargo once available |

## Version calculation

The user-facing mental model is `0.0.0 + 1 → 0.0.1` (PATCH), `0.0.0 + 0.1.0 → 0.1.0` (MINOR), `0.0.0 + 1.0.0 → 1.0.0` (MAJOR). **Do not use `0.0.0+1` as a patch version:** in SemVer, `+1` is build metadata and does not change precedence.

The release planner takes the **highest** kind among non-merge commits since the newest reachable `vMAJOR.MINOR.PATCH` release tag:

| PR title / merge commit | Bump | Example from `v0.2.4` |
| --- | --- | --- |
| `fix:`, `docs:`, `chore:`, `ci:`, `perf:`, nonstandard | PATCH | `v0.2.5` |
| `feat:` or `feat(scope):` | MINOR | `v0.3.0` |
| `feat!:`, `refactor(scope)!:`, `BREAKING CHANGE:` footer | MAJOR | `v1.0.0` |
| Explicit `Release-Bump: major\|minor\|patch` commit trailer | Specified override for that commit | As declared |

If multiple changes are merged between releases, the strongest bump wins. **One release per validated main state**, not necessarily one release per PR during a busy merge burst. Squash merges are strongly preferred so the Conventional Commit PR title becomes the release commit subject. No release is created on a PR or a fork. Immutable version tags are never rewritten. There is no live version file that agents must mutate to publish.

## Guardrails

1. Release job runs **only on main**, after a complete reusable CI gate. No untrusted PR code ever receives a publishing token.
2. Release is blocked until `Cargo.toml` exists, x86_64 and aarch64 checks pass, and a real `target/release/lidash` executable is produced on both architectures.
3. Latest-main check prevents outdated queued runs from tagging stale commits. A release is not published if the proposed tag already exists.
4. Both architecture tarballs and their SHA-256 files must be present and pass checksum verification before publication; each archive additionally receives a GitHub build-provenance attestation.
5. GitHub prerelease flag is used for `0.x.y` versions, indicating immature API support.
6. Workflow tokens default to read-only; only the final publishing job has `contents: write`. Never use `pull_request_target` for build-and-execute workflows.
7. Checksums are integrity checks, while GitHub artifact attestations provide cryptographic build provenance, **not a guarantee of binary safety**. Verify released archives with `gh attestation verify <file> -R hami9/LiDB`. Artifact attestations are configured but remain **untested until the first real binary release**. SBOM and reproducible build hardening remain future gates.
8. Agent tasks must keep worklogs and disclose tests not run. No release script can assert that hardware support was validated solely by a successful cross-architecture build.

## Branch protection

The desired rules are version-controlled in [main.json](../.github/rulesets/main.json) and [release-tags.json](../.github/rulesets/release-tags.json). They are NOT enforced merely because the files exist. The admin-only application procedure is in [BRANCH_PROTECTION.md](BRANCH_PROTECTION.md).

## Running local checks

```bash
python3 scripts/check_docs.py
python3 -m unittest discover -s tests/ci -p 'test_*.py' -v
python3 scripts/release/version.py plan
bash scripts/admin/apply_rulesets.sh --dry-run
# Once Cargo.toml and Cargo.lock exist:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --all-targets
cargo test --workspace --locked --doc
# Standalone prototype (independent of the root workspace):
python3 scripts/ci/check_tui.py
cargo fmt --manifest-path prototypes/tui/Cargo.toml --all -- --check
cargo clippy --manifest-path prototypes/tui/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path prototypes/tui/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path prototypes/tui/Cargo.toml --workspace --doc --locked
cargo run --manifest-path prototypes/tui/Cargo.toml --locked -- --headless-test
cargo run --manifest-path prototypes/tui/Cargo.toml --locked -- --smoke-test
```

The `plan` command requires Git history/tags. Publishing a release requires GitHub Actions, configured permissions and passing CI. Do not copy production tokens into local config or the repository.

## Standalone TUI gate

`TUI (x86_64)` and `TUI (aarch64)` run on native Linux runners. They detect `prototypes/tui` independently of the root `Cargo.toml`. An absent directory is reported explicitly; an existing directory without its manifest or lockfile fails. Formatting, locked Clippy, target tests, documentation tests and both non-interactive CLI checks must pass when the prototype exists. These checks validate fixtures and rendering, not live collectors or hardware.

`CI Gate` always aggregates docs, root Rust, Windows worktree and TUI jobs through `scripts/ci/check_gate.py`. Failed, cancelled, skipped, missing or unknown required results block the gate. A docs-only checkout may pass after both Rust jobs explicitly report absent code. CodeQL Rust also detects the standalone TUI manifest. The required check name and release permissions are unchanged.

Rollback is a normal revert of the CI slice through a reviewed PR. No runtime schema, dependency or release intent changes are required. See [P0 integration review](P0_INTEGRATION_REVIEW.md) for the reviewed branch boundaries and remaining gates.

## Remaining open-source release hardening

Before a supported stable release, implement: supply chain and dependency scanning beyond baseline CodeQL/Dependabot; independently verified attestation/provenance policy; SBOM; reproducible environment; security advisories; supported kernel/toolchain matrix; actual DGX/cluster evidence; and installation/uninstall smoke tests. These remain explicitly incomplete.
