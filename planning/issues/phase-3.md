# Phase 3 Tickets (GUI Alpha) Issue Bodies

Last updated: 2026-02-23

Generated from `planning/implementation-checklist.md`, with supplemental mockup-derived execution tickets.

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

## Mockup-Derived Supplemental Execution Tickets

These tickets are derived from approved GUI mockups (Image #1 through Image #4) and are intended to guide implementation sequencing within Phase 3.

## Global UI Theme Contract (applies to all supplemental tickets)

- Base visual direction: dark UI with green system/success emphasis.
- Highlight/accent direction: marine blue for selected/active/focus states.
- Use tokenized values only for color, spacing, radius, typography, and shadows.
- No hardcoded hex values in feature components after token system lands.
- Adapter names and statuses must be data-driven from runtime, never hardcoded to mockup strings.

Recommended initial token values:

- `color.bg.950 = #060B0A`
- `color.bg.900 = #0A1412`
- `color.surface.800 = #0F1E1A`
- `color.border.700 = #1C3B33`
- `color.green.500 = #22C55E`
- `color.green.400 = #4ADE80`
- `color.marine.500 = #2F6F9F`
- `color.marine.400 = #4C8DBF`
- `color.warning.500 = #EAB308`
- `color.danger.500 = #EF4444`

## [P3-DS-01] Visual Design System v0 (Dark/Green + Marine Blue)

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: M3.1
- Milestone alignment: M3.1 foundation

### Issue Body (Markdown)

```md
## Problem
Without a shared visual token system, AI-generated frontend code will drift in colors, spacing, typography, and states across screens.

## Scope
Create a design-system foundation for the Tauri React app: color/spacing/type tokens, global theme variables, and core primitives (`Button`, `Badge`, `Card`, `Panel`, `Tabs`, `Modal`, `Table`).

## Acceptance Criteria
- [ ] A token source defines dark/green base colors and marine-blue highlight states.
- [ ] Core primitives consume tokens only and expose required states (default, hover, focus, disabled, loading).
- [ ] Focus-visible and selected states consistently use marine-blue highlight treatment.
- [ ] Lint or style checks fail when raw hex colors are introduced in feature components.

## Out of Scope
Screen-level feature implementation.

## Dependencies
- M3.1

## Notes
- Theme intent is mandatory: dark + green with marine-blue highlights.
- This ticket should land before major screen implementation to prevent churn.
```

## [P3-IPC-01] GUI Race IPC + Event Backpressure Layer

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: M3.1, M2.10
- Milestone alignment: M3.2

### Issue Body (Markdown)

```md
## Problem
The GUI screens require a stable command/event contract for launching races, streaming output, and loading final results. Without a state layer and backpressure controls, UI rendering can freeze under high event volume.

## Scope
Implement typed IPC commands and a frontend event store for: start race, subscribe stream, fetch results, and map errors to user-readable messages. Add bounded buffering/backpressure behavior.

## Acceptance Criteria
- [ ] GUI can launch a race, subscribe to events, and fetch final results via IPC.
- [ ] Core errors are mapped to human-readable frontend messages.
- [ ] High-throughput event streams do not freeze the UI thread.
- [ ] Event store supports selection state needed by running agent cards and diff/results views.

## Out of Scope
Workflow (Phase 4) IPC commands.

## Dependencies
- M3.1
- M2.10
```

## [P3-UI-01] System Preflight Dashboard (Mockup Image #1)

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: P3-DS-01, P3-IPC-01
- Milestone alignment: M3.1, M3.2, M3.3 readiness

### Issue Body (Markdown)

```md
## Problem
Users need a fast readiness surface before running races. CLI doctor output exists, but there is no visual dashboard for subsystem health, diagnostic checks, environment readiness, and quick recovery actions.

## Scope
Build the preflight dashboard matching Image #1: system readiness hero card, diagnostics checklist, environment panel, and actions (`View Logs`, `Re-run Diagnostics`).

## Acceptance Criteria
- [ ] Preflight screen renders readiness state, passed/failed count, and health indicator.
- [ ] Diagnostic rows show status badge and evidence text.
- [ ] Environment panel shows active adapters and warning block when resource constraints are detected.
- [ ] Re-run diagnostics action refreshes screen state from backend data.

## Out of Scope
Historical diagnostics analytics.

## Dependencies
- P3-DS-01
- P3-IPC-01

## Mockup References
- Image #1
```

## [P3-UI-02] Experimental Adapter Opt-In Modal (Mockup Image #2)

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: S
- Dependencies: P3-DS-01, P3-IPC-01
- Milestone alignment: M3.6

### Issue Body (Markdown)

```md
## Problem
Experimental adapters must require explicit acknowledgment of risk. Without a blocking confirmation flow, users can accidentally run unstable adapters.

## Scope
Implement the warning modal from Image #2 for experimental adapter selection with a required risk acknowledgment checkbox and disabled confirm action until acknowledged.

## Acceptance Criteria
- [ ] Experimental adapters trigger a blocking warning modal before selection is accepted.
- [ ] Confirm action remains disabled until risk acknowledgment is checked.
- [ ] Modal includes a clear warning treatment and resource-impact messaging.
- [ ] Tier-1 adapters remain the default selection path.

## Out of Scope
Adapter-specific advanced configuration.

## Dependencies
- P3-DS-01
- P3-IPC-01

## Mockup References
- Image #2
```

## [P3-UI-03] Live Agent Output + Running Agents Rail

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: P3-DS-01, P3-IPC-01
- Milestone alignment: M3.3

### Issue Body (Markdown)

```md
## Problem
The GUI must provide real-time visibility into each agent execution. Without an agent rail and live output panel, users cannot monitor progress or diagnose failures quickly.

## Scope
Implement running-agent list with lifecycle badges and selected-agent live output panel. Ensure rendering remains responsive under sustained stream volume.

## Acceptance Criteria
- [ ] One list item per running agent with status (running, completed, failed, timed out).
- [ ] Selecting an agent switches the live output panel context.
- [ ] Stream rendering remains responsive under high-volume output.
- [ ] Failure and timeout states are visually distinct and persist in history for completed run review.

## Out of Scope
Full-text log search and export.

## Dependencies
- P3-DS-01
- P3-IPC-01
```

## [P3-UI-04] Results Scoreboard + Winner Selection (Mockup Image #4)

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: P3-DS-01, P3-IPC-01
- Milestone alignment: M3.4

### Issue Body (Markdown)

```md
## Problem
Scoring artifacts exist, but users need a clear ranked decision surface to choose a winner safely and understand why candidates are blocked.

## Scope
Implement results page matching Image #4: ranked candidate cards, mergeability indicators, explicit "Select as Winner" action, and per-dimension score breakdown.

## Acceptance Criteria
- [ ] Ranked cards show composite score and mergeability/gate status.
- [ ] Non-mergeable candidates are visually blocked from winner action.
- [ ] Winner selection is explicit and not auto-applied by UI.
- [ ] Per-dimension score breakdown table matches scoring artifact fields.
- [ ] Run-level metadata (duration, cost where available) is displayed.

## Out of Scope
Cross-run trend analytics.

## Dependencies
- P3-DS-01
- P3-IPC-01

## Mockup References
- Image #4
```

## [P3-UI-05] Candidate Diff Review + Merge Action Rail (Mockup Image #3)

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-ui, type-feature
- Estimate: M
- Dependencies: P3-DS-01, P3-IPC-01, P3-UI-04
- Milestone alignment: M3.5 and M3.4 merge path

### Issue Body (Markdown)

```md
## Problem
Users need to inspect candidate code changes before merge decisions. Without a diff workspace and merge action rail, winner selection cannot be confidently validated.

## Scope
Build the diff review surface from Image #3: candidate tabs, side-by-side diff viewer, modified-file list, and merge action rail with status signals.

## Acceptance Criteria
- [ ] User can switch candidate tabs and update diff content accordingly.
- [ ] Side-by-side diff remains usable on large patches (virtualization or equivalent safeguards).
- [ ] Fallback state is shown when diff is unavailable.
- [ ] Merge action rail reflects mergeability status and blocks invalid actions.
- [ ] Accept/Reject actions are explicit and require confirmation path aligned with CLI semantics.

## Out of Scope
Inline code review comments.

## Dependencies
- P3-DS-01
- P3-IPC-01
- P3-UI-04

## Mockup References
- Image #3
```

## [P3-QA-01] GUI Smoke Test Pack for Mockup Flows

- Phase: Phase 3 Tickets (GUI Alpha)
- Labels: hydra, phase-3, area-test, type-test
- Estimate: M
- Dependencies: P3-UI-01, P3-UI-02, P3-UI-03, P3-UI-04, P3-UI-05
- Milestone alignment: M3.7

### Issue Body (Markdown)

```md
## Problem
Mockup-driven GUI implementation is vulnerable to regressions in interaction flow, selection state, and merge gating unless core paths are smoke tested.

## Scope
Add smoke tests for startup, preflight rendering, race launch path, live output selection, results winner selection, experimental modal gating, and diff candidate switching.

## Acceptance Criteria
- [ ] Startup test passes on Linux and Windows CI.
- [ ] Preflight diagnostics screen loads and refresh action works.
- [ ] Experimental adapter flow enforces risk acknowledgment before confirm.
- [ ] Race launch and completion path validate live output and results transitions.
- [ ] Winner selection and merge action path validate dry-run gating behavior.
- [ ] Diff candidate switching path is covered.

## Out of Scope
Pixel-perfect visual regression suite.

## Dependencies
- P3-UI-01
- P3-UI-02
- P3-UI-03
- P3-UI-04
- P3-UI-05
```
