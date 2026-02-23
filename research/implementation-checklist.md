# Implementation Checklist (Issue-Ready)

Last updated: 2026-02-23

## 1. Locked Product Decisions

1. Tier-1 adapters at launch are `claude` and `codex`.
2. `cursor-agent` is Tier-2 experimental until probe and conformance gates pass.
3. Linux is release blocker for all core flows.
4. Windows parity is required before v1 release candidate.

## 2. Issue Template (Use For Every Ticket)

```md
Title: [M#.##] <short outcome-focused title>
Labels: hydra, phase-<n>, area-<core|adapter|ui|scoring|workflow>, type-<feature|chore|test>
Estimate: <S|M|L>
Dependencies: <ticket IDs or none>

Problem
- What is broken or missing now.

Scope
- Exact implementation boundary.

Acceptance Criteria
- Observable pass/fail checks.

Out of Scope
- Explicit non-goals for this ticket.
```

## 3. Phase 0 Tickets (Validation and Guardrails)

### M0.1 Adapter Probe Framework

- Labels: `phase-0`, `area-adapter`, `type-feature`
- Estimate: `M`
- Dependencies: none
- Problem: Adapter assumptions can drift silently.
- Scope: Add probe interface and unified probe report model.
- Acceptance Criteria:
1. `hydra doctor` emits JSON report with adapter probe status.
2. Probe output includes binary path, version, supported flags, confidence.
3. Unknown adapters do not crash doctor command.
- Out of Scope: Full adapter execution.

### M0.2 Claude Probe Implementation

- Labels: `phase-0`, `area-adapter`, `type-feature`
- Estimate: `S`
- Dependencies: `M0.1`
- Problem: Tier-1 adapter must be validated at startup.
- Scope: Implement Claude probe for required headless flags.
- Acceptance Criteria:
1. Probe verifies `-p` and `--output-format` support.
2. Probe result status is `ready` or `blocked` with clear reason.
3. Fixture-based probe test passes in CI.
- Out of Scope: runtime parsing logic.

### M0.3 Codex Probe Implementation

- Labels: `phase-0`, `area-adapter`, `type-feature`
- Estimate: `S`
- Dependencies: `M0.1`
- Problem: Tier-1 adapter must be validated at startup.
- Scope: Implement Codex probe for `exec`, `--json`, and approval mode flags.
- Acceptance Criteria:
1. Probe verifies `exec` subcommand exists.
2. Probe verifies JSON output flag support.
3. Probe handles known flag variants without panic.
- Out of Scope: full scoring integration.

### M0.4 Cursor Experimental Probe

- Labels: `phase-0`, `area-adapter`, `type-feature`
- Estimate: `M`
- Dependencies: `M0.1`
- Problem: Cursor interface variability must not break default flows.
- Scope: Add Cursor probe with experimental classification.
- Acceptance Criteria:
1. Cursor probe never promotes adapter to Tier-1.
2. Probe result can be `experimental-ready`, `experimental-blocked`, or `missing`.
3. UI and CLI mark adapter as experimental.
- Out of Scope: enabling Cursor by default.

### M0.5 Run Artifact Convention

- Labels: `phase-0`, `area-core`, `type-feature`
- Estimate: `S`
- Dependencies: none
- Problem: Runs need deterministic artifact paths for replay.
- Scope: Define `.hydra/runs/<run_id>/` structure and write metadata manifest.
- Acceptance Criteria:
1. Every run writes `manifest.json` and `events.jsonl`.
2. Artifact paths are OS-safe on Linux and Windows.
3. Cleanup policy respects retention config.
- Out of Scope: GUI history viewer.

### M0.6 Doctor Command MVP

- Labels: `phase-0`, `area-core`, `type-feature`
- Estimate: `S`
- Dependencies: `M0.1`, `M0.2`, `M0.3`, `M0.4`
- Problem: Users need quick readiness check before run.
- Scope: Implement `hydra doctor` summary + JSON output mode.
- Acceptance Criteria:
1. Exit code is non-zero when Tier-1 prerequisites fail.
2. Output includes git repo checks and adapter readiness.
3. `--json` output is stable and parseable.
- Out of Scope: auto-fix behavior.

## 4. Phase 1 Tickets (Core Orchestrator + Single Agent)

### M1.1 Core Workspace Scaffold

