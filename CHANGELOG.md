# Changelog

All notable changes will be documented here, following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning when releases begin.

## [Unreleased]

### Linux Core

- Add versioned host snapshots with exact counters, source timestamps, explicit availability and bounded memory.
- Preserve and integrate the existing capability, telemetry, protocol negotiation and history contracts from PRs #9 and #20.
- Add read-only Linux CPU, memory, load, uptime, network, disk and pressure collection, with CLI/JSON, doctor and a terminal dashboard.
- Focus the product on general Linux diagnostics; remove AI/vendor/cluster requirements and unused daemon scaffolding.
- Require native Linux x86_64/aarch64 Core builds, tests and CLI smoke in CI; pin Rust 1.85.0 and embed the release version in packaged binaries.

### Added
- Per-interface RX/TX error and drop counters from `/proc/net/dev`, with exact cumulative values and existing source failure states.
- English open-source architecture, product requirements and research blueprint.
- Agent collaboration contracts, phase gates, review and worklog templates.
- Security, governance, feature-capability, test and integration specifications.

> No runnable software has been released yet.

### Build and automation (unreleased)

- Added CI Gate, Linux x86_64/aarch64 checks, CodeQL and dependency-update configuration.
- Added SemVer release planner, regression tests and gated GitHub Releases with checksummed architecture-specific binaries (first published release still pending).
- Added proposed branch/tag rulesets and documented administrator activation procedure; rules are **not yet enforced** by committing them alone.

### Developer collaboration (unreleased)

- Added local multi-agent Git Worktree manager with safe create/list/remove/doctor actions for Windows and Linux.
- Added Claude Code and Antigravity-specific instruction entry points and centralized ownership rules.
- Added integration tests for isolated branches, clean removal, invalid paths and cross-platform Git operation.
