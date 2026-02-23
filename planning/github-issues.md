# GitHub Issue Body Pack

Last updated: 2026-02-22

This file contains copy-paste-ready issue bodies generated from `planning/implementation-checklist.md` milestones `M0.1` through `M5.6`.

## Global Label Prefix

- `hydra` (add this label to every issue)

## [M0.1] Adapter Probe Framework

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: M
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Adapter assumptions can drift silently.

## Scope
Add probe interface and unified probe report model.

## Acceptance Criteria
- [ ] hydra doctor emits JSON report with adapter probe status.
- [ ] Probe output includes binary path, version, supported flags, confidence.
- [ ] Unknown adapters do not crash doctor command.

## Out of Scope
Full adapter execution.

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.2] Claude Probe Implementation

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: S
- Dependencies: M0.1

### Issue Body (Markdown)

```md
## Problem
Tier-1 adapter must be validated at startup.

## Scope
Implement Claude probe for required headless flags.

## Acceptance Criteria
- [ ] Probe verifies -p and --output-format support.
- [ ] Probe result status is ready or blocked with clear reason.
- [ ] Fixture-based probe test passes in CI.

## Out of Scope
runtime parsing logic.

## Dependencies
- M0.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.3] Codex Probe Implementation

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: S
- Dependencies: M0.1

### Issue Body (Markdown)

```md
## Problem
Tier-1 adapter must be validated at startup.

## Scope
Implement Codex probe for exec, --json, and approval mode flags.

## Acceptance Criteria
- [ ] Probe verifies exec subcommand exists.
- [ ] Probe verifies JSON output flag support.
- [ ] Probe handles known flag variants without panic.

## Out of Scope
full scoring integration.

## Dependencies
- M0.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.4] Cursor Experimental Probe

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.1

### Issue Body (Markdown)

```md
## Problem
Cursor interface variability must not break default flows.

## Scope
Add Cursor probe with experimental classification.

## Acceptance Criteria
- [ ] Cursor probe never promotes adapter to Tier-1.
- [ ] Probe result can be experimental-ready, experimental-blocked, or missing.
- [ ] UI and CLI mark adapter as experimental.

## Out of Scope
enabling Cursor by default.

## Dependencies
- M0.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.5] Run Artifact Convention

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-feature
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Runs need deterministic artifact paths for replay.

## Scope
Define .hydra/runs/<run_id>/ structure and write metadata manifest.

## Acceptance Criteria
- [ ] Every run writes manifest.json and events.jsonl.
- [ ] Artifact paths are OS-safe on Linux and Windows.
- [ ] Cleanup policy respects retention config.

## Out of Scope
GUI history viewer.

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.6] Doctor Command MVP

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-feature
- Estimate: S
- Dependencies: M0.1, M0.2, M0.3, M0.4

### Issue Body (Markdown)

```md
## Problem
Users need quick readiness check before run.

## Scope
Implement hydra doctor summary + JSON output mode.

## Acceptance Criteria
- [ ] Exit code is non-zero when Tier-1 prerequisites fail.
- [ ] Output includes git repo checks and adapter readiness.
- [ ] --json output is stable and parseable.

## Out of Scope
auto-fix behavior.

## Dependencies
- M0.1, M0.2, M0.3, M0.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.7] Security Baseline Implementation

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-feature
- Estimate: M
- Dependencies: M0.5

### Issue Body (Markdown)

