# Phase 4.7 Unified Race Cockpit Convergence Guide

Last updated: 2026-02-25

## Why this doc exists

`M4.7` is a pre-Phase-5 UX convergence gate. Phase 4 delivered capability completeness, but operator workflow is still split across multiple tabs. This guide defines the concrete contract to converge the GUI into a single cockpit before Phase 5 collaboration workflows are built.

Execution mode for this milestone is local-first:
- primary tracker: `planning/m4.7-local-execution-pack.md`
- GitHub issue files are optional synchronization artifacts only

Without this gate, Phase 5 would likely require structural rework of:
- UI information architecture
- run/session state ownership
- streaming and intervention surfaces
- smoke test topology

## Deep analysis: current state vs target cockpit

### Current state (implemented)

1. Race launch and monitoring are in `Race` tab.
2. Scoreboard and winner selection are in `Results` tab.
3. Diff review and merge action rail are in `Review` tab.
4. Mid-flight intervention is in separate `Interactive` tab/session model.
5. Workspace selection is in `Settings` and consumed by race/review/interactive paths.

### Gap categories

| Target cockpit block | Current status | Gap severity | Notes |
|---|---|---|---|
| Single-page operator workflow | Partial | High | Current flow requires tab-switching across race/results/review/interactive surfaces. |
| Left nav rail + shell layout | Missing | Medium | Current layout is tab bar with page-level content. |
| Center live terminal focus tied to leaderboard selection | Partial | High | Focus switching exists for race rail, but not in integrated cockpit shell. |
| Right leaderboard with rich live card states | Partial | Medium | Agent rail exists, but card richness and operational summary are limited. |
| Inline intervention during race context | Partial | High | Intervention exists, but only in separate interactive workspace. |
| Completion summary + direct review action in same view | Partial | Medium | Exists via tab jump, not inline cockpit flow. |
| Operational status strip (connected, workspace, doctor status) | Partial | Medium | Fragments exist; no unified status strip. |

## Non-negotiable invariants

1. Race mode determinism remains unchanged.
2. Existing IPC commands remain backward compatible unless a new command is additive.
3. No auto-merge behavior is introduced.
4. Interactive session artifacts and race run artifacts remain logically separate.
5. Design system token policy remains enforced (no hardcoded hex colors).
6. Desktop-first delivery for this milestone (`>=1280px` primary, `>=1024px` minimum).

## Target cockpit contract (M4.7)

The default app entry surface should become a unified dashboard with:

1. **Left rail**:
- compact mode icons (Preflight, Race/Cockpit, Results/Review shortcut, Settings)
- persistent utility indicators (git/doctor status)

2. **Top status strip**:
- workspace identifier
- backend connection status
- run controls (`Run`, `Run All`, `Stop` where valid)

3. **Center workspace**:
- race configuration card (lanes/adapters/prompt/context attach)
- live terminal/output panel for selected agent
- inline intervention composer (send input, stop/interrupt)

4. **Right leaderboard rail**:
- per-agent cards with lifecycle, elapsed time, score snapshot, mergeability hint
- selected agent highlight controlling center focus
- failure state message inline per card

5. **Completion block**:
- winner + mergeability summary when run completes
- direct action to diff review/merge panel

## State model convergence requirements

`M4.7` requires explicit separation of:

1. **Race orchestration state**
- run lifecycle, per-agent lifecycle, scoreboard snapshots

2. **Terminal focus state**
- selected agent card
- output stream cursor/tail buffers

3. **Intervention state**
- input buffer
- send/stop command status
- validation feedback

4. **Navigation state**
- cockpit as default route
- deep link to review for merge actions

Recommended frontend state strategy:
- create a cockpit container hook (example: `useCockpitState`) that composes existing race and interactive hooks rather than duplicating IPC polling logic in component tree.

## Backend/IPC expectations

No mandatory breaking backend changes are required for `M4.7`. Existing contracts should be reused:

- `start_race`, `poll_race_events`, `get_race_result`
- interactive session commands from M4.2-M4.6
- review/merge commands from Phase 3

Additive IPC is allowed only if needed for leaderboard richness (for example, elapsed runtime snapshots) and must not break existing calls.

## Recommended implementation touchpoints

Frontend:
- `crates/hydra-app/frontend/src/App.tsx`
- `crates/hydra-app/frontend/src/components/AgentRail.tsx`
- `crates/hydra-app/frontend/src/components/LiveOutputPanel.tsx`
- `crates/hydra-app/frontend/src/components/InteractiveWorkspace.tsx`
- `crates/hydra-app/frontend/src/components/InputComposer.tsx`
- `crates/hydra-app/frontend/src/components/ResultsScoreboard.tsx`
- `crates/hydra-app/frontend/src/__tests__/smoke.test.tsx`

Backend (only if additive data needed):
- `crates/hydra-app/src/ipc_types.rs`
- `crates/hydra-app/src/commands.rs`
- `crates/hydra-app/src/state.rs`

## Milestone work breakdown (suggested slices)

### W4.7.a Cockpit shell and layout skeleton
1. Implement left rail + top strip + center/right pane grid.
2. Keep responsive behavior explicit (desktop and smaller widths).
3. Move tab-based navigation to secondary/utility behavior.

### W4.7.b Race configuration convergence
1. Migrate race launch controls into cockpit center card.
2. Keep adapter selection logic and experimental warning behavior.
3. Show effective workspace value from Settings with quick jump/edit affordance.

### W4.7.c Leaderboard rail upgrade
1. Replace/extend current agent rail with richer state cards.
2. Surface lifecycle, runtime, score snapshot, mergeability hint.
3. Add explicit selected-card focus model.

### W4.7.d Terminal focus + intervention unification
1. Bind center terminal to selected leaderboard card.
2. Integrate intervention controls inline in cockpit center.
3. Preserve bounded buffering and stable auto-scroll semantics.

### W4.7.e Completion and review transition
1. Show completion summary panel (winner, mergeability, basic stats).
2. Add direct CTA into detailed diff review panel.
3. Ensure no automatic merge side effects.

### W4.7.f QA hardening and regression safety
1. Expand smoke tests for cockpit path.
2. Keep all existing smoke tests green.
3. Verify Rust tests/clippy remain clean where touched.

## Acceptance test matrix (must pass)

Backend:
1. `cargo test --workspace --locked --offline`
2. `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`

Frontend:
1. `npm run lint` in `crates/hydra-app/frontend`
2. `npm run test:smoke` in `crates/hydra-app/frontend`

Required new smoke scenarios:
1. Cockpit shell renders with left/top/center/right regions.
2. Race start from cockpit updates leaderboard lifecycle.
3. Agent selection in leaderboard changes terminal focus.
4. Inline intervention send success/failure path.
5. Stop/interrupt path transitions UI state correctly.
6. Completion summary appears and can navigate to review.

## Done evidence template (M4.7)

Attach on milestone closure:
1. Commit list and touched files.
2. Before/after cockpit screenshots.
3. Smoke test additions with named scenario list.
4. Command outputs for required test matrix.
5. Brief regression statement for race/review/interactive behavior.

## Explicit non-goals

1. Do not implement Phase 5 workflow DAG/presets in this milestone.
2. Do not add multi-user or remote collaboration semantics.
3. Do not migrate transport to websocket-only architecture.
4. Do not add visual workflow graph editor.

## Open inputs still useful from product/design

If available, provide:
1. Breakpoint-specific behavior priorities (desktop-first vs strict tablet support).
2. Preferred default terminal focus policy (top score vs first running vs manual persist).
3. Exact leaderboard card fields required at launch vs deferred.
