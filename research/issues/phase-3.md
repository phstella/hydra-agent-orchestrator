# Phase 3 Tickets (GUI Alpha) Issue Bodies

Last updated: 2026-02-22

Generated from `research/implementation-checklist.md`.

Global label prefix: `hydra`

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
Functional race UI; Windows packaging.

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
Workflow IPC commands; settings UI.

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
Log search/filter; output export.

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
Score comparison across runs; score trend charts.

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
Semantic diff highlighting; inline commenting.

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
Adapter configuration UI; custom adapter registration.

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
Full E2E browser tests; accessibility audit.

## Dependencies
- M3.3, M3.4, M3.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```
