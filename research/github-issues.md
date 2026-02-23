# GitHub Issue Body Pack

Last updated: 2026-02-23

This file contains copy-paste-ready issue bodies generated from `research/implementation-checklist.md` milestones `M0.1` through `M5.5`.

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

## [M1.1] Core Workspace Scaffold

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Implement milestone M1.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.1.

## Acceptance Criteria
- [ ] Workspace builds with hydra-core and hydra-cli crates.
- [ ] Logging and error crates wired consistently.
- [ ] CI compiles on Linux and Windows.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.2.

## Acceptance Criteria
- [ ] hydra.toml parses with schema validation.
- [ ] Missing optional fields get deterministic defaults.
- [ ] Invalid config returns actionable error messages.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.3.

## Acceptance Criteria
- [ ] Create/list/remove worktree operations are implemented.
- [ ] Interrupt-safe cleanup path exists.
- [ ] Windows path handling tests pass.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.4.

## Acceptance Criteria
- [ ] Supports start, stream, timeout, cancel.
- [ ] Bounded output buffering prevents memory blowups.
- [ ] Emits normalized lifecycle events.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.5.

## Acceptance Criteria
- [ ] claude runs in isolated worktree.
- [ ] Stream parser maps key events to normalized schema.
- [ ] Timeout and cancellation are covered by tests.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.6 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.6.

## Acceptance Criteria
- [ ] codex exec works in isolated worktree.
- [ ] JSON stream parser maps events and usage data.
- [ ] Unsupported flag fallback logic is tested.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.7 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.7.

## Acceptance Criteria
- [ ] hydra race --agents claude completes end-to-end.
- [ ] Run summary includes branch and artifact path.
- [ ] Non-zero exit codes on fatal failures.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.8 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.8.

## Acceptance Criteria
- [ ] Ctrl+C cleanup test passes.
- [ ] Partial failure leaves usable artifacts.
- [ ] No orphan worktrees after test run.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.1.

## Acceptance Criteria
- [ ] Registry supports Tier-1 and experimental tiers.
- [ ] Default run selects only Tier-1 adapters.
- [ ] Experimental adapters require explicit opt-in flag.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.2.

## Acceptance Criteria
- [ ] Two Tier-1 agents run concurrently.
- [ ] One agent failure does not kill others.
- [ ] Aggregate run status is deterministic.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.3.

## Acceptance Criteria
- [ ] Build/test/lint baseline captured once per run.
- [ ] Baseline outputs persisted as artifacts.
- [ ] Missing commands handled with explicit status.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.4.

## Acceptance Criteria
- [ ] Build score computed per candidate.
- [ ] Timeout and command failure paths tested.
- [ ] Score payload includes raw evidence references.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.5.

## Acceptance Criteria
- [ ] Regression-aware formula implemented.
- [ ] Parser fallback to exit-code mode works.
- [ ] Test-drop anti-gaming checks included.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.6 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.6.

## Acceptance Criteria
- [ ] Lint delta scoring implemented.
- [ ] Diff scope scoring includes file/churn checks.
- [ ] Protected path penalty is configurable.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.7 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.7.

## Acceptance Criteria
- [ ] Weighted composite scores are reproducible.
- [ ] Missing dimensions renormalize weights.
- [ ] Mergeability gates are exposed in output.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.8 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.8.

## Acceptance Criteria
- [ ] Dry-run reports potential conflicts.
- [ ] Real merge requires explicit confirmation flag.
- [ ] Conflict report artifact is written on failure.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M2.9 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.9.

## Acceptance Criteria
- [ ] Cursor can run only with --allow-experimental-adapters.
- [ ] Output labels include experimental warning.
- [ ] Failing probe blocks runtime activation.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
- Dependencies: M2.2 to M2.8

### Issue Body (Markdown)