```md
## Problem
Security intent exists in architecture docs but has no implementation milestones. Agent processes inherit environment variables including API keys; logs and artifacts can capture secrets from agent output.

## Scope
Implement secret redaction rules for logs and artifacts, sandbox policy enforcement for agent worktrees, and unsafe-mode guardrails.

## Acceptance Criteria
- [ ] Known secret patterns (API keys, tokens) are redacted from persisted logs and artifacts.
- [ ] Agent processes cannot write outside their assigned worktree unless unsafe mode is explicitly enabled.
- [ ] Unsafe mode requires explicit per-run opt-in flag and emits a visible warning.
- [ ] Log scrubbing unit tests pass with known secret fixtures.

## Out of Scope
full threat model document; runtime network sandboxing.

## Dependencies
- M0.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M0.8] Architecture Decision Lock

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-chore
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Two architecture decisions (process model and storage model) were deferred as open questions but affect implementation choices in Phase 1 and Phase 3.

## Scope
Document locked decisions for process model (short-lived CLI, embedded GUI) and storage model (JSONL source of truth, SQLite derived index from Phase 3) in architecture.md.

## Acceptance Criteria
- [ ] ADR entries 6 and 7 are present in docs/architecture.md.
- [ ] Open questions section is updated to reflect resolved status.
- [ ] No implementation is blocked by unresolved architecture questions.

## Out of Scope
implementing the SQLite index (Phase 3).

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.1] Core Workspace Scaffold

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
No Rust workspace exists yet. All subsequent features need a compilable crate structure with shared error handling and logging.

## Scope
Create `hydra-core` library crate and `hydra-cli` binary crate in a Cargo workspace. Wire `tracing` for structured logging and `thiserror`/`anyhow` for error handling. Set up CI for Linux and Windows compilation.

## Acceptance Criteria
- [ ] Workspace builds with hydra-core and hydra-cli crates.
- [ ] Logging and error crates wired consistently.
- [ ] CI compiles on Linux and Windows.

## Out of Scope
runtime logic, config parsing, adapter code.

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.2] Config Parser and Defaults

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
Hydra needs a user-editable configuration file to control scoring weights, adapter preferences, timeout values, and artifact retention. Without a config parser, all behavior must be hardcoded or passed via CLI flags.

## Scope
Implement `hydra.toml` parser using `serde` + `toml` crate. Define the full configuration schema with typed fields, defaults for all optional values, and actionable validation error messages.

## Acceptance Criteria
- [ ] hydra.toml parses with schema validation.
- [ ] Missing optional fields get deterministic defaults.
- [ ] Invalid config returns actionable error messages.

## Out of Scope
GUI config editor; runtime config reload.

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.3] Worktree Lifecycle Service

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: M
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
Agents must run in isolated worktrees to prevent file collisions. Worktree creation, tracking, and cleanup (including on interrupt and failure) must be reliable to avoid orphaned directories and branches.

## Scope
Implement worktree create/list/remove operations via git CLI. Add interrupt-safe cleanup using signal handlers. Ensure paths are valid on both Linux and Windows.

## Acceptance Criteria
- [ ] Create/list/remove worktree operations are implemented.
- [ ] Interrupt-safe cleanup path exists.
- [ ] Windows path handling tests pass.

## Out of Scope
garbage collection scheduler; sparse checkout support.

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.4] Process Supervisor (Single Agent)

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: M
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
Agent CLI processes need lifecycle management: spawning with correct arguments, streaming stdout/stderr, enforcing timeouts, and graceful cancellation. Without supervision, hung agents consume resources indefinitely and produce no usable artifacts.

## Scope
Build a single-agent process supervisor with start, stream, timeout (idle + hard), and cancel support. Implement bounded output buffering and emit normalized lifecycle events to the event bus.

## Acceptance Criteria
- [ ] Supports start, stream, timeout, cancel.
- [ ] Bounded output buffering prevents memory blowups.
- [ ] Emits normalized lifecycle events.

## Out of Scope
parallel supervision (Phase 2); PTY layer (handled separately).

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.5] Claude Adapter Runtime Path

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.2, M1.4

### Issue Body (Markdown)

```md
## Problem
The Claude adapter probe (M0.2) validates binary presence and flags, but the actual runtime path (spawning Claude in a worktree, parsing its stream-json output, mapping events to the normalized schema) has not been implemented.

## Scope
Implement `build_command()` and `parse_line()`/`parse_raw()` for the Claude adapter. Wire it through the process supervisor. Cover timeout and cancellation with integration tests.

## Acceptance Criteria
- [ ] claude runs in isolated worktree.
- [ ] Stream parser maps key events to normalized schema.
- [ ] Timeout and cancellation are covered by tests.

## Out of Scope
multi-agent orchestration; scoring.

## Dependencies
- M0.2, M1.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.6] Codex Adapter Runtime Path

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.3, M1.4

### Issue Body (Markdown)

```md
## Problem
The Codex adapter probe (M0.3) validates binary presence and flags, but the runtime path (spawning `codex exec` in a worktree, parsing JSON stream output, handling flag variants) has not been implemented.

## Scope
Implement `build_command()` and `parse_line()`/`parse_raw()` for the Codex adapter. Handle known flag variants gracefully. Wire through process supervisor with integration tests.

## Acceptance Criteria
- [ ] codex exec works in isolated worktree.
- [ ] JSON stream parser maps events and usage data.
- [ ] Unsupported flag fallback logic is tested.

## Out of Scope
multi-agent orchestration; scoring.

## Dependencies
- M0.3, M1.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.7] CLI Race Command (Single Agent)

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: M1.2, M1.3, M1.5

### Issue Body (Markdown)

```md
## Problem
There is no end-to-end CLI command that ties together config parsing, worktree creation, and agent execution. Users need a single command to run an agent on a task and get results.

