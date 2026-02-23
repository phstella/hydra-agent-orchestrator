# Phase 5 Collaboration Workflows Implementation Guide

Last updated: 2026-02-23

## Why this doc exists

`planning/issues/phase-5.md` defines acceptance criteria, but implementation
agents still need concrete guidance for:

1. Workflow runtime boundaries and ownership across crates.
2. Workflow artifact and timeline contracts.
3. CLI/GUI contract shape for timeline and artifact drilldown.
4. Milestone-level execution order and negative test expectations.

This guide is the implementation-level contract for `M5.1` through `M5.6`.

## Non-negotiable invariants

1. Deterministic race mode remains intact and unchanged.
2. Workflow steps communicate through persisted artifacts, not shared mutable state.
3. Workflow failure in one node does not corrupt unrelated node artifacts.
4. Human merge confirmation remains explicit; workflows cannot silently merge.
5. Workflow logs/artifacts must pass existing secret-redaction guarantees.

## Current repo reality (confirmed)

Implemented:

- Race orchestration, scoring, and merge semantics in `hydra-cli`.
- Stable run artifacts (`manifest.json`, `events.jsonl`, agent artifacts).
- Tauri race IPC (`start_race`, `poll_race_events`, `get_race_result`).
- GUI tabs for preflight, race, results, and review.

Missing for Phase 5:

- No workflow runtime in `hydra-core`.
- No workflow CLI subcommand in `hydra-cli`.
- No workflow timeline/artifact IPC in `hydra-app`.
- No workflow timeline UI in frontend.
- No workflow preset integration tests.

## Architecture boundary

- Workflow engine and presets live in `hydra-core`.
- CLI orchestrates workflow runs and prints timeline in `hydra-cli`.
- GUI consumes typed workflow IPC from `hydra-app`; it does not execute workflow
  logic directly.
- Existing race flows stay separate and callable as building blocks.
- Interactive sessions (Phase 4) remain a separate mode and are not a hidden
  dependency for baseline workflow execution.

## Recommended implementation touchpoints

Core/runtime:
- `crates/hydra-core/src/lib.rs`
- `crates/hydra-core/src/artifact/layout.rs` (new workflow helpers)
- `crates/hydra-core/src/artifact/mod.rs`
- `crates/hydra-core/src/artifact/events.rs` (workflow event kinds)
- `crates/hydra-core/src/scoring/` (reuse scoring and gates)
- `crates/hydra-core/src/workflow/` (new module tree)

CLI:
- `crates/hydra-cli/src/main.rs`
- `crates/hydra-cli/src/race.rs` (optional helper reuse only)
- `crates/hydra-cli/src/run.rs` (workflow inspection support)
- `crates/hydra-cli/src/workflow.rs` (new command module)

GUI backend:
- `crates/hydra-app/src/state.rs` (workflow runtime state)
- `crates/hydra-app/src/ipc_types.rs` (workflow IPC DTOs)
- `crates/hydra-app/src/commands.rs` (workflow command handlers)
- `crates/hydra-app/src/main.rs` (register commands)

Frontend:
- `crates/hydra-app/frontend/src/App.tsx`
- `crates/hydra-app/frontend/src/types.ts`
- `crates/hydra-app/frontend/src/ipc.ts`
- `crates/hydra-app/frontend/src/components/` (workflow timeline components)
- `crates/hydra-app/frontend/src/__tests__/smoke.test.tsx`

## Artifact contract (target)

Keep top-level run contract stable; add workflow-specific subtree:

`.hydra/runs/<run_id>/workflow/`
- `workflow_manifest.json`
- `timeline.jsonl`
- `nodes/<node_id>/request.json`
- `nodes/<node_id>/result.json`
- `nodes/<node_id>/logs/stdout.log`
- `nodes/<node_id>/logs/stderr.log`
- `artifacts/<artifact_id>.json` (or `.patch`, `.txt` as appropriate)

Design rules:

1. Node result files are append-safe and idempotent.
2. Artifact IDs are immutable and referenced by ID, not by ad-hoc path.
3. Timeline entries are append-only and timestamped.
4. Workflow artifact writes must use the same redaction path as existing event writes.

## Runtime model contract (target)

Core types:

`WorkflowSpec`
- `workflowId: string`
- `preset: string`
- `nodes: WorkflowNodeSpec[]`
- `edges: WorkflowEdge[]`
- `policy: WorkflowPolicy`

`WorkflowNodeSpec`
- `nodeId: string`
- `nodeType: "agent_task" | "score_gate" | "merge_step" | "human_gate"`
- `inputs: string[]` (artifact IDs)
- `timeoutMs: number | null`
- `retries: number`
- `config: serde_json::Value`

`WorkflowNodeResult`
- `nodeId: string`
- `status: "pending" | "running" | "completed" | "failed" | "skipped"`
- `startedAt: string | null`
- `endedAt: string | null`
- `producedArtifacts: string[]`
- `error: string | null`

Runtime rules:

1. Validate DAG acyclicity before execution.
2. Execute only when dependencies are completed.
3. Retries create new timeline events but update one canonical node status.
4. Workflow final status is `completed` only when required nodes succeed.

## CLI contract (target)

Add workflow command family:

1. `hydra workflow run --preset <name> --prompt <text> [--json]`
2. `hydra workflow show --run-id <uuid> [--json]`
3. `hydra workflow timeline --run-id <uuid> [--json]`

`workflow run --json` minimum payload:
- `run_id`
- `workflow_id`
- `status`
- `nodes` (id + status + retries_used)
- `artifacts_path`
- `duration_ms`

Non-JSON mode must print:

