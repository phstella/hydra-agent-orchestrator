# Hydra Progress Tracker

Last updated: 2026-02-23

## Current State

- **Phase**: 0 complete; ready for Phase 1
- **Milestone**: All Phase 0 milestones (M0.1-M0.8) complete
- **Sprint**: Sprint 1 (8/10 tickets done — remaining: M1.1, M1.2)
- **Status**: Adapter probes, artifact convention, security baseline, and doctor command all implemented

## Completed Milestones

| ID | Title | Date | Notes |
|----|-------|------|-------|
| M0.1 | Adapter Probe Framework | 2026-02-23 | `AgentAdapter` trait, `ProbeRunner`, `ProbeReport`, `DetectResult`, `CapabilitySet`, error taxonomy. 6 unit tests. |
| M0.2 | Claude Probe Implementation | 2026-02-23 | Parses --help for -p, --output-format, --permission-mode. Fixture-based tests. |
| M0.3 | Codex Probe Implementation | 2026-02-23 | Parses exec subcommand, --json, approval/sandbox flags. Fixture-based tests. |
| M0.4 | Cursor Experimental Probe | 2026-02-23 | Always experimental tier. Status: experimental-ready/blocked/missing. Observed confidence. |
| M0.5 | Run Artifact Convention | 2026-02-23 | `RunLayout`, `RunManifest` (schema_version=1), `EventWriter`/`EventReader` for JSONL. 15 unit tests. |
| M0.6 | Doctor Command MVP | 2026-02-23 | `hydra doctor` with adapter probes + git checks. Human and JSON output. Non-zero exit on failure. |
| M0.7 | Security Baseline Implementation | 2026-02-23 | `SecretRedactor` (13 patterns + custom), `SandboxPolicy` (strict/unsafe). 19 unit tests. |
| M0.8 | Architecture Decision Lock | 2026-02-23 | ADR 6 (process model) and ADR 7 (storage model) confirmed in architecture.md. |

## In-Progress Work

(none — Phase 0 complete)

## Decisions Made

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-23 | Restructured repo: `research/` split into `docs/` and `planning/` | Separate reference docs from project management; add agent entry points |
| 2026-02-23 | Used `which` crate for binary resolution in probes | Cross-platform PATH lookup without reinventing |
| 2026-02-23 | `CapabilityEntry` pairs `supported: bool` with `confidence` tag | Matches docs/agent-adapters.md confidence model (verified/observed/unknown) |
| 2026-02-23 | `RunManifest` includes `schema_version: 1` from day one | Forward-compatibility per ADR 7; supports future migration (M5.6) |
| 2026-02-23 | `resolve_binary` does not fall back to PATH when configured path is set but missing | Explicit config takes precedence; prevents unexpected binary resolution |
| 2026-02-23 | M0.8 satisfied by existing docs/architecture.md content | ADR 6 and 7 were already documented during planning phase |

## Open Issues

- `which` v7 pinned; v8 available but not yet evaluated.
- Doctor command discovers real adapters on PATH but probe tests use fixture-only approach.

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 61 passing |
| hydra-cli | Yes | Yes | 0 (functional tests via `cargo run`) |
| hydra-app | No | - | - |

## Phase Progress

| Phase | Name | Status | Milestones Done |
|-------|------|--------|-----------------|
| 0 | Validation and Guardrails | **Complete** | 8/8 |
| 1 | Core Orchestrator + Single Agent | Not started | 0/8 |
| 2 | Multi-Agent Race + Scoring | Not started | 0/12 |
| 3 | GUI Alpha | Not started | 0/7 |
| 4 | Collaboration Workflows | Not started | 0/6 |
| 5 | Windows Parity + Hardening | Not started | 0/6 |

## Instructions for Next Agent

1. Read `CLAUDE.md` for project overview and conventions.
2. Phase 0 is **complete**. All 8 milestones done with 61 passing tests.
3. **Next**: Start Phase 1 per `instructions/phase-1.md` (if exists) or `planning/sprint-1-cut.md`.
4. Sprint 1 remaining tickets: `M1.1` (Core Workspace Scaffold) and `M1.2` (Config Parser and Defaults).
5. `M1.1` depends on M0.8 (done). `M1.2` depends on M1.1.
6. Key files to know:
   - Adapter framework: `crates/hydra-core/src/adapter/`
   - Artifact convention: `crates/hydra-core/src/artifact/`
   - Security: `crates/hydra-core/src/security/`
   - CLI entry: `crates/hydra-cli/src/main.rs`
   - Doctor command: `crates/hydra-cli/src/doctor.rs`
7. Test fixtures: `crates/hydra-core/tests/fixtures/adapters/{claude,codex,cursor}/`