## Scope
Implement `hydra race --agents <agent>` command using clap. Wire config -> worktree -> adapter -> supervisor -> artifact output into a single flow. Output run summary with branch name and artifact path.

## Acceptance Criteria
- [ ] hydra race --agents claude completes end-to-end.
- [ ] Run summary includes branch and artifact path.
- [ ] Non-zero exit codes on fatal failures.

## Out of Scope
multi-agent parallel execution; scoring.

## Dependencies
- M1.2, M1.3, M1.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M1.8] Interrupt and Recovery Tests

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-test, type-test
- Estimate: M
- Dependencies: M1.3, M1.4, M1.7

### Issue Body (Markdown)

```md
## Problem
Interrupted runs (Ctrl+C, process kill, system crash) can leave orphaned worktrees, stale branches, and incomplete artifacts. These failure paths must be tested to ensure cleanup is reliable.

## Scope
Write integration tests for interrupt scenarios: Ctrl+C during agent execution, agent process crash, partial completion. Verify worktree and branch cleanup, artifact integrity, and absence of orphaned resources.

## Acceptance Criteria
- [ ] Ctrl+C cleanup test passes.
- [ ] Partial failure leaves usable artifacts.
- [ ] No orphan worktrees after test run.

## Out of Scope
crash recovery metadata (Phase 5); Windows-specific interrupt behavior (Phase 5).

## Dependencies
- M1.3, M1.4, M1.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.1] Adapter Registry and Tier Policy

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-adapter, type-feature
- Estimate: S
- Dependencies: M1.6

### Issue Body (Markdown)

```md
## Problem
Phase 1 implements individual adapters, but there is no central registry that enforces tier policy (Tier-1 vs experimental). Without a registry, the system cannot programmatically select which adapters to use in a run or block experimental adapters by default.

## Scope
Implement an adapter registry that discovers available adapters, applies tier policy, and exposes the filtered set to the orchestrator. Default runs select only Tier-1 adapters; experimental adapters require `--allow-experimental-adapters`.

## Acceptance Criteria
- [ ] Registry supports Tier-1 and experimental tiers.
- [ ] Default run selects only Tier-1 adapters.
- [ ] Experimental adapters require explicit opt-in flag.

## Out of Scope
dynamic adapter loading; third-party adapter registration.

## Dependencies
- M1.6

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.2] Parallel Spawn and Supervision

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-core, type-feature
- Estimate: M
- Dependencies: M1.4, M2.1

### Issue Body (Markdown)

```md
## Problem
Phase 1 supervisor handles one agent at a time. Race mode requires spawning multiple agents concurrently with independent lifecycle management so that one agent's failure does not kill the others.

## Scope
Extend the process supervisor to manage multiple concurrent agent processes. Implement independent failure isolation, aggregate status computation, and concurrent event stream merging.

## Acceptance Criteria
- [ ] Two Tier-1 agents run concurrently.
- [ ] One agent failure does not kill others.
- [ ] Aggregate run status is deterministic.

## Out of Scope
resource throttling; agent priority scheduling.

## Dependencies
- M1.4, M2.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.3] Baseline Capture Engine

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: S
- Dependencies: M1.2

### Issue Body (Markdown)

```md
## Problem
Scoring requires a baseline to compare agent outputs against. Without capturing build/test/lint state on the base ref before agents run, the scoring engine cannot distinguish pre-existing failures from agent-introduced regressions.

## Scope
Run configured build/test/lint commands on the base ref before agent execution. Persist baseline results as artifacts. Handle missing commands gracefully with explicit unavailable status.

## Acceptance Criteria
- [ ] Build/test/lint baseline captured once per run.
- [ ] Baseline outputs persisted as artifacts.
- [ ] Missing commands handled with explicit status.

## Out of Scope
baseline caching across runs; custom baseline commands.

## Dependencies
- M1.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.4] Scoring Dimension: Build

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: S
- Dependencies: M2.3

### Issue Body (Markdown)

```md
## Problem
No build score dimension exists yet. Build pass/fail is the most fundamental viability gate: a broken build means the agent output cannot be used.

## Scope
Implement the build scoring dimension (pass=100, fail=0). Run the configured build command in each agent's worktree. Handle timeouts and command failures. Include raw evidence references in score payload.

## Acceptance Criteria
- [ ] Build score computed per candidate.
- [ ] Timeout and command failure paths tested.
- [ ] Score payload includes raw evidence references.

## Out of Scope
partial build credit; incremental build support.

## Dependencies
- M2.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.5] Scoring Dimension: Tests

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: M
- Dependencies: M2.3

### Issue Body (Markdown)

```md
## Problem
Test pass rate alone is insufficient for scoring. Agents can game scores by deleting tests or introducing regressions that were masked by other failures. The scoring dimension needs to be regression-aware and resistant to test-drop manipulation.