- Labels: `phase-1`, `area-core`, `type-feature`
- Estimate: `S`
- Dependencies: none
- Acceptance Criteria:
1. Workspace builds with `hydra-core` and `hydra-cli` crates.
2. Logging and error crates wired consistently.
3. CI compiles on Linux and Windows.

### M1.2 Config Parser and Defaults

- Labels: `phase-1`, `area-core`, `type-feature`
- Estimate: `S`
- Dependencies: `M1.1`
- Acceptance Criteria:
1. `hydra.toml` parses with schema validation.
2. Missing optional fields get deterministic defaults.
3. Invalid config returns actionable error messages.

### M1.3 Worktree Lifecycle Service

- Labels: `phase-1`, `area-core`, `type-feature`
- Estimate: `M`
- Dependencies: `M1.1`
- Acceptance Criteria:
1. Create/list/remove worktree operations are implemented.
2. Interrupt-safe cleanup path exists.
3. Windows path handling tests pass.

### M1.4 Process Supervisor (Single Agent)

- Labels: `phase-1`, `area-core`, `type-feature`
- Estimate: `M`
- Dependencies: `M1.1`
- Acceptance Criteria:
1. Supports start, stream, timeout, cancel.
2. Bounded output buffering prevents memory blowups.
3. Emits normalized lifecycle events.

### M1.5 Claude Adapter Runtime Path

- Labels: `phase-1`, `area-adapter`, `type-feature`
- Estimate: `M`
- Dependencies: `M0.2`, `M1.4`
- Acceptance Criteria:
1. `claude` runs in isolated worktree.
2. Stream parser maps key events to normalized schema.
3. Timeout and cancellation are covered by tests.

### M1.6 Codex Adapter Runtime Path

- Labels: `phase-1`, `area-adapter`, `type-feature`
- Estimate: `M`
- Dependencies: `M0.3`, `M1.4`
- Acceptance Criteria:
1. `codex exec` works in isolated worktree.
2. JSON stream parser maps events and usage data.
3. Unsupported flag fallback logic is tested.

### M1.7 CLI Race Command (Single Agent)

- Labels: `phase-1`, `area-core`, `type-feature`
- Estimate: `S`
- Dependencies: `M1.2`, `M1.3`, `M1.5`
- Acceptance Criteria:
1. `hydra race --agents claude` completes end-to-end.
2. Run summary includes branch and artifact path.
3. Non-zero exit codes on fatal failures.

### M1.8 Interrupt and Recovery Tests

- Labels: `phase-1`, `area-test`, `type-test`
- Estimate: `M`
- Dependencies: `M1.3`, `M1.4`, `M1.7`
- Acceptance Criteria:
1. Ctrl+C cleanup test passes.
2. Partial failure leaves usable artifacts.
3. No orphan worktrees after test run.

## 5. Phase 2 Tickets (Multi-Agent Race + Scoring)

### M2.1 Adapter Registry and Tier Policy

- Labels: `phase-2`, `area-adapter`, `type-feature`
- Estimate: `S`
- Dependencies: `M1.6`
- Acceptance Criteria:
1. Registry supports Tier-1 and experimental tiers.
2. Default run selects only Tier-1 adapters.
3. Experimental adapters require explicit opt-in flag.

### M2.2 Parallel Spawn and Supervision

- Labels: `phase-2`, `area-core`, `type-feature`
- Estimate: `M`
- Dependencies: `M1.4`, `M2.1`
- Acceptance Criteria:
1. Two Tier-1 agents run concurrently.
2. One agent failure does not kill others.
3. Aggregate run status is deterministic.

### M2.3 Baseline Capture Engine

- Labels: `phase-2`, `area-scoring`, `type-feature`
- Estimate: `S`
- Dependencies: `M1.2`
- Acceptance Criteria:
1. Build/test/lint baseline captured once per run.
2. Baseline outputs persisted as artifacts.
3. Missing commands handled with explicit status.

### M2.4 Scoring Dimension: Build

- Labels: `phase-2`, `area-scoring`, `type-feature`
- Estimate: `S`
- Dependencies: `M2.3`
- Acceptance Criteria:
1. Build score computed per candidate.
2. Timeout and command failure paths tested.
3. Score payload includes raw evidence references.

### M2.5 Scoring Dimension: Tests

