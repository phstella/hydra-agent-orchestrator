# Phase 2 Tickets (Multi-Agent Race + Scoring) Issue Bodies

Last updated: 2026-02-23

Generated from `research/github-issues.md`.

Global label prefix: `hydra`

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


