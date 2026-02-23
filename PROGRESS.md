# Hydra Progress Tracker

Last updated: 2026-02-23 (session 3)

## Current State

- **Phase**: 5 (Windows Parity + Hardening) -- COMPLETE
- **Milestone**: All 47 milestones complete
- **Sprint**: All sprints complete
- **Status**: Release candidate ready. 368 tests passing.

## Completed Milestones

### Phase 0: Validation and Guardrails (8/8)
- **M0.1**: Adapter Probe Framework
- **M0.2**: Claude Probe Implementation
- **M0.3**: Codex Probe Implementation
- **M0.4**: Cursor Experimental Probe
- **M0.5**: Run Artifact Convention
- **M0.6**: Doctor Command MVP
- **M0.7**: Security Baseline Implementation
- **M0.8**: Architecture Decision Lock

### Phase 1: Core Orchestrator + Single Agent (8/8)
- **M1.1**: Core Workspace Scaffold
- **M1.2**: Config Parser and Defaults
- **M1.3**: Worktree Lifecycle Service
- **M1.4**: Process Supervisor (Single Agent)
- **M1.5**: Claude Adapter Runtime Path
- **M1.6**: Codex Adapter Runtime Path
- **M1.7**: CLI Race Command (Single Agent)
- **M1.8**: Interrupt and Recovery Tests

### Phase 2: Multi-Agent Race + Scoring (12/12)
- **M2.1**: Adapter Registry and Tier Policy
- **M2.2**: Parallel Spawn and Supervision
- **M2.3**: Baseline Capture Engine
- **M2.4-M2.6**: Scoring Dimensions (Build, Tests, Lint, Diff Scope, Speed)
- **M2.7**: Composite Ranking and Mergeability Gates
- **M2.8**: CLI Merge Command with Dry-Run
- **M2.9**: Experimental Cursor Opt-In Path
- **M2.10**: End-to-End Race Integration Test
- **M2.11**: Cost and Budget Engine
- **M2.12**: Observability Contract

### Phase 3: GUI Alpha (7/7)
- **M3.1**: Tauri v2 App Bootstrap
- **M3.2**: IPC Command Surface
- **M3.3-M3.6**: React Frontend Components
- **M3.7**: Smoke Tests and Build Verification

### Phase 4: Collaboration Workflows (6/6)
- **M4.1**: Workflow Engine Core
- **M4.2-M4.4**: Workflow Presets (Builder-Reviewer, Specialization, Iterative)
- **M4.5**: Workflow CLI Subcommand
- **M4.6**: Integration Tests

### Phase 5: Windows Parity + Release Hardening (6/6)
- **M5.1**: ConPTY and Process Control Validation — Platform module with cross-platform process termination (SIGTERM/SIGKILL on Unix, taskkill on Windows), orphan detection, and cleanup
- **M5.2**: Path and Filesystem Edge Cases — Path normalization (Windows long paths), artifact path validation, safe_write with retry-on-lock, Unicode safety checks
- **M5.3**: Crash Recovery and Resume Metadata — RecoveryService for scanning stale runs, cleaning up interrupted state, and writing recovery checkpoints
- **M5.4**: Packaging and Release Automation — GitHub Actions CI (fmt, clippy, test) and Release (multi-platform build) workflows
- **M5.5**: Release Candidate Acceptance Suite — 17 integration tests covering platform detection, doctor reports, artifact integrity, scoring determinism, config profiles, workflow execution, merge reports, schema migration, recovery, and path validation
- **M5.6**: Artifact and Schema Migration — SchemaVersion parsing and compatibility, MigrationTool for upgrading artifact manifests between schema versions

## In-Progress Work

(none -- all phases complete)

## Decisions Made

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-23 | Restructured repo: `research/` split into `docs/` and `planning/` | Separate reference docs from project management; add agent entry points |
| 2026-02-23 | Platform-specific code uses `#[cfg(unix)]`/`#[cfg(windows)]` | Standard Rust conditional compilation for cross-platform support |
| 2026-02-23 | Recovery service reads manifest status to detect stale runs | Simple heuristic: manifest.status == Running with no active process indicates crash |
| 2026-02-23 | Schema migration is manifest-version-based (semver) | Forward-compatible reads within same major version |

## Open Issues

(none)

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 271 unit + 17 acceptance + 48 adapter + 6 interrupt + 24 workflow = 366 |
| hydra-cli | Yes | Yes | 0 (CLI binary, tested via integration) |
| hydra-app | Yes | Yes | 2 |

## Phase Progress

| Phase | Name | Status | Milestones Done |
|-------|------|--------|-----------------|
| 0 | Validation and Guardrails | Complete | 8/8 |
| 1 | Core Orchestrator + Single Agent | Complete | 8/8 |
| 2 | Multi-Agent Race + Scoring | Complete | 12/12 |
| 3 | GUI Alpha | Complete | 7/7 |
| 4 | Collaboration Workflows | Complete | 6/6 |
| 5 | Windows Parity + Hardening | Complete | 6/6 |

## Instructions for Next Agent

All implementation milestones are complete. Next steps:
1. Manual testing on Windows to verify platform-specific code paths.
2. Create release tags and verify CI/CD pipeline.
3. Write user-facing documentation and README.
