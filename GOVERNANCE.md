# Open-source project governance

LiDashBoard is currently maintainer-led. Contributors propose and review changes through issues and pull requests. The repository owner has final merge and release authority while the project is in bootstrap.

## Roles

- **Maintainers:** approve architecture, security boundaries, release policy, capability stability and merges.
- **Reviewers:** provide domain-specific approval; approval does not imply hardware validation.
- **Contributors and agents:** submit evidence-backed changes, tests and documentation. Agents cannot act as maintainers or self-approve sensitive changes.

## Decision process

Material API, privilege, data privacy, licensing and extension changes require an ADR and public review where safe. Breaking stable contracts require a major release and migration notes. Security advisories are coordinated privately. Feature removal and deprecation should be announced in advance.

## Release policy

Before public binaries: reproducible Linux x86_64/aarch64 build verification, testing matrix, changelog, license compliance, dependency audit, SBOM and verifiable release signatures. Releases should use SemVer. Do not imply a guarantee of support before maintainers publish one.

## Community expectations

Technical disagreements should cite evidence and focus on impact. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