- Labels: `phase-2`, `area-scoring`, `type-feature`
- Estimate: `M`
- Dependencies: `M2.3`
- Acceptance Criteria:
1. Regression-aware formula implemented.
2. Parser fallback to exit-code mode works.
3. Test-drop anti-gaming checks included.

### M2.6 Scoring Dimension: Lint and Diff Scope

- Labels: `phase-2`, `area-scoring`, `type-feature`
- Estimate: `M`
- Dependencies: `M2.3`
- Acceptance Criteria:
1. Lint delta scoring implemented.
2. Diff scope scoring includes file/churn checks.
3. Protected path penalty is configurable.

### M2.7 Composite Ranking and Mergeability Gates

- Labels: `phase-2`, `area-scoring`, `type-feature`
- Estimate: `S`
- Dependencies: `M2.4`, `M2.5`, `M2.6`
- Acceptance Criteria:
1. Weighted composite scores are reproducible.
2. Missing dimensions renormalize weights.
3. Mergeability gates are exposed in output.

### M2.8 CLI Merge Command with Dry-Run

- Labels: `phase-2`, `area-core`, `type-feature`
- Estimate: `M`
- Dependencies: `M2.7`
- Acceptance Criteria:
1. Dry-run reports potential conflicts.
2. Real merge requires explicit confirmation flag.
3. Conflict report artifact is written on failure.

### M2.9 Experimental Cursor Opt-In Path

- Labels: `phase-2`, `area-adapter`, `type-feature`
- Estimate: `M`
- Dependencies: `M0.4`, `M2.1`, `M2.2`
- Acceptance Criteria:
1. Cursor can run only with `--allow-experimental-adapters`.
2. Output labels include experimental warning.
3. Failing probe blocks runtime activation.

### M2.10 End-to-End Race Integration Test

- Labels: `phase-2`, `area-test`, `type-test`
- Estimate: `M`
- Dependencies: `M2.2` to `M2.8`
- Acceptance Criteria:
1. Full race test verifies ranking output shape.
2. Artifacts are complete and replayable.
3. Linux and Windows CI jobs pass.

## 6. Phase 3 Tickets (GUI Alpha)

### M3.1 Tauri App Bootstrap

- Labels: `phase-3`, `area-ui`, `type-feature`
- Estimate: `S`
- Dependencies: `M1.1`
- Acceptance Criteria:
1. GUI launches and can query backend health.
2. Shared types compile on frontend and backend.
3. Linux packaging smoke test passes.

### M3.2 IPC Command Surface (Race)

- Labels: `phase-3`, `area-ui`, `type-feature`
- Estimate: `M`
- Dependencies: `M2.10`, `M3.1`
- Acceptance Criteria:
1. Start race and fetch results via IPC.
2. Error mapping is human-readable.
3. Backpressure does not freeze UI.

### M3.3 Live Agent Output Panels

- Labels: `phase-3`, `area-ui`, `type-feature`
- Estimate: `M`
- Dependencies: `M3.2`
- Acceptance Criteria:
1. One panel per running agent.
2. Status badges track lifecycle changes.
3. Stream rendering remains responsive under load.

### M3.4 Scoreboard and Mergeability UI

- Labels: `phase-3`, `area-ui`, `type-feature`
- Estimate: `S`
- Dependencies: `M2.7`, `M3.2`
- Acceptance Criteria:
1. Ranked cards show score breakdown.
2. Non-mergeable candidates are visually blocked.
3. Winner selection action is explicit.

### M3.5 Diff Viewer Integration

- Labels: `phase-3`, `area-ui`, `type-feature`
- Estimate: `M`
- Dependencies: `M3.2`
- Acceptance Criteria:
1. User can switch candidate diff views.
2. Large diff rendering remains usable.
3. Fallback message shown when diff unavailable.

### M3.6 Experimental Adapter UX Warnings

- Labels: `phase-3`, `area-ui`, `type-feature`
- Estimate: `S`
- Dependencies: `M2.1`, `M2.9`, `M3.2`
- Acceptance Criteria:
1. Experimental adapters are clearly labeled.
2. Opt-in flow includes risk confirmation.
3. Tier-1 adapters remain default selections.

### M3.7 GUI Smoke Test Pack

- Labels: `phase-3`, `area-test`, `type-test`
- Estimate: `M`
- Dependencies: `M3.3`, `M3.4`, `M3.5`
- Acceptance Criteria:
1. Startup test passes on Linux and Windows.
2. Race launch and completion path validated.
3. Merge action UI path tested in dry-run mode.