1. Step timeline with stable ordering.
2. Node status and duration.
3. Final outcome with actionable failure details.

## GUI IPC contract (target)

Commands:

1. `start_workflow(request) -> WorkflowStarted`
2. `poll_workflow_events(run_id, cursor) -> WorkflowEventBatch`
3. `get_workflow_result(run_id) -> Option<WorkflowResult>`
4. `get_workflow_timeline(run_id) -> WorkflowTimelinePayload`
5. `get_workflow_artifact(run_id, artifact_id) -> WorkflowArtifactPayload`

Suggested types:

`WorkflowStartRequest`
- `preset: string`
- `taskPrompt: string`
- `agents: string[]`
- `allowExperimental: boolean`

`WorkflowTimelineNode`
- `nodeId: string`
- `title: string`
- `status: string`
- `startedAt: string | null`
- `endedAt: string | null`
- `artifactIds: string[]`

`WorkflowArtifactPayload`
- `runId: string`
- `artifactId: string`
- `artifactType: string`
- `content: string`
- `truncated: boolean`
- `warning: string | null`

## Milestone execution details

### M5.1 Workflow Engine Core

Implementation details:

1. Create `workflow` module in `hydra-core` with graph validation and executor.
2. Add node scheduler with dependency-aware readiness checks.
3. Persist workflow manifest and timeline artifacts under run namespace.
4. Define error taxonomy for node timeout/retry/validation failures.

Negative tests:

- Cyclic graph is rejected with actionable error.
- Missing artifact dependency fails node deterministically.
- Retry exhaustion marks node failed and workflow terminal.

### M5.2 Builder-Reviewer-Refiner Preset

Implementation details:

1. Define preset node graph (builder -> reviewer -> refiner -> score_gate).
2. Persist reviewer critique as structured artifact (`severity`, `file`, `recommendation`).
3. Refiner prompt must include reviewer artifact IDs and summaries.
4. Apply existing mergeability gates to final node output.

Negative tests:

- Reviewer malformed output falls back to raw-text artifact with warning.
- Missing reviewer artifact blocks refiner node.
- Final score below threshold marks workflow non-mergeable.

### M5.3 Specialization Preset

Implementation details:

1. Generate shared contract artifact before parallel specialization nodes.
2. Run scoped nodes in parallel with declared path ownership.
3. Enforce out-of-scope edit detection against declared scope paths.
4. Persist integration report artifact summarizing conflicts and scope violations.

Negative tests:

- Out-of-scope edit produces violation event and gated status.
- Integration conflict persists conflict artifact and halts merge node.
- Missing shared contract artifact blocks specialization node start.

### M5.4 Iterative Refinement Preset

Implementation details:

1. Implement loop controller with explicit max-iteration bound.
2. Build refinement prompts from failing score dimensions and evidence.
3. Add convergence guard (decrease twice, no improvement over N iterations).
4. Persist per-iteration summary artifacts and final convergence reason.

Negative tests:

- Infinite-loop guard triggers with deterministic stop reason.
- Score parser failure creates fallback artifact and stops loop cleanly.
- Budget exceeded marks remaining iterations skipped.

### M5.5 Workflow CLI and GUI Timeline

Implementation details:

1. Add CLI timeline output for each node state transition.
2. Add GUI workflow timeline view with node statuses and artifact links.
3. Add artifact drilldown panel supporting text/json/diff artifact types.
4. Keep design-token compliance (no hardcoded colors) in new components.

Recommended frontend components:

- `WorkflowTimelineView.tsx`
- `WorkflowNodeCard.tsx`
- `WorkflowArtifactDrawer.tsx`

Negative tests:

- Unknown node status renders safe fallback badge.
- Missing artifact content shows explicit unavailable state.
- Timeline polling handles cursor gaps without UI freeze.

### M5.6 Workflow Integration Tests

Implementation details:

1. One golden-path integration test per preset (`M5.2`-`M5.4`).
2. One failure-path integration test per preset.
3. Artifact graph snapshot tests with stable ordering.
4. Frontend smoke tests cover timeline and artifact drilldown states.

Required test coverage:

- Builder-reviewer-refiner: reviewer failure fallback path.
- Specialization: out-of-scope violation path.
- Iterative refinement: convergence guard termination path.

## Test matrix (must pass)

Backend:

1. `cargo test --workspace --locked --offline`
2. `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`

CLI targeted:

1. `cargo test -p hydra-cli --locked --offline`
2. `cargo test -p hydra-core --locked --offline`

GUI:

1. `cargo test --manifest-path crates/hydra-app/Cargo.toml --locked --offline`
2. `npm run lint` in `crates/hydra-app/frontend`
3. `npm run test:smoke` in `crates/hydra-app/frontend`

## Ticket-to-deliverable mapping

M5.1:
- workflow engine core, DAG validation, artifact/timeline persistence

M5.2-M5.4:
- three presets with deterministic node behavior and workflow-level gating

M5.5:
- timeline surfaces in CLI + GUI with artifact drilldown

M5.6:
- integration and smoke tests that prevent preset regressions

## Done evidence template

Attach to each Phase 5 ticket closure:

1. Commit(s) and touched files.
2. One workflow timeline sample (`timeline.jsonl`) with node transitions.
3. One preset artifact sample (review/scope/refinement).
4. Test outputs (commands + pass/fail + duration).
5. Screenshot or short recording for GUI timeline milestones (`M5.5`).

## Explicit non-goals for Phase 5

1. No visual drag-and-drop workflow editor.
2. No remote/multi-user collaborative editing semantics.
3. No implicit background merge without explicit user action.
