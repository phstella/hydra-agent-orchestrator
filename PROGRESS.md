# Hydra Progress Tracker

Last updated: 2026-02-23

## Current State

- **Phase**: 1 **complete**
- **Milestone**: M1.8 complete; Phase 1 finished (8/8 milestones)
- **Sprint**: Sprint 1 complete (10/10 tickets done); Phase 1 complete
- **Status**: All Phase 0 + Phase 1 milestones implemented. `cargo check/build/test/clippy` clean. 126 tests passing (120 unit + 6 integration).

## Completed Milestones

| ID | Title | Date | Notes |
|----|-------|------|-------|
| M0.1 | Adapter Probe Framework | 2026-02-23 | `AgentAdapter` trait, `ProbeRunner`, `ProbeReport`, `DetectResult`, `CapabilitySet`, error taxonomy. 6 unit tests. |
| M0.2 | Claude Probe Implementation | 2026-02-23 | Parses --help for -p, --output-format, --permission-mode. Fixture-based tests. |
| M0.3 | Codex Probe Implementation | 2026-02-23 | Parses exec subcommand, --json, approval/sandbox flags. Fixture-based tests. |
| M0.4 | Cursor Experimental Probe | 2026-02-23 | Always experimental tier. Status: experimental-ready/blocked/missing. Observed confidence. |
| M0.5 | Run Artifact Convention | 2026-02-23 | `RunLayout`, `RunManifest` (schema_version=1), `EventWriter`/`EventReader` for JSONL. Event persistence now applies secret redaction before write. 16 unit tests. |
| M0.6 | Doctor Command MVP | 2026-02-23 | `hydra doctor` with adapter probes + git checks. Human and JSON output. Non-zero exit on failure. |
| M0.7 | Security Baseline Implementation | 2026-02-23 | `SecretRedactor` (13 patterns + custom), `SandboxPolicy` (strict/unsafe). Hardened with multi-match redaction, path-normalized sandbox checks, and redacted `events.jsonl` persistence. |
| M0.8 | Architecture Decision Lock | 2026-02-23 | ADR 6 (process model) and ADR 7 (storage model) confirmed in architecture.md. |
| M1.1 | Core Workspace Scaffold | 2026-02-23 | Cargo workspace, crate structure, tracing, error handling all existed from Phase 0. Added GitHub Actions CI workflow for Linux and Windows (fmt, clippy, workspace build, all-target build, test). |
| M1.2 | Config Parser and Defaults | 2026-02-23 | `hydra.toml` parser via `serde` + `toml` in `hydra-core::config`. Full typed schema: scoring (profile, weights, gates, diff_scope), adapters, worktree, supervisor. `deny_unknown_fields` catches typos. 11 unit tests. Replaced hand-rolled TOML parser in `doctor.rs`. |
| M1.3 | Worktree Lifecycle Service | 2026-02-23 | `WorktreeService` with async create/list/remove/force_cleanup via git CLI. Branch naming: `hydra/<run_id>/agent/<agent_key>`. Porcelain parser for worktree list. Force cleanup now surfaces cleanup failures and is idempotent. 7 unit tests including full create-remove lifecycle and force cleanup. |
| M1.4 | Process Supervisor (Single Agent) | 2026-02-23 | `supervise()` function with hard timeout, idle timeout (with activity-based reset), cancellation, bounded output buffering. Emits `SupervisorEvent` stream via mpsc channel. Line parser callback for adapter-specific event extraction. Process-group termination now sends Unix `SIGTERM`/`SIGKILL` to the group (`setsid` + group kill). 8 unit tests. |
| M1.5 | Claude Adapter Runtime Path | 2026-02-23 | `build_command()` produces `claude -p <prompt> --output-format stream-json --permission-mode bypassPermissions`. `parse_line()` maps stream-json events (system, assistant, result) to `AgentEvent` variants. `parse_raw()` handles multi-line chunks. 14 new tests (build_command flags, parser for all event types, fixture processing, unknown field tolerance). |
| M1.6 | Codex Adapter Runtime Path | 2026-02-23 | `build_command()` produces `codex exec <prompt> --json --full-auto`. `parse_line()` maps JSONL events (start, message, tool_call, tool_result, completed) to `AgentEvent` variants. `parse_raw()` handles multi-line chunks. 13 new tests (build_command flags, parser for all event types, fixture processing, unknown field tolerance). |
| M1.7 | CLI Race Command (Single Agent) | 2026-02-23 | `hydra race --agents <agent> --prompt <task> [--base-ref HEAD] [--json]`. Wires load_config -> WorktreeService::create -> adapter.build_command -> supervise -> EventWriter -> artifact output. Run summary with branch and artifact path. Human and JSON output modes. Non-zero exit on failure. |
| M1.8 | Interrupt and Recovery Tests | 2026-02-23 | 6 integration tests in `tests/interrupt_recovery.rs`: cancel during execution with cleanup, agent crash with no orphans, timeout with partial artifacts, idempotent force_cleanup, concurrent worktree cleanup without interference, simulated Ctrl+C with process group kill and full cleanup. |

