# Phase 4 Tickets (Collaboration Workflows) Issue Bodies

Last updated: 2026-02-23

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