## Scope
Implement the test scoring formula with regression penalty, new-test bonus, and baseline comparison. Add parser fallback to exit-code mode for test frameworks that do not produce structured output. Include anti-gaming checks for dropped test counts.

## Acceptance Criteria
- [ ] Regression-aware formula implemented.
- [ ] Parser fallback to exit-code mode works.
- [ ] Test-drop anti-gaming checks included.

## Out of Scope
per-test-case tracking; flaky test detection.

## Dependencies
- M2.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.6] Scoring Dimension: Lint and Diff Scope

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: M
- Dependencies: M2.3

### Issue Body (Markdown)

```md
## Problem
Agents may introduce lint violations or make broad, unfocused changes that touch files outside the task scope. Without lint and diff scope scoring, there is no signal for code maintainability or change reviewability.

## Scope
Implement lint delta scoring (new errors/warnings vs baseline). Implement diff scope scoring (files touched, lines churned, protected path violations). Make protected path penalty configurable via `hydra.toml`.

## Acceptance Criteria
- [ ] Lint delta scoring implemented.
- [ ] Diff scope scoring includes file/churn checks.
- [ ] Protected path penalty is configurable.

## Out of Scope
formatter-aware diff normalization; semantic diff analysis.

## Dependencies
- M2.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.7] Composite Ranking and Mergeability Gates

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: S
- Dependencies: M2.4, M2.5, M2.6

### Issue Body (Markdown)

```md
## Problem
Individual dimension scores exist but there is no composite ranking or mergeability decision. Users need a single ranked list with clear merge/no-merge signals.

## Scope
Implement weighted composite score calculation with dimension renormalization for missing dimensions. Apply mergeability gates (build must pass, test regression below threshold). Expose ranking and gate results in structured output.

## Acceptance Criteria
- [ ] Weighted composite scores are reproducible.
- [ ] Missing dimensions renormalize weights.
- [ ] Mergeability gates are exposed in output.

## Out of Scope
user-adjustable weights at runtime; pairwise preference learning.

## Dependencies
- M2.4, M2.5, M2.6

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.8] CLI Merge Command with Dry-Run

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-core, type-feature
- Estimate: M
- Dependencies: M2.7

### Issue Body (Markdown)

```md
## Problem
After scoring, users need a safe way to merge the winning agent's branch. Without dry-run support, users must manually run git merge and may encounter unexpected conflicts.

## Scope
Implement `hydra merge` command with `--dry-run` mode that reports potential conflicts without modifying the working tree. Real merge requires explicit `--confirm` flag. Write conflict report artifact on merge failure.

## Acceptance Criteria
- [ ] Dry-run reports potential conflicts.
- [ ] Real merge requires explicit confirmation flag.
- [ ] Conflict report artifact is written on failure.

## Out of Scope
automatic conflict resolution; cherry-pick mode.

## Dependencies
- M2.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.9] Experimental Cursor Opt-In Path

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.4, M2.1, M2.2

### Issue Body (Markdown)

```md
## Problem
Cursor's CLI stability is not at Tier-1 level, but users may want to include it in race runs for comparison. The experimental adapter path must enforce opt-in gating to prevent accidental use and clearly communicate risk.

## Scope
Wire the Cursor adapter through the registry with experimental tier classification. Require `--allow-experimental-adapters` flag for inclusion. Label all Cursor output as experimental. Block runtime activation if probe fails.

## Acceptance Criteria
- [ ] Cursor can run only with --allow-experimental-adapters.
- [ ] Output labels include experimental warning.
- [ ] Failing probe blocks runtime activation.

## Out of Scope
Cursor Tier-1 promotion; Cursor-specific output parsing improvements.

## Dependencies
- M0.4, M2.1, M2.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.10] End-to-End Race Integration Test

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-test, type-test
- Estimate: M
- Dependencies: M2.2, M2.3, M2.4, M2.5, M2.6, M2.7, M2.8

### Issue Body (Markdown)