## In-Progress Work

(none — Phase 1 complete; ready for Phase 2)

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
| 2026-02-23 | Added Tokio `sync` feature and CI workspace-build step | Prevents feature-unification masking where tests/clippy pass but normal workspace build fails |
| 2026-02-23 | Supervisor termination escalates from process-group `SIGTERM` to `SIGKILL` on Unix | Prevents orphaned subprocesses from surviving timeout/cancel paths |
| 2026-02-23 | Claude probe flag parsing switched to token-aware matching | Avoids false-positive `-p` detection from unrelated flag substrings |
| 2026-02-23 | `force_cleanup()` now returns concrete cleanup errors and remains idempotent | Improves reliability and observability for interrupt/failure cleanup |
| 2026-02-23 | `EventWriter` now redacts secrets before persisting JSONL lines | Enforces log/artifact scrubbing at the persistence boundary |
| 2026-02-23 | Renamed `supervisor::SupervisorConfig` to `SupervisorPolicy` | Resolves name collision with `config::SupervisorConfig` (TOML schema type) flagged in ANALYSIS.md |
| 2026-02-23 | Claude adapter uses static `parse_stream_json_line()` for parser reuse | Allows both `parse_line()` trait method and race command line_parser closure to share parsing logic |
| 2026-02-23 | Codex adapter uses static `parse_json_line()` for parser reuse | Same pattern as Claude adapter; enables line_parser closure in race command |
| 2026-02-23 | Race command uses `tokio::runtime::Runtime::new()` in CLI main | CLI is sync (clap); race is async. Avoids `block_on` inside existing async context. |

## Open Issues

- `which` v7 pinned; v8 available but not yet evaluated.
- CI workflow not yet pushed to remote/tested on GitHub Actions.

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 120 unit + 6 integration = 126 passing |
| hydra-cli | Yes | Yes | 3 passing |
| hydra-app | No | - | - |

## Phase Progress

| Phase | Name | Status | Milestones Done |
|-------|------|--------|-----------------|
| 0 | Validation and Guardrails | **Complete** | 8/8 |
| 1 | Core Orchestrator + Single Agent | **Complete** | 8/8 |
| 2 | Multi-Agent Race + Scoring | Not started | 0/12 |
| 3 | GUI Alpha | Not started | 0/7 |
| 4 | Collaboration Workflows | Not started | 0/6 |
| 5 | Windows Parity + Hardening | Not started | 0/6 |

## Instructions for Next Agent

1. Read `CLAUDE.md` for project overview and conventions.
2. Phase 1 is **complete** — all 8 milestones done (M1.1 through M1.8).
3. Current baseline: `hydra-core` 126 passing (120 unit + 6 integration), `hydra-cli` 3 passing. `cargo check --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all` are clean.
4. **Next**: Phase 2 — Multi-Agent Race + Scoring, starting with M2.1 (Adapter Registry and Tier Policy).
5. Key files added/modified in this session:
   - `crates/hydra-core/src/adapter/claude.rs` — Added `build_command()`, `parse_stream_json_line()`, `parse_line()`, `parse_raw()`, 14 new tests
   - `crates/hydra-core/src/adapter/codex.rs` — Added `build_command()`, `parse_json_line()`, `parse_line()`, `parse_raw()`, 13 new tests
   - `crates/hydra-core/src/supervisor/mod.rs` — Renamed `SupervisorConfig` to `SupervisorPolicy`
   - `crates/hydra-cli/src/race.rs` — New: `hydra race` command implementation
   - `crates/hydra-cli/src/main.rs` — Added Race subcommand with clap
   - `crates/hydra-core/tests/interrupt_recovery.rs` — New: 6 integration tests for interrupt/recovery
   - `crates/hydra-cli/Cargo.toml` — Added `uuid` dependency
6. Adapter parsers use static methods (`ClaudeAdapter::parse_stream_json_line()`, `CodexAdapter::parse_json_line()`) so both trait methods and CLI closures can call them.
7. Race command flow: `load_config()` → `discover_repo_root()` → `resolve_adapter()` → `RunLayout::create_dirs()` → `WorktreeService::create()` → `adapter.build_command()` → `supervise()` → event loop → `EventWriter` → manifest update → summary output.
8. Integration tests cover: cancellation with cleanup, crash with no orphans, timeout with partial artifacts, idempotent cleanup, concurrent worktree isolation, and full Ctrl+C simulation with process group kill.
