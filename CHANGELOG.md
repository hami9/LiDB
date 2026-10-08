# Changelog

All notable changes will be documented here, following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and Semantic Versioning when releases begin.

## [Unreleased]

### Added
- English open-source architecture, product requirements and research blueprint.
- Agent collaboration contracts, phase gates, review and worklog templates.
- Security, governance, feature-capability, test and integration specifications.

> No runnable software has been released yet.

### Build and automation (unreleased)

- Required standalone TUI checks on Linux x86_64/aarch64, fail-closed job aggregation, documentation tests and standalone Rust detection for CodeQL.
- Added CI Gate, Linux x86_64/aarch64 checks, CodeQL and dependency-update configuration.
- Added SemVer release planner, regression tests and gated GitHub Releases with checksummed architecture-specific binaries (pending executable implementation).
- Added proposed branch/tag rulesets and documented administrator activation procedure; rules are **not yet enforced** by committing them alone.

### Developer collaboration (unreleased)

- Added local multi-agent Git Worktree manager with safe create/list/remove/doctor actions for Windows and Linux.
- Added Claude Code and Antigravity-specific instruction entry points and centralized ownership rules.
- Added integration tests for isolated branches, clean removal, invalid paths and cross-platform Git operation.
