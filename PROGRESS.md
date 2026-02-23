# Hydra Progress Tracker

Last updated: 2026-02-23

## Current State

- **Phase**: 0 complete; ready for Phase 1
- **Milestone**: All Phase 0 milestones (M0.1-M0.8) complete
- **Sprint**: Sprint 1 (8/10 tickets done — remaining: M1.1, M1.2)
- **Status**: Phase 0 hardening pass complete (sandbox + redaction fixes, probe robustness, config-aware doctor)

## Completed Milestones

| ID | Title | Date | Notes |
|----|-------|------|-------|
| M0.1 | Adapter Probe Framework | 2026-02-23 | `AgentAdapter` trait, `ProbeRunner`, `ProbeReport`, `DetectResult`, `CapabilitySet`, error taxonomy. 6 unit tests. |
| M0.2 | Claude Probe Implementation | 2026-02-23 | Parses --help for -p, --output-format, --permission-mode. Fixture-based tests. |
| M0.3 | Codex Probe Implementation | 2026-02-23 | Parses exec subcommand, --json, approval/sandbox flags. Fixture-based tests. |
| M0.4 | Cursor Experimental Probe | 2026-02-23 | Always experimental tier. Status: experimental-ready/blocked/missing. Observed confidence. |
| M0.5 | Run Artifact Convention | 2026-02-23 | `RunLayout`, `RunManifest` (schema_version=1), `EventWriter`/`EventReader` for JSONL. 15 unit tests. |
| M0.6 | Doctor Command MVP | 2026-02-23 | `hydra doctor` with adapter probes + git checks. Human and JSON output. Non-zero exit on failure. |
| M0.7 | Security Baseline Implementation | 2026-02-23 | `SecretRedactor` (13 patterns + custom), `SandboxPolicy` (strict/unsafe). Hardened with multi-match redaction and path-normalized sandbox checks. |
| M0.8 | Architecture Decision Lock | 2026-02-23 | ADR 6 (process model) and ADR 7 (storage model) confirmed in architecture.md. |

## In-Progress Work

(none — hardening complete; ready for Phase 1)

## Decisions Made

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-23 | Restructured repo: `research/` split into `docs/` and `planning/` | Separate reference docs from project management; add agent entry points |
| 2026-02-23 | Used `which` crate for binary resolution in probes | Cross-platform PATH lookup without reinventing |
| 2026-02-23 | `CapabilityEntry` pairs `supported: bool` with `confidence` tag | Matches docs/agent-adapters.md confidence model (verified/observed/unknown) |
| 2026-02-23 | `RunManifest` includes `schema_version: 1` from day one | Forward-compatibility per ADR 7; supports future migration (M5.6) |
| 2026-02-23 | `resolve_binary` does not fall back to PATH when configured path is set but missing | Explicit config takes precedence; prevents unexpected binary resolution |
| 2026-02-23 | M0.8 satisfied by existing docs/architecture.md content | ADR 6 and 7 were already documented during planning phase |
| 2026-02-23 | Shared adapter version parser extracted to `adapter/mod.rs` | Removes duplication across Claude/Codex/Cursor probes and centralizes version parsing behavior |
| 2026-02-23 | Adapter help probes now require successful exit status | Prevents false-positive readiness when `--help`/`exec --help` exits non-zero |
| 2026-02-23 | Sandbox strict-mode fallback now normalizes absolute paths and components | Closes prefix-based bypass for non-existent paths (`worktree` vs `worktree-evil`, `..` traversal) |
| 2026-02-23 | Secret redaction now handles multiple occurrences per line | Prevents leakage when the same token prefix appears multiple times on one log line |
| 2026-02-23 | `hydra doctor` reads optional adapter path overrides from `hydra.toml` | Enables configured binary paths before full config parser milestone (M1.2) lands |
| 2026-02-23 | Reduced Tokio feature sets in core/cli crates | Keeps runtime surface lean while preserving required async/runtime capabilities |

## Open Issues

- `which` v7 pinned; v8 available but not yet evaluated.
- `hydra.toml` parsing in doctor is intentionally lightweight and local (full typed config parser deferred to M1.2).

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 65 passing |
| hydra-cli | Yes | Yes | 4 passing |
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
2. Phase 0 is **complete** and hardening fixes from `ANALYSIS.md` are implemented.
3. Current test baseline: `hydra-core` 65 passing, `hydra-cli` 4 passing.
4. **Next**: Start Phase 1 per `instructions/phase-1.md` (if exists) or `planning/sprint-1-cut.md`.
5. Sprint 1 remaining tickets: `M1.1` (Core Workspace Scaffold) and `M1.2` (Config Parser and Defaults).
6. `M1.1` depends on M0.8 (done). `M1.2` depends on M1.1.
7. Hardening-related files:
   - Shared adapter version parser: `crates/hydra-core/src/adapter/mod.rs`
   - Probe exit-status checks: `crates/hydra-core/src/adapter/{claude,codex,cursor}.rs`
   - Redaction multi-match logic: `crates/hydra-core/src/security/redact.rs`
   - Sandbox normalized path enforcement: `crates/hydra-core/src/security/sandbox.rs`
   - Doctor path overrides (`hydra.toml`): `crates/hydra-cli/src/doctor.rs`, `crates/hydra-cli/src/main.rs`
8. Key files to know:
   - Adapter framework: `crates/hydra-core/src/adapter/`
   - Artifact convention: `crates/hydra-core/src/artifact/`
   - Security: `crates/hydra-core/src/security/`
   - CLI entry: `crates/hydra-cli/src/main.rs`
   - Doctor command: `crates/hydra-cli/src/doctor.rs`
9. Test fixtures: `crates/hydra-core/tests/fixtures/adapters/{claude,codex,cursor}/`
