# Hydra Progress Tracker

Last updated: 2026-02-23

## Current State

- **Phase**: 3 **in progress** (GUI Alpha)
- **Milestone**: P3-DS-01, P3-IPC-01, P3-UI-01, P3-UI-02, P3-UI-03, P3-UI-04 complete; M3.1 bootstrap done
- **Sprint**: Phase 3 GUI implementation
- **Status**: All Phase 0–2 milestones remain clean. Phase 3 GUI includes: Tauri v2 app crate, React+TS frontend with design system tokens, IPC layer with backpressure, Preflight Dashboard, Experimental Adapter Modal, Running Agents Rail with Live Output Panel, and Results Scoreboard + explicit winner selection. Runtime status normalization now preserves timeout-vs-failure distinction in the rail/panel lifecycle. Default workspace (`hydra-core`, `hydra-cli`) passes `cargo check/test/clippy` clean. Frontend builds via `npm run build` and `tsc --noEmit` clean. `hydra-app` crate requires system packages (`webkit2gtk-4.1`, `javascriptcoregtk-4.1`) to compile — excluded from default-members.

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
| M1.2 | Config Parser and Defaults | 2026-02-23 | `hydra.toml` parser via `serde` + `toml` in `hydra-core::config`. Full typed schema: scoring (profile, weights, gates, diff_scope), adapters, worktree, supervisor. `deny_unknown_fields` catches typos. 11 unit tests. |
| M1.3 | Worktree Lifecycle Service | 2026-02-23 | `WorktreeService` with async create/list/remove/force_cleanup via git CLI. Branch naming: `hydra/<run_id>/agent/<agent_key>`. Porcelain parser for worktree list. 7 unit tests. |
| M1.4 | Process Supervisor (Single Agent) | 2026-02-23 | `supervise()` function with hard timeout, idle timeout, cancellation, bounded output buffering. 8 unit tests. |
| M1.5 | Claude Adapter Runtime Path | 2026-02-23 | `build_command()`, `parse_stream_json_line()`, `parse_line/raw()`. Runtime command now adds `--verbose` when supported to satisfy current CLI stream-json requirements. |
| M1.6 | Codex Adapter Runtime Path | 2026-02-23 | `build_command()`, `parse_json_line()`, flag fallback logic. Parser now supports current Codex JSON events (`thread.started`, `item.completed`, `turn.completed`, `turn.failed`) for runtime usage capture. |
| M1.7 | CLI Race Command (Single Agent) | 2026-02-23 | `hydra race --agents <agent>` end-to-end pipeline. |
| M1.8 | Interrupt and Recovery Tests | 2026-02-23 | 6 integration tests covering all interrupt/cleanup paths. |
| M2.1 | Adapter Registry and Tier Policy | 2026-02-23 | `AdapterRegistry` in hydra-core with tier-policy enforcement. `from_config()`, `resolve()`, `resolve_many()`, `tier1()`, `available()`. `--allow-experimental-adapters` flag on CLI. 10 unit tests. |
| M2.2 | Parallel Spawn and Supervision | 2026-02-23 | `run_race()` refactored for multi-agent parallel execution using `JoinSet`. Each agent isolated: own worktree, event channel, event writer. One failure does not kill others. `--agents` accepts comma-separated values. |
| M2.3 | Baseline Capture Engine | 2026-02-23 | `scoring` module with `baseline.rs`. `capture_baseline()`, `resolve_commands()`, profile resolution (rust/js-node/python), `CommandsConfig` overrides, test/lint output parsers. `sha2` crate replaces hand-rolled SHA-256 (~90 LOC deleted). 14 unit tests. |
| M2.4 | Scoring Dimension: Build | 2026-02-23 | `score_build()` — binary pass=100/fail=0 with evidence. 5 tests. |
| M2.5 | Scoring Dimension: Tests | 2026-02-23 | `score_tests()` — regression-aware formula with pass_rate, reg_penalty, new_test_bonus, anti-gaming test_drop detection. 10 tests. |
| M2.6 | Scoring Dimension: Lint and Diff Scope | 2026-02-23 | `score_lint()` with delta formula. `score_diff_scope()` with churn/files penalties and protected path cap. `parse_numstat()` for git diff. `compute_diff_stats()` async. 14 tests. |
| M2.7 | Composite Ranking and Mergeability Gates | 2026-02-23 | `rank_agents()` with weighted composite, speed dimension, missing-dimension renormalization, mergeability gates. `AgentScore` serializable. 8 tests. |
| M2.8 | CLI Merge Command with Dry-Run | 2026-02-23 | `hydra merge --run-id <UUID> [--agent] [--dry-run\|--confirm] [--force]`. Dry-run with conflict detection. Winner auto-selection from scores. Merge report artifact. |
| M2.9 | Experimental Cursor Opt-In Path | 2026-02-23 | `CursorAdapter.build_command()` and `parse_line()`/`parse_raw()` implemented. Stream-json parsing. `[experimental]` label in race output. 8 new tests. |
| M2.10 | End-to-End Race Integration Test | 2026-02-23 | 6 integration tests: two-agent concurrent completion, one-agent failure isolation, scoring ranking correctness, scoring reproducibility from artifacts, baseline capture roundtrip, artifact layout completeness. |
| M2.11 | Cost and Budget Engine | 2026-02-23 | `UsageAccumulator` for token capture, `CostEstimate`, `BudgetAction::Continue\|Stop`, token/cost budget enforcement. Live-validated token budget stop from real adapter usage in single-agent (`5021b0e3-9848-464f-91ef-6e14a92cc092`) and multi-agent (`31032e95-dad2-4d83-a18e-151a80bcd8f5`) races. |
| M2.12 | Observability Contract | 2026-02-23 | `schema_version` bumped to 2, `event_schema_version` added. `EventSchemaDefinition` enumerates all 13 event kinds. `RunHealthMetrics` computed from events (success_rate, overhead, adapter_errors). 4 tests. |
| M3.1-partial | Tauri App Bootstrap | 2026-02-23 | `hydra-app` crate scaffolded with Tauri v2 + React + TypeScript. IPC commands registered. Shared types between Rust and TS. Requires system libs to compile. |
| P3-DS-01 | Visual Design System v0 | 2026-02-23 | CSS token system (colors, spacing, typography, shadows, transitions). Core primitives: Button, Badge, Card, Panel, Modal, Tabs, ProgressBar. Dark+green theme with marine-blue highlights. |
| P3-IPC-01 | GUI Race IPC + Event Backpressure | 2026-02-23 | Typed IPC commands (health_check, run_preflight, list_adapters, start_race, get_race_result). Frontend event buffer with bounded backpressure (2000 events, 100ms flush). Mock fallback for standalone dev. |
| P3-UI-01 | System Preflight Dashboard | 2026-02-23 | Readiness hero card, diagnostic check rows with status badges, environment panel with adapter badges, warnings panel. Re-run diagnostics action. Matches Image #1 mockup layout. |
| P3-UI-02 | Experimental Adapter Opt-In Modal | 2026-02-23 | Warning modal with resource impact bar, risk acknowledgment checkbox, disabled confirm until acknowledged. Matches Image #2 mockup. |
| P3-UI-03 | Live Agent Output + Running Agents Rail | 2026-02-23 | AgentRail with lifecycle badges (running/completed/failed/timed_out), selected-agent state driving LiveOutputPanel context switch, backpressure-safe rendering via useEventBuffer, mock IPC with multi-agent event stream. Matches M3.3 acceptance criteria. |
| P3-UI-04 | Results Scoreboard + Winner Selection | 2026-02-23 | Ranked candidate cards with mergeability gating, explicit winner selection, per-dimension score table, and run metadata badges (duration/cost where available). Winner state is app-owned and consistent across tabs. |

