# CI/CD, release versioning and operational policy

**Status:** the runnable Linux Core workspace is reviewed through PR CI. Binary publishing requires the complete CI gate and actual `lidash` binaries; no release is published by this development task.

## Workflows

| Workflow | Trigger | Responsibilities |
| --- | --- | --- |
| [CI](../.github/workflows/ci.yml) | PR, main push, merge queue, manual, reusable call | Docs/link checks; Python policy tests; PR naming; required Linux x86_64 and aarch64 Rust format, clippy, unit/integration/doc tests and live CLI smoke; stable `CI Gate` |
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
8. Rust is pinned to 1.85.0 in `rust-toolchain.toml` and build workflows. The release job supplies `LIDB_BUILD_VERSION`, so `lidash --version` reports the planned release version. Packaging includes only the current `lidash` binary from a fresh staging directory, preventing obsolete helper binaries from entering an archive.
9. Agent tasks must keep worklogs and disclose tests not run. Cross-architecture compilation is distinct from validating every Linux/kernel/container configuration.

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
cargo run -p lidash --locked -- snapshot --json
cargo run -p lidash --locked -- doctor --json
```

The `plan` command requires Git history/tags. Publishing a release requires GitHub Actions, configured permissions and passing CI. Do not copy production tokens into local config or the repository.

## Remaining open-source release hardening

Before a supported stable release, implement: supply chain and dependency scanning beyond baseline CodeQL/Dependabot; independently verified attestation/provenance policy; SBOM; reproducible environment; security advisories; supported Linux/kernel/container matrix; and installation/uninstall smoke tests. These remain explicitly incomplete.
