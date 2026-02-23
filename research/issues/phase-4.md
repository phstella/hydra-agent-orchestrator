# Phase 4 Tickets (Collaboration Workflows) Issue Bodies

Last updated: 2026-02-22

Generated from `research/github-issues.md`.

Global label prefix: `hydra`

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
- Visual workflow editor.
- Custom node types.

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
- Multi-round review loops.
- Reviewer read-only enforcement.

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
- Automatic path-revert for out-of-scope edits.
- Dynamic scope assignment.

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
- Cross-agent iteration (switching agents between iterations).
- Auto-tuning thresholds.

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
- Drag-and-drop workflow editing.
- Real-time timeline animation.

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
- Performance benchmarks.
- Fuzz testing.

## Dependencies
- M4.2, M4.3, M4.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```