## In-Progress Work

- **Phase 3 GUI Alpha**: P3-UI-05 (Diff Review + Merge Rail) and P3-QA-01 smoke coverage are next.
- M3.2 IPC surface is wired end-to-end through `hydra race --json` with event polling and parsed run summaries.
- M3.5 diff/review components and M3.7 smoke tests are pending.

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
| 2026-02-23 | Sandbox strict-mode fallback now normalizes absolute paths and components | Closes prefix-based bypass for non-existent paths |
| 2026-02-23 | Secret redaction now handles multiple occurrences per line | Prevents leakage when the same token prefix appears multiple times on one log line |
| 2026-02-23 | `hydra doctor` reads optional adapter path overrides from `hydra.toml` | Enables configured binary paths before full config parser milestone (M1.2) lands |
| 2026-02-23 | Reduced Tokio feature sets in core/cli crates | Keeps runtime surface lean while preserving required async/runtime capabilities |
| 2026-02-23 | Config uses `deny_unknown_fields` on all serde structs | Catches TOML typos at parse time with actionable error messages |
| 2026-02-23 | Supervisor idle timeout yields forever when pipe senders drop | Prevents false idle timeout when process exits normally |
| 2026-02-23 | Supervisor uses `setsid` on Unix for process group isolation | Ensures child processes can be killed as a group on cancellation |
| 2026-02-23 | Worktree service uses `git worktree list --porcelain` for parsing | Machine-readable format avoids fragile human-output parsing |
| 2026-02-23 | Added Tokio `sync` feature and CI workspace-build step | Prevents feature-unification masking |
| 2026-02-23 | Supervisor termination escalates from process-group `SIGTERM` to `SIGKILL` on Unix | Prevents orphaned subprocesses from surviving timeout/cancel paths |
| 2026-02-23 | Claude probe flag parsing switched to token-aware matching | Avoids false-positive `-p` detection from unrelated flag substrings |
| 2026-02-23 | `force_cleanup()` now returns concrete cleanup errors and remains idempotent | Improves reliability and observability for interrupt/failure cleanup |
| 2026-02-23 | `EventWriter` now redacts secrets before persisting JSONL lines | Enforces log/artifact scrubbing at the persistence boundary |
| 2026-02-23 | Renamed `supervisor::SupervisorConfig` to `SupervisorPolicy` | Resolves name collision with `config::SupervisorConfig` |
| 2026-02-23 | Claude adapter uses static `parse_stream_json_line()` for parser reuse | Allows both `parse_line()` trait method and race command line_parser closure to share parsing logic |
| 2026-02-23 | Codex adapter uses static `parse_json_line()` for parser reuse | Same pattern as Claude adapter |
| 2026-02-23 | Race command uses `tokio::runtime::Runtime::new()` in CLI main | CLI is sync (clap); race is async. Avoids `block_on` inside existing async context. |
| 2026-02-23 | `AdapterRegistry` replaces `resolve_adapter()` free function | Centralized tier-policy enforcement; extensible for future adapters |
| 2026-02-23 | `--agents` accepts comma-separated values via clap `value_delimiter` | Consistent multi-value pattern; `hydra race --agents claude,codex` |
| 2026-02-23 | Parallel agent execution uses `JoinSet` with per-agent event writers | Full isolation: one agent's panic/failure cannot affect others |
| 2026-02-23 | `sha2` crate replaces hand-rolled SHA-256 | Deletes 90 LOC of crypto code in favor of well-tested library |
| 2026-02-23 | Scoring module uses `CommandsConfig` for explicit command overrides | Profile provides defaults; explicit commands override; no profile = no scoring |
| 2026-02-23 | Test output parser supports cargo test, pytest, jest/mocha patterns | Fallback to exit-code mode when no pattern matches |
| 2026-02-23 | Diff scope uses `git diff --numstat` for machine-parseable output | Avoids fragile stat parsing; gives per-file line counts |
| 2026-02-23 | Manifest schema_version bumped to 2 for Phase 2 | Adds `event_schema_version` field; breaking change from v1 |
| 2026-02-23 | `RunHealthMetrics` computed from events, not manifest | Events are source of truth; metrics are derived |
| 2026-02-23 | Codex parser now handles current `--json` event envelope (`turn.completed` usage) | Restores runtime token capture and budget-stop behavior after upstream CLI event-shape drift |
| 2026-02-23 | `hydra-app` excluded from Cargo workspace `default-members` | Tauri requires `webkit2gtk-4.1` and `javascriptcoregtk-4.1` system libs; prevents breaking `cargo check/test/clippy` for non-GUI developers |
| 2026-02-23 | Frontend uses React 19 JSX transform (no `import React`) | Cleaner imports; `type`-only imports for CSSProperties/ReactNode |
| 2026-02-23 | Mock IPC fallback for standalone frontend dev | `ipc.ts` detects missing `@tauri-apps/api` and falls back to mock data with simulated latency |
| 2026-02-23 | Event buffer uses bounded flush at 100ms intervals | Prevents per-event re-renders during high-throughput streams; caps at 2000 events to avoid memory blowup |
| 2026-02-23 | Agent lifecycle derived from event stream via `useAgentStatuses` hook | Stateless derivation from events; no separate status channel needed; terminal events (`agent_completed`, `agent_failed`, `agent_timed_out`) transition lifecycle |
| 2026-02-23 | LiveOutputPanel uses tail-window rendering (last 200 events) | Prevents DOM bloat under sustained high-volume output while preserving auto-scroll UX |
| 2026-02-23 | Race summary output now includes top-level `duration_ms` and `total_cost` fields | Keeps GUI run metadata rendering stable while preserving detailed nested `cost` payload |
| 2026-02-23 | Timeout lifecycle is inferred from `agent_failed` timeout signatures in UI hook | Preserves visual distinction between timed-out and failed agents without changing artifact event schema |
| 2026-02-23 | Design tokens defined as CSS custom properties (not JS) | Faster at runtime; works with any CSS-in-JS approach; lint-enforceable via pattern match |

