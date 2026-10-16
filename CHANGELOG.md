# Changelog

All notable changes will be documented here, following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning when releases begin.

## [Unreleased]

### Interactive collector lifecycle

- Restore the terminal before collector cleanup; return an explicit timeout after a 100ms shutdown grace rather than waiting indefinitely for a blocked sample.
- Reject interactive collection batches taking 5 seconds, including while paused; discard late results and exit 2 with an explanatory error.
- Prevent replacement workers after a shutdown timeout, and test blocked sampling, destructor cleanup, publication/admission races and real PTY quit/signal/watchdog paths.

### Core review and persistent agent delivery

- Document the maintainer's integrated Linux arm64 no-follow fix and pre-stable bootstrap removal; preserve the separately owned regression work in PR #27.
- Add the maintainer-supplied shared github-style skill, safe local user-scope installation and native agent entry points.
- Add an always-on Antigravity continuation rule and ordered task queue with ownership, dependencies, acceptance checks and session checkpoints.
- Record the actual Core architecture review, boundary fixes and remaining lifecycle/release limitations.

## [0.2.0] — 2026-10-10 (prerelease)

- Add the CPU/Memory overview tab and keyboard tab navigation in the public terminal app (PR #34).
- Publish architecture-specific Linux prerelease archives and checksums through the existing release workflow.

## [0.1.0] — 2026-10-10 (prerelease)

### Linux Core

- Add versioned host snapshots with exact counters, source timestamps, explicit availability and bounded memory.
- Preserve and integrate the existing capability, telemetry, protocol negotiation and history contracts from PRs #9 and #20.
- Add read-only Linux CPU, memory, load, uptime, network, disk and pressure collection, with CLI/JSON, doctor and a terminal dashboard.
- Focus the product on general Linux diagnostics; remove AI/vendor/cluster requirements and unused daemon scaffolding.
- Require native Linux x86_64/aarch64 Core builds, tests and CLI smoke in CI; pin Rust 1.85.0 and embed the release version in packaged binaries.

### Added
- English open-source architecture, product requirements and research blueprint.
- Agent collaboration contracts, phase gates, review and worklog templates.
- Security, governance, feature-capability, test and integration specifications.

> The first runnable prerelease was published; stable compatibility, performance and broader support commitments remain under development.

### Build and automation

- Added CI Gate, Linux x86_64/aarch64 checks, CodeQL and dependency-update configuration.
- Added SemVer release planner, regression tests and gated GitHub Releases with checksummed architecture-specific binaries (prereleases now published).
- Added proposed branch/tag rulesets and documented administrator activation procedure; rules are **not yet enforced** by committing them alone.

### Developer collaboration

- Added local multi-agent Git Worktree manager with safe create/list/remove/doctor actions for Windows and Linux.
- Added Claude Code and Antigravity-specific instruction entry points and centralized ownership rules.
- Added integration tests for isolated branches, clean removal, invalid paths and cross-platform Git operation.