## 7. Phase 4 Tickets (Collaboration Workflows)

### M4.1 Workflow Engine Core

- Labels: `phase-4`, `area-workflow`, `type-feature`
- Estimate: `M`
- Dependencies: `M2.10`
- Acceptance Criteria:
1. DAG step executor supports artifacts and statuses.
2. Node timeout/retry policies are honored.
3. Workflow run summary is persisted.

### M4.2 Builder-Reviewer-Refiner Preset

- Labels: `phase-4`, `area-workflow`, `type-feature`
- Estimate: `M`
- Dependencies: `M4.1`
- Acceptance Criteria:
1. Preset runs end-to-end from CLI.
2. Reviewer artifact is persisted and reusable.
3. Final output is scored and gated.

### M4.3 Specialization Preset

- Labels: `phase-4`, `area-workflow`, `type-feature`
- Estimate: `M`
- Dependencies: `M4.1`
- Acceptance Criteria:
1. Parallel scoped tasks run in separate branches.
2. Out-of-scope edits are detected and reported.
3. Integration branch result is scored.

### M4.4 Iterative Refinement Preset

- Labels: `phase-4`, `area-workflow`, `type-feature`
- Estimate: `M`
- Dependencies: `M4.1`, `M2.7`
- Acceptance Criteria:
1. Refinement loop uses structured score failures.
2. Convergence guard prevents endless loops.
3. Iteration history artifacts are persisted.

### M4.5 Workflow CLI and GUI Timeline

- Labels: `phase-4`, `area-ui`, `type-feature`
- Estimate: `M`
- Dependencies: `M4.2`, `M4.3`, `M4.4`
- Acceptance Criteria:
1. CLI prints step timeline with statuses.
2. GUI shows node timeline and artifact links.
3. Failure states include retry guidance.

### M4.6 Workflow Integration Tests

- Labels: `phase-4`, `area-test`, `type-test`
- Estimate: `M`
- Dependencies: `M4.2`, `M4.3`, `M4.4`
- Acceptance Criteria:
1. One golden-path test per workflow preset.
2. One failure-path test per preset.
3. Artifact graph snapshot test is stable.

## 8. Phase 5 Tickets (Windows Parity and Release Hardening)

### M5.1 ConPTY and Process Control Validation

- Labels: `phase-5`, `area-core`, `type-test`
- Estimate: `M`
- Dependencies: `M3.7`
- Acceptance Criteria:
1. PTY and fallback stream paths both tested.
2. Cancel/timeout behavior verified on Windows.
3. No orphan process remains after cancellation.

### M5.2 Path and Filesystem Edge Cases

- Labels: `phase-5`, `area-core`, `type-feature`
- Estimate: `M`
- Dependencies: `M1.3`
- Acceptance Criteria:
1. Long path handling tests pass.
2. Space/Unicode path cases are covered.
3. Artifact writes are robust under locked files.

### M5.3 Crash Recovery and Resume Metadata

- Labels: `phase-5`, `area-core`, `type-feature`
- Estimate: `M`
- Dependencies: `M2.10`
- Acceptance Criteria:
1. Interrupted runs can be inspected post-crash.
2. Cleanup tool can reconcile stale state.
3. Recovery metadata is included in run manifest.

### M5.4 Packaging and Release Automation

- Labels: `phase-5`, `area-release`, `type-feature`
- Estimate: `M`
- Dependencies: `M5.1`, `M5.2`
- Acceptance Criteria:
1. Versioned builds produced for Linux and Windows.
2. Release artifacts include checksums.
3. Release notes generated from milestone labels.

### M5.5 Release Candidate Acceptance Suite

- Labels: `phase-5`, `area-test`, `type-test`
- Estimate: `M`
- Dependencies: `M5.1` to `M5.4`
- Acceptance Criteria:
1. Tier-1 race and merge path pass on Linux/Windows.
2. Experimental adapter behavior remains opt-in.
3. No P0 bugs open at RC cut.

## 9. Backlog Hygiene Rules

1. No ticket closes without explicit acceptance evidence.
2. Adapter-related tickets must include fixture updates.
3. Experimental adapter changes cannot alter Tier-1 defaults.
4. Every phase ends with at least one integration test ticket.