## Open Issues

- `which` v7 pinned; v8 available but not yet evaluated.
- CI workflow not yet pushed to remote/tested on GitHub Actions.
- Multi-agent budget stop may cancel slower adapters before they emit usage, resulting in expected per-agent `usage_status: missing` for canceled peers.
- **`hydra-app` requires system packages** `webkit2gtk-4.1` and `javascriptcoregtk-4.1` (Arch: `pacman -S webkit2gtk-4.1`). Without them, `cargo check/test -p hydra-app` fails at build-script stage.
- Frontend `@tauri-apps/api` is a peer dep — only resolves at runtime inside Tauri webview. Mock fallback covers standalone dev.

## Crate Status

| Crate | Exists | Compiles | Tests |
|-------|--------|----------|-------|
| hydra-core | Yes | Yes | 218 unit + 12 integration = 230 passing |
| hydra-cli | Yes | Yes | 7 passing |
| hydra-app | Yes | Requires system libs | 5 unit tests (pending system libs) |

## Phase Progress

| Phase | Name | Status | Milestones Done |
|-------|------|--------|-----------------|
| 0 | Validation and Guardrails | **Complete** | 8/8 |
| 1 | Core Orchestrator + Single Agent | **Complete** | 8/8 |
| 2 | Multi-Agent Race + Scoring | **Complete** | 12/12 |
| 3 | GUI Alpha | **In Progress** | 6/7+ (P3-DS-01, P3-IPC-01, P3-UI-01, P3-UI-02, P3-UI-03, P3-UI-04) |
| 4 | Collaboration Workflows | Not started | 0/6 |
| 5 | Windows Parity + Hardening | Not started | 0/6 |

