# Hydra Progress Tracker

Last updated: 2026-02-23

## Current State

- **Phase**: 1 in progress
- **Milestone**: M1.4 complete; M1.5-M1.8 remaining
- **Sprint**: Sprint 1 complete (10/10 tickets done); Phase 1 continues
- **Status**: M1.1 through M1.4 implemented with full test coverage

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
| M1.1 | Core Workspace Scaffold | 2026-02-23 | Cargo workspace, crate structure, tracing, error handling all existed from Phase 0. Added GitHub Actions CI workflow for Linux and Windows (fmt, clippy, build, test). |
| M1.2 | Config Parser and Defaults | 2026-02-23 | `hydra.toml` parser via `serde` + `toml` in `hydra-core::config`. Full typed schema: scoring (profile, weights, gates, diff_scope), adapters, worktree, supervisor. `deny_unknown_fields` catches typos. 11 unit tests. Replaced hand-rolled TOML parser in `doctor.rs`. |
| M1.3 | Worktree Lifecycle Service | 2026-02-23 | `WorktreeService` with async create/list/remove/force_cleanup via git CLI. Branch naming: `hydra/<run_id>/agent/<agent_key>`. Porcelain parser for worktree list. 6 unit tests including full create-remove lifecycle and force cleanup. |
| M1.4 | Process Supervisor (Single Agent) | 2026-02-23 | `supervise()` function with hard timeout, idle timeout (with activity-based reset), cancellation, bounded output buffering. Emits `SupervisorEvent` stream via mpsc channel. Line parser callback for adapter-specific event extraction. Process group isolation via `setsid` on Unix. 7 unit tests. |

## In-Progress Work

(none — M1.1-M1.4 complete; ready for M1.5-M1.8)

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
| 2026-02-23 | Config uses `deny_unknown_fields` on all serde structs | Catches TOML typos at parse time with actionable error messages |
| 2026-02-23 | Supervisor idle timeout yields forever when pipe senders drop | Prevents false idle timeout when process exits normally (stdout/stderr close before wait returns) |
| 2026-02-23 | Supervisor uses `setsid` on Unix for process group isolation | Ensures child processes can be killed as a group on cancellation |
| 2026-02-23 | Worktree service uses `git worktree list --porcelain` for parsing | Machine-readable format avoids fragile human-output parsing |

## Open Issues

- `which` v7 pinned; v8 available but not yet evaluated.
- CI workflow not yet pushed to remote/tested on GitHub Actions.
- Tokio features expanded for supervisor/worktree needs; `process`, `io-util`, `time`, `signal`, `fs` now included.

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 89 passing |
| hydra-cli | Yes | Yes | 3 passing |
| hydra-app | No | - | - |

## Phase Progress

| Phase | Name | Status | Milestones Done |
|-------|------|--------|-----------------|
| 0 | Validation and Guardrails | **Complete** | 8/8 |
| 1 | Core Orchestrator + Single Agent | **In Progress** | 4/8 |
| 2 | Multi-Agent Race + Scoring | Not started | 0/12 |
| 3 | GUI Alpha | Not started | 0/7 |
| 4 | Collaboration Workflows | Not started | 0/6 |
| 5 | Windows Parity + Hardening | Not started | 0/6 |

## Instructions for Next Agent

1. Read `CLAUDE.md` for project overview and conventions.
2. Phase 1 is **in progress** — M1.1 through M1.4 are complete.
3. Current test baseline: `hydra-core` 89 passing, `hydra-cli` 3 passing. `cargo clippy` and `cargo fmt` clean.
4. **Next**: Implement M1.5 (Claude Adapter Runtime Path) and M1.6 (Codex Adapter Runtime Path) in parallel.
   - Both depend on M1.4 (done) and their respective Phase 0 probe milestones (done).
   - Implement `build_command()` and `parse_line()`/`parse_raw()` for each adapter.
   - Wire through the process supervisor.
   - Add integration tests for timeout and cancellation scenarios.
5. After M1.5+M1.6: M1.7 (CLI Race Command) wires config → worktree → adapter → supervisor → artifact.
6. After M1.7: M1.8 (Interrupt and Recovery Tests) covers Ctrl+C, crash, partial completion.
7. Key new files from this session:
   - Config module: `crates/hydra-core/src/config/{mod.rs, schema.rs}`
   - Worktree service: `crates/hydra-core/src/worktree/mod.rs`
   - Process supervisor: `crates/hydra-core/src/supervisor/mod.rs`
   - CI workflow: `.github/workflows/ci.yml`
8. Config schema covers: scoring (profile, weights, gates, diff_scope), adapters, worktree (base_dir, retain), supervisor (hard_timeout, idle_timeout, output_buffer).
9. Doctor command now uses the typed config parser instead of hand-rolled TOML.
10. Worktree service branch convention: `hydra/<run_id>/agent/<agent_key>`.
11. Supervisor supports: start, stream (stdout/stderr), line parsing callback, hard timeout, idle timeout with activity reset, cancellation via handle, bounded output buffering.