```md
## Problem
Individual scoring dimensions and parallel execution are tested in isolation, but no test validates the full race flow from spawn to ranked output with complete artifacts.

## Scope
Write an end-to-end integration test that starts a multi-agent race, verifies scoring output shape, checks artifact completeness, and confirms reproducibility from saved artifacts.

## Acceptance Criteria
- [ ] Full race test verifies ranking output shape.
- [ ] Artifacts are complete and replayable.
- [ ] Linux and Windows CI jobs pass.

## Out of Scope
GUI integration; workflow mode testing.

## Dependencies
- M2.2, M2.3, M2.4, M2.5, M2.6, M2.7, M2.8

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.11] Cost and Budget Engine

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: M
- Dependencies: M2.2, M2.7

### Issue Body (Markdown)

```md
## Problem
Race mode runs multiple agents on the same task, multiplying API cost. Agents that report token usage (via `emits_usage` capability) produce cost-relevant data, but Hydra has no system to capture, normalize, aggregate, or display this data. Users cannot make cost-informed decisions about which agent to prefer or when to stop a run.

## Scope
Implement token usage capture from agent event streams. Normalize usage data across adapters. Aggregate per-run and per-agent cost estimates. Add budget stop conditions (`max_tokens_total`, `max_cost_usd`) that terminate agents when limits are exceeded. Display cost summary in CLI race output.

## Acceptance Criteria
- [ ] Token usage from adapters that emit it is captured and persisted in run artifacts.
- [ ] Per-agent and per-run cost estimates are included in scoring output.
- [ ] Budget limits in hydra.toml stop agents when exceeded.
- [ ] Adapters that do not emit usage data produce explicit unavailable status rather than silent omission.

## Out of Scope
real-time cost streaming to GUI; historical cost trend analysis.

## Dependencies
- M2.2, M2.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M2.12] Observability Contract

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-core, type-feature
- Estimate: S
- Dependencies: M2.10

### Issue Body (Markdown)

```md
## Problem
The event schema (events.jsonl) is not versioned, making it fragile across Hydra updates. Run health indicators exist informally in logs but are not structured for programmatic consumption. Without a stable observability contract, the GUI and external tooling cannot reliably consume run data.

## Scope
Define and version the event schema. Add a schema version field to `manifest.json`. Implement minimum run health indicators (success rate, overhead timing, adapter error counts) as structured output. Ensure CLI `--json` output and artifact schema are documented and stable.

## Acceptance Criteria
- [ ] manifest.json includes a schema_version field.
- [ ] Event types are enumerated in a versioned schema definition.
- [ ] Run health metrics (success rate, orchestration overhead, adapter error rate) are computable from persisted artifacts.
- [ ] Breaking schema changes require version bump and migration note.

## Out of Scope
Prometheus/Grafana export; GUI dashboard integration.

## Dependencies
- M2.10

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.1] Tauri App Bootstrap

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: S
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
The GUI surface does not exist yet. A Tauri v2 application needs to be bootstrapped with shared type definitions between the Rust backend and React frontend to ensure the GUI can consume the same data structures as the CLI.

## Scope
Create Tauri v2 app scaffold with React + TypeScript frontend. Define shared type generation between Rust and TypeScript. Verify GUI launches and can query backend health endpoint. Set up Linux packaging smoke test.

## Acceptance Criteria
- [ ] GUI launches and can query backend health.
- [ ] Shared types compile on frontend and backend.
- [ ] Linux packaging smoke test passes.

## Out of Scope
functional race UI; Windows packaging.

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.2] IPC Command Surface (Race)

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: M2.10, M3.1

### Issue Body (Markdown)

```md
## Problem
The GUI needs to trigger races and receive results via Tauri IPC commands. Without a defined IPC surface, the frontend cannot drive orchestration or display results.

## Scope
Implement Tauri IPC commands for starting a race, streaming events, and fetching results. Map core errors to human-readable frontend messages. Implement backpressure handling to prevent UI freezes during high-throughput event streams.

## Acceptance Criteria
- [ ] Start race and fetch results via IPC.
- [ ] Error mapping is human-readable.
- [ ] Backpressure does not freeze UI.

## Out of Scope
workflow IPC commands; settings UI.

## Dependencies
- M2.10, M3.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.3] Live Agent Output Panels

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: M3.2

### Issue Body (Markdown)

```md
## Problem
During a race, users need to see each agent's output in real time to understand progress and catch issues early. Without live panels, the GUI is no better than CLI for monitoring.

## Scope
Build a per-agent output panel component using xterm.js or equivalent. Display lifecycle status badges (running, completed, failed, timed out). Ensure stream rendering remains responsive under high output volume.

## Acceptance Criteria
- [ ] One panel per running agent.
- [ ] Status badges track lifecycle changes.
- [ ] Stream rendering remains responsive under load.

## Out of Scope
log search/filter; output export.

## Dependencies
- M3.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.4] Scoreboard and Mergeability UI

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: S
- Dependencies: M2.7, M3.2

### Issue Body (Markdown)

```md
## Problem
Scoring results are available as JSON but not visually presented. Users need a ranked scoreboard with per-dimension breakdown and clear mergeability signals to make informed merge decisions.

