# Hydra Progress Tracker

Last updated: 2026-02-23

## Current State

- **Phase**: 0 (Validation and Guardrails)
- **Milestone**: M0.1 and M0.5 complete; ready for M0.2, M0.3, M0.4 (Lane A) and M0.7 (Lane B)
- **Sprint**: Sprint 1 (10 tickets)
- **Status**: Workspace scaffold created, probe framework and artifact convention implemented

## Completed Milestones

| ID | Title | Date | Notes |
|----|-------|------|-------|
| M0.1 | Adapter Probe Framework | 2026-02-23 | `AgentAdapter` trait, `ProbeRunner`, `ProbeReport`, `DetectResult`, `CapabilitySet`, error taxonomy. 6 unit tests. |
| M0.5 | Run Artifact Convention | 2026-02-23 | `RunLayout` for deterministic paths, `RunManifest` with schema_version, `EventWriter`/`EventReader` for JSONL. 15 unit tests. |

## In-Progress Work

(none yet — next: M0.2, M0.3, M0.4 in parallel on Lane A; M0.7 on Lane B)

## Decisions Made

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-23 | Restructured repo: `research/` split into `docs/` and `planning/` | Separate reference docs from project management; add agent entry points |
| 2026-02-23 | Used `which` crate for binary resolution in probes | Cross-platform PATH lookup without reinventing |
| 2026-02-23 | `CapabilityEntry` pairs `supported: bool` with `confidence` tag | Matches docs/agent-adapters.md confidence model (verified/observed/unknown) |
| 2026-02-23 | `RunManifest` includes `schema_version: 1` from day one | Forward-compatibility per ADR 7; supports future migration (M5.6) |

## Open Issues

- CLI `doctor` command works but has no real adapter registrations yet (needs M0.2-M0.4).
- `which` v7 pinned; v8 available but not yet evaluated.

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 21 passing |
| hydra-cli | Yes | Yes | 0 (stub) |
| hydra-app | No | - | - |

## Phase Progress

| Phase | Name | Status | Milestones Done |
|-------|------|--------|-----------------|
| 0 | Validation and Guardrails | In progress | 2/8 |
| 1 | Core Orchestrator + Single Agent | Not started | 0/8 |
| 2 | Multi-Agent Race + Scoring | Not started | 0/12 |
| 3 | GUI Alpha | Not started | 0/7 |
| 4 | Collaboration Workflows | Not started | 0/6 |
| 5 | Windows Parity + Hardening | Not started | 0/6 |

## Instructions for Next Agent

1. Read `CLAUDE.md` for project overview and conventions.
2. Read `instructions/phase-0.md` for Phase 0 guidance.
3. **Lane A next**: Implement M0.2 (Claude Probe), M0.3 (Codex Probe), M0.4 (Cursor Probe) in parallel. All depend on M0.1 (done). Then M0.6 (Doctor Command MVP).
4. **Lane B next**: Implement M0.7 (Security Baseline). Depends on M0.5 (done).
5. **Lane C**: M0.8 (Architecture Decision Lock) can run anytime — check if ADR 6 and 7 are already in `docs/architecture.md` (they are, so this may be a quick verify+close).
6. Refer to `planning/implementation-checklist.md` for full acceptance criteria.
7. Test fixtures go in `crates/hydra-core/tests/fixtures/adapters/{claude,codex,cursor}/`.