## Instructions for Next Agent

1. Read `CLAUDE.md` for project overview and conventions.
2. Phase 2 is **complete** — all 12 milestones done (M2.1 through M2.12).
3. Phase 3 is **in progress** — P3-DS-01, P3-IPC-01, P3-UI-01, P3-UI-02, P3-UI-03, P3-UI-04 implemented.
4. Current baseline: `hydra-core` 230 passing, `hydra-cli` 7 passing. Default workspace `cargo check/test/clippy` clean. Frontend `tsc --noEmit` and `vite build` clean.
5. **System package requirement**: `hydra-app` needs `webkit2gtk-4.1` (`pacman -S webkit2gtk-4.1` on Arch). Install before attempting `cargo check -p hydra-app`.
6. **Next priorities**:
   - P3-UI-05: Candidate Diff Review + Merge Action Rail (M3.5)
   - M3.7: GUI Smoke Test Pack
7. Key files added in Phase 3:
   - `crates/hydra-app/` — NEW crate: Tauri v2 backend (commands, state, IPC types)
   - `crates/hydra-app/frontend/` — React+TS frontend app
   - `crates/hydra-app/frontend/src/styles/tokens.css` — Design system token source
   - `crates/hydra-app/frontend/src/components/design-system/` — Core primitives
   - `crates/hydra-app/frontend/src/components/PreflightDashboard.tsx` — P3-UI-01
   - `crates/hydra-app/frontend/src/components/ExperimentalAdapterModal.tsx` — P3-UI-02
   - `crates/hydra-app/frontend/src/components/AgentRail.tsx` — P3-UI-03 running-agent rail
   - `crates/hydra-app/frontend/src/components/LiveOutputPanel.tsx` — P3-UI-03 live output panel
   - `crates/hydra-app/frontend/src/hooks/useAgentStatuses.ts` — P3-UI-03 agent lifecycle derivation
   - `crates/hydra-app/frontend/src/components/ResultsScoreboard.tsx` — P3-UI-04 results scoreboard + winner selection
   - `crates/hydra-app/frontend/src/ipc.ts` — IPC bridge with mock fallback
   - `crates/hydra-app/frontend/src/hooks/` — usePreflight, useEventBuffer (backpressure)
8. Design system tokens are CSS custom properties in `tokens.css`. All primitives consume tokens only. Feature components must NOT use hardcoded hex colors.
9. IPC flow: Frontend calls `invoke('command_name', args)` → Tauri dispatches to `#[tauri::command]` fn → returns JSON. Mock fallback for dev without Tauri runtime.
10. Adapter/status data is runtime-driven from `ProbeRunner`/`AdapterRegistry`. No hardcoded agent names in UI components.