## Scope
Build ranked score cards showing composite score, per-dimension breakdown, and mergeable/not-mergeable status. Visually block merge actions for non-mergeable candidates. Make winner selection an explicit user action.

## Acceptance Criteria
- [ ] Ranked cards show score breakdown.
- [ ] Non-mergeable candidates are visually blocked.
- [ ] Winner selection action is explicit.

## Out of Scope
score comparison across runs; score trend charts.

## Dependencies
- M2.7, M3.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.5] Diff Viewer Integration

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: M3.2

### Issue Body (Markdown)

```md
## Problem
Users need to visually inspect agent diffs to validate scoring results and understand what each agent changed. A side-by-side diff viewer is essential for the GUI's review workflow.

## Scope
Integrate Monaco diff viewer or equivalent. Allow switching between candidate diffs. Handle large diffs gracefully (virtualized rendering). Show fallback message when diff is unavailable.

## Acceptance Criteria
- [ ] User can switch candidate diff views.
- [ ] Large diff rendering remains usable.
- [ ] Fallback message shown when diff unavailable.

## Out of Scope
semantic diff highlighting; inline commenting.

## Dependencies
- M3.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.6] Experimental Adapter UX Warnings

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: S
- Dependencies: M2.1, M2.9, M3.2

### Issue Body (Markdown)

```md
## Problem
The GUI must clearly distinguish Tier-1 from experimental adapters. Without visual warnings and explicit opt-in flows, users may accidentally include unstable adapters in production runs.

## Scope
Add experimental labels and warning badges to adapter selection UI. Require explicit risk confirmation before including experimental adapters. Ensure Tier-1 adapters are always the default selections.

## Acceptance Criteria
- [ ] Experimental adapters are clearly labeled.
- [ ] Opt-in flow includes risk confirmation.
- [ ] Tier-1 adapters remain default selections.

## Out of Scope
adapter configuration UI; custom adapter registration.

## Dependencies
- M2.1, M2.9, M3.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M3.7] GUI Smoke Test Pack

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-test, type-test
- Estimate: M
- Dependencies: M3.3, M3.4, M3.5

### Issue Body (Markdown)

```md
## Problem
GUI functionality needs automated validation to prevent regressions. Without smoke tests, visual and interaction bugs may ship undetected.

## Scope
Write smoke tests covering app startup, race launch and completion path, and merge action in dry-run mode. Run on Linux and Windows CI.

## Acceptance Criteria
- [ ] Startup test passes on Linux and Windows.
- [ ] Race launch and completion path validated.
- [ ] Merge action UI path tested in dry-run mode.

## Out of Scope
full E2E browser tests; accessibility audit.

## Dependencies
- M3.3, M3.4, M3.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M4.1] Workflow Engine Core

- Phase: Phase 4 Tickets (Collaboration Workflows)
- Labels: hydra, phase-4, area-workflow, type-feature
- Estimate: M
- Dependencies: M2.10

### Issue Body (Markdown)

```md
## Problem
Race mode only supports independent parallel execution. Structured cooperation patterns (builder/reviewer, specialization, iterative refinement) require a DAG-based workflow engine that manages step execution, artifact passing, and conditional branching.

## Scope
Implement a DAG step executor that runs workflow nodes sequentially or in parallel based on graph structure. Support artifact passing between nodes via immutable artifact IDs. Honor per-node timeout and retry policies. Persist workflow run summary.

## Acceptance Criteria
- [ ] DAG step executor supports artifacts and statuses.
- [ ] Node timeout/retry policies are honored.
- [ ] Workflow run summary is persisted.

## Out of Scope
visual workflow editor; custom node types.

## Dependencies
- M2.10

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M4.2] Builder-Reviewer-Refiner Preset

- Phase: Phase 4 Tickets (Collaboration Workflows)
- Labels: hydra, phase-4, area-workflow, type-feature
- Estimate: M
- Dependencies: M4.1

### Issue Body (Markdown)