```md
## Problem
Implement milestone M2.10 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M2.10.

## Acceptance Criteria
- [ ] Full race test verifies ranking output shape.
- [ ] Artifacts are complete and replayable.
- [ ] Linux and Windows CI jobs pass.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M2.2 to M2.8

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
Implement milestone M3.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.1.

## Acceptance Criteria
- [ ] GUI launches and can query backend health.
- [ ] Shared types compile on frontend and backend.
- [ ] Linux packaging smoke test passes.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M3.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.2.

## Acceptance Criteria
- [ ] Start race and fetch results via IPC.
- [ ] Error mapping is human-readable.
- [ ] Backpressure does not freeze UI.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M3.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.3.

## Acceptance Criteria
- [ ] One panel per running agent.
- [ ] Status badges track lifecycle changes.
- [ ] Stream rendering remains responsive under load.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M3.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.4.

## Acceptance Criteria
- [ ] Ranked cards show score breakdown.
- [ ] Non-mergeable candidates are visually blocked.
- [ ] Winner selection action is explicit.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M3.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.5.

## Acceptance Criteria
- [ ] User can switch candidate diff views.
- [ ] Large diff rendering remains usable.
- [ ] Fallback message shown when diff unavailable.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M3.6 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.6.

## Acceptance Criteria
- [ ] Experimental adapters are clearly labeled.
- [ ] Opt-in flow includes risk confirmation.
- [ ] Tier-1 adapters remain default selections.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M3.7 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M3.7.

## Acceptance Criteria
- [ ] Startup test passes on Linux and Windows.
- [ ] Race launch and completion path validated.
- [ ] Merge action UI path tested in dry-run mode.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M4.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M4.1.

## Acceptance Criteria
- [ ] DAG step executor supports artifacts and statuses.
- [ ] Node timeout/retry policies are honored.
- [ ] Workflow run summary is persisted.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M4.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M4.2.

## Acceptance Criteria
- [ ] Preset runs end-to-end from CLI.
- [ ] Reviewer artifact is persisted and reusable.
- [ ] Final output is scored and gated.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M4.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M4.3.

## Acceptance Criteria
- [ ] Parallel scoped tasks run in separate branches.
- [ ] Out-of-scope edits are detected and reported.
- [ ] Integration branch result is scored.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M4.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M4.4.

## Acceptance Criteria
- [ ] Refinement loop uses structured score failures.
- [ ] Convergence guard prevents endless loops.
- [ ] Iteration history artifacts are persisted.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M4.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M4.5.

## Acceptance Criteria
- [ ] CLI prints step timeline with statuses.
- [ ] GUI shows node timeline and artifact links.
- [ ] Failure states include retry guidance.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M4.6 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M4.6.

## Acceptance Criteria
- [ ] One golden-path test per workflow preset.
- [ ] One failure-path test per preset.
- [ ] Artifact graph snapshot test is stable.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M5.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.1.

## Acceptance Criteria
- [ ] PTY and fallback stream paths both tested.
- [ ] Cancel/timeout behavior verified on Windows.
- [ ] No orphan process remains after cancellation.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M5.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.2.

## Acceptance Criteria
- [ ] Long path handling tests pass.
- [ ] Space/Unicode path cases are covered.
- [ ] Artifact writes are robust under locked files.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M5.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.3.

## Acceptance Criteria
- [ ] Interrupted runs can be inspected post-crash.
- [ ] Cleanup tool can reconcile stale state.
- [ ] Recovery metadata is included in run manifest.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M5.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.4.

## Acceptance Criteria
- [ ] Versioned builds produced for Linux and Windows.
- [ ] Release artifacts include checksums.
- [ ] Release notes generated from milestone labels.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
- Dependencies: M5.1 to M5.4

### Issue Body (Markdown)

```md
## Problem
Implement milestone M5.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.5.

## Acceptance Criteria
- [ ] Tier-1 race and merge path pass on Linux/Windows.
- [ ] Experimental adapter behavior remains opt-in.
- [ ] No P0 bugs open at RC cut.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M5.1 to M5.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## Coverage Check

- Total issues generated: 42
- Expected range: `M0.1` through `M5.5`
