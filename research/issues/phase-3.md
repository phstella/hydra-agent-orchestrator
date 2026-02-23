# Phase 3 Tickets (GUI Alpha) Issue Bodies

Last updated: 2026-02-23

Generated from `research/github-issues.md`.

Global label prefix: `hydra`

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