```md
## Problem
The builder-reviewer-refiner pattern is a common code quality improvement loop, but there is no preset that orchestrates it. Users would have to manually chain agent runs and pass artifacts between them.

## Scope
Implement the builder-reviewer-refiner workflow preset. Builder generates code, reviewer critiques via structured rubric, refiner applies feedback. Persist reviewer artifact for reuse. Score and gate the final output.

## Acceptance Criteria
- [ ] Preset runs end-to-end from CLI.
- [ ] Reviewer artifact is persisted and reusable.
- [ ] Final output is scored and gated.

## Out of Scope
multi-round review loops; reviewer read-only enforcement.

## Dependencies
- M4.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M4.3] Specialization Preset

- Phase: Phase 4 Tickets (Collaboration Workflows)
- Labels: hydra, phase-4, area-workflow, type-feature
- Estimate: M
- Dependencies: M4.1

### Issue Body (Markdown)

```md
## Problem
Some features naturally split into bounded scopes (e.g., backend + frontend). Without a specialization preset, users cannot assign different agents to different scopes and then integrate results automatically.

## Scope
Implement the specialization workflow preset. Create shared contract artifact, launch parallel scoped agent tasks, detect out-of-scope edits, merge specialized branches into integration branch, and score the result.

## Acceptance Criteria
- [ ] Parallel scoped tasks run in separate branches.
- [ ] Out-of-scope edits are detected and reported.
- [ ] Integration branch result is scored.

## Out of Scope
automatic path-revert for out-of-scope edits; dynamic scope assignment.

## Dependencies
- M4.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M4.4] Iterative Refinement Preset

- Phase: Phase 4 Tickets (Collaboration Workflows)
- Labels: hydra, phase-4, area-workflow, type-feature
- Estimate: M
- Dependencies: M4.1, M2.7

### Issue Body (Markdown)

```md
## Problem
A single agent pass may not achieve the desired quality threshold. Iterative refinement uses scoring feedback as a correction signal, but without a preset, users must manually re-run agents with synthesized prompts.

## Scope
Implement the iterative refinement workflow preset. Run agent, score result, synthesize refinement prompt from failures, repeat until threshold or max iterations. Include convergence guard (stop if score decreases twice or no improvement after N iterations). Persist iteration history.

## Acceptance Criteria
- [ ] Refinement loop uses structured score failures.
- [ ] Convergence guard prevents endless loops.
- [ ] Iteration history artifacts are persisted.

## Out of Scope
cross-agent iteration (switching agents between iterations); auto-tuning thresholds.

## Dependencies
- M4.1, M2.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M4.5] Workflow CLI and GUI Timeline

- Phase: Phase 4 Tickets (Collaboration Workflows)
- Labels: hydra, phase-4, area-ui, type-feature
- Estimate: M
- Dependencies: M4.2, M4.3, M4.4

### Issue Body (Markdown)

```md
## Problem
Workflow execution involves multiple steps with dependencies and artifacts. Without a timeline view, users cannot track progress, understand step relationships, or diagnose failures across the workflow.

## Scope
Add CLI step timeline with per-node status indicators. Add GUI node timeline view with artifact links and drilldown. Include retry guidance in failure states.

## Acceptance Criteria
- [ ] CLI prints step timeline with statuses.
- [ ] GUI shows node timeline and artifact links.
- [ ] Failure states include retry guidance.

## Out of Scope
drag-and-drop workflow editing; real-time timeline animation.

## Dependencies
- M4.2, M4.3, M4.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M4.6] Workflow Integration Tests

- Phase: Phase 4 Tickets (Collaboration Workflows)
- Labels: hydra, phase-4, area-test, type-test
- Estimate: M
- Dependencies: M4.2, M4.3, M4.4

### Issue Body (Markdown)

```md
## Problem
Workflow presets involve complex multi-step interactions that can fail in non-obvious ways. Without dedicated integration tests, workflow regressions may go undetected.

## Scope
Write one golden-path and one failure-path integration test per workflow preset. Add deterministic artifact graph snapshot tests to detect structural regressions.

## Acceptance Criteria
- [ ] One golden-path test per workflow preset.
- [ ] One failure-path test per preset.
- [ ] Artifact graph snapshot test is stable.

## Out of Scope
performance benchmarks; fuzz testing.

## Dependencies
- M4.2, M4.3, M4.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M5.1] ConPTY and Process Control Validation

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-test
- Estimate: M
- Dependencies: M3.7

### Issue Body (Markdown)

```md
## Problem
PTY behavior on Windows (ConPTY) differs from Unix and has not been validated under real workloads. Process termination semantics, orphan process prevention, and ANSI rendering may behave differently than on Linux.

## Scope
Validate PTY and fallback stream paths on Windows. Test cancel/timeout behavior with real agent CLIs. Verify no orphan processes remain after cancellation. Document any Windows-specific behavior differences.

## Acceptance Criteria
- [ ] PTY and fallback stream paths both tested.
- [ ] Cancel/timeout behavior verified on Windows.
- [ ] No orphan process remains after cancellation.

## Out of Scope
macOS PTY testing; custom terminal emulator support.

## Dependencies
- M3.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M5.2] Path and Filesystem Edge Cases

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-feature
- Estimate: M
- Dependencies: M1.3

### Issue Body (Markdown)

```md
## Problem
Windows has distinct path length limits (260 chars default), separator conventions, and file locking behavior that can cause failures in worktree creation, artifact writes, and cleanup operations.

## Scope
Test and fix long path handling, paths with spaces and Unicode characters, and artifact writes under locked file conditions. Ensure all filesystem operations use OS-safe path construction.

## Acceptance Criteria
- [ ] Long path handling tests pass.
- [ ] Space/Unicode path cases are covered.
- [ ] Artifact writes are robust under locked files.

## Out of Scope
network drive support; junction point handling.

## Dependencies
- M1.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M5.3] Crash Recovery and Resume Metadata

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-feature
- Estimate: M
- Dependencies: M2.10

### Issue Body (Markdown)

```md
## Problem
Interrupted runs (system crash, power loss, OOM kill) can leave the `.hydra/` directory in an inconsistent state with stale worktrees, partial artifacts, and incomplete manifests. Users need tools to inspect and recover from these states.

## Scope
Add recovery metadata to run manifests. Implement a cleanup tool that detects and reconciles stale state (orphaned worktrees, incomplete runs). Ensure interrupted runs are inspectable post-crash.

## Acceptance Criteria
- [ ] Interrupted runs can be inspected post-crash.
- [ ] Cleanup tool can reconcile stale state.
- [ ] Recovery metadata is included in run manifest.

## Out of Scope
automatic run resumption; partial result scoring.

## Dependencies
- M2.10

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M5.4] Packaging and Release Automation

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-release, type-feature
- Estimate: M
- Dependencies: M5.1, M5.2

### Issue Body (Markdown)

```md
## Problem
There is no automated pipeline for producing versioned release artifacts. Manual packaging is error-prone and blocks release cadence.

## Scope
Set up CI/CD release pipeline for Linux and Windows. Produce versioned binaries with checksums. Generate release notes from milestone labels. Define version numbering scheme.

## Acceptance Criteria
- [ ] Versioned builds produced for Linux and Windows.
- [ ] Release artifacts include checksums.
- [ ] Release notes generated from milestone labels.

## Out of Scope
macOS builds; Homebrew formula; auto-update mechanism.

## Dependencies
- M5.1, M5.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M5.5] Release Candidate Acceptance Suite

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-test, type-test
- Estimate: M
- Dependencies: M5.1, M5.2, M5.3, M5.4

### Issue Body (Markdown)

```md
## Problem
There is no comprehensive acceptance test that validates the full product surface before release. Without a release gate, regressions in core flows could ship to users.

## Scope
Write an acceptance test suite covering Tier-1 race and merge paths on Linux and Windows. Verify experimental adapter behavior remains opt-in. Confirm no P0 bugs are open at RC cut.

## Acceptance Criteria
- [ ] Tier-1 race and merge path pass on Linux/Windows.
- [ ] Experimental adapter behavior remains opt-in.
- [ ] No P0 bugs open at RC cut.

## Out of Scope
performance regression tests; security audit.

## Dependencies
- M5.1, M5.2, M5.3, M5.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## [M5.6] Artifact and Schema Migration Strategy

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-feature
- Estimate: M
- Dependencies: M2.12, M5.3

### Issue Body (Markdown)

```md
## Problem
As Hydra evolves, the artifact format (manifest.json, events.jsonl, score output) and configuration schema (hydra.toml) will change. Without a migration strategy, users upgrading Hydra may encounter broken run history, unreadable artifacts, or invalid configuration files.

## Scope
Implement versioned manifest and event schema with forward-compatibility rules. Add a migration tool that upgrades older artifacts/configs to current schema. Write forward/backward compatibility tests for at least one schema transition. Document upgrade path in release notes.

## Acceptance Criteria
- [ ] Schema version is checked on artifact read and config parse.
- [ ] Migration tool upgrades v1 artifacts/configs to current format.
- [ ] Forward and backward compatibility tests pass for at least one schema transition.
- [ ] Upgrade path is documented.

## Out of Scope
automatic background migration; multi-version concurrent support.

## Dependencies
- M2.12, M5.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## Coverage Check

- Total issues generated: 47
- Expected range: `M0.1` through `M5.6`
