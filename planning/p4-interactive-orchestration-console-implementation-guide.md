# Phase 4.8 Interactive Orchestration Console Implementation Guide

Last updated: 2026-02-25

## Why this doc exists

`M4.1` through `M4.6` delivered interactive capability and `M4.7` converged race cockpit workflow, but interactive orchestration is still not fully aligned with the target orchestration-console operator model.

This guide defines the implementation contract for `M4.8`:
1. mockup-aligned interactive orchestration flow
2. multi-session concurrency including duplicate adapter instances
3. strict separation from race/scoring behavior

Execution mode for this milestone is local-first:
1. primary tracker: `planning/m4.8-interactive-orchestration-pack.md`
2. issue documents are optional synchronization outputs only

## Deep analysis: current state vs target orchestration console

### Current state (implemented baseline)

1. Interactive mode supports multiple sessions, per-session polling, input, resize, and stop.
2. Session state is available through `list_interactive_sessions` and per-session event polling.
3. UI includes interactive session rail, terminal panel, and create-session form.
4. Backend maintains session artifacts and cleanup invariants.
5. Race and interactive execution paths are already separate at runtime.

### Gap categories

| Target orchestration block | Current status | Gap severity | Notes |
|---|---|---|---|
| Single orchestration-first interactive surface | Partial | High | Current flow is functional but still closer to tab/session list behavior than orchestration console behavior. |
| Explicit lane identity for duplicate adapter sessions | Partial | High | Session IDs exist, but UX needs stronger lane identity and visual disambiguation for same adapter keys. |
| Create-panel + focused output + running-lanes rail parity | Partial | High | Existing components map to these primitives, but IA and presentation need convergence. |
| Per-lane intervention isolation under duplicate adapters | Partial | High | Runtime supports session-scoped commands; UX/test contracts must explicitly validate duplicate-adapter isolation. |
| Race-separation guarantee as explicit regression gate | Partial | Medium | Separation exists, but M4.8 requires explicit no-regression validation criteria. |
| Closure evidence pack specific to orchestration flow | Missing | Medium | Existing evidence patterns need M4.8-specific scenarios and artifacts. |

## Non-negotiable invariants

1. Interactive and race remain separate execution paths.
2. Race scoring, review, and merge semantics remain unchanged.
3. Interactive lane identity is session-based (`session_id` is the control authority).
4. Duplicate adapter sessions are fully supported.
5. Secret redaction and artifact persistence invariants from M4.6 remain enforced.
6. Desktop-first delivery target remains `>=1280px` primary and `>=1024px` minimum.
7. Design-token policy remains enforced (no hardcoded hex in new UI code).

## Target orchestration console contract (M4.8)

The interactive orchestration surface should expose one operator loop:
1. configure and spawn new lane
2. monitor running lanes
3. select lane and inspect focused terminal output
4. intervene (write/stop) on selected lane
5. continue monitoring remaining lanes
6. inspect per-session artifacts

### Layout contract

1. Left block: session creation and interactive safety controls.
2. Center block: focused terminal/output for selected lane with lane header context.
3. Right block: running-lanes rail with per-session lifecycle and quick actions.

### Interaction contract

1. Lane selection changes focused stream within one polling cycle.
2. Input and stop actions are enabled only for selected running lane.
3. One lane failure must not terminate or freeze sibling running lanes.
4. Duplicate adapter lanes must remain visually distinguishable.

## State model convergence requirements

`M4.8` requires explicit state ownership boundaries:

1. **Lane registry state**
   - authoritative session summaries keyed by `session_id`
   - selected lane id and lane ordering metadata

2. **Lane stream state**
   - per-lane event buffers
   - per-lane polling cursors and retry/error state

3. **Lane action state**
   - create lane request state
   - write/stop pending states scoped to selected lane
   - lane-local validation and feedback

4. **Artifact/replay state**
   - session artifact availability metadata
   - lane terminal status and completion reason

Recommended frontend strategy:
1. keep session-keyed maps as canonical state (`session_id` map keys only)
2. avoid adapter-key keyed control paths
3. add lane label helpers for duplicate adapter disambiguation

## Backend and IPC expectations

No mandatory breaking IPC changes are required if existing session-scoped APIs remain authoritative:
1. `start_interactive_session`
2. `poll_interactive_events`
3. `write_interactive_input`
4. `resize_interactive_terminal`
5. `stop_interactive_session`
6. `list_interactive_sessions`

Allowed additive work:
1. optional lane metadata fields for richer UI identity
2. optional per-session runtime metadata for rail cards

Disallowed for `M4.8`:
1. race IPC behavior changes
2. score/report merge semantics changes
3. transport migration scope expansion (websocket-only refactor)

## Artifact and cleanup invariants for M4.8

1. Session artifacts remain under `.hydra/sessions/<session_id>/`.
2. Duplicate adapter sessions produce distinct artifact directories.
3. Session finalization status must remain lane-correct under:
   - explicit stop
   - natural completion
   - failure
   - app shutdown
4. Oversized interactive event payload protection remains active.

## Recommended implementation touchpoints

Frontend:
1. `crates/hydra-app/frontend/src/App.tsx`
2. `crates/hydra-app/frontend/src/components/InteractiveWorkspace.tsx`
3. `crates/hydra-app/frontend/src/components/InteractiveSessionRail.tsx`
4. `crates/hydra-app/frontend/src/components/InteractiveTerminalPanel.tsx`
5. `crates/hydra-app/frontend/src/components/InputComposer.tsx`
6. `crates/hydra-app/frontend/src/ipc.ts`
7. `crates/hydra-app/frontend/src/types.ts`
8. `crates/hydra-app/frontend/src/__tests__/smoke.test.tsx`

Backend/app runtime (only if required for additive lane metadata):
1. `crates/hydra-app/src/state.rs`
2. `crates/hydra-app/src/commands.rs`
3. `crates/hydra-app/src/ipc_types.rs`
4. `crates/hydra-app/src/main.rs`

Core/artifacts (only if additional verification is needed):
1. `crates/hydra-core/src/artifact/`

## Milestone work breakdown (suggested slices)

### W4.8.a Orchestration shell convergence
1. reshape interactive view into orchestration-console IA
2. keep create panel, focused terminal, and running-lanes rail simultaneously visible on desktop
3. retain fallback route behavior for legacy paths

### W4.8.b Session-lane identity hardening
1. audit all client interactive maps/cursors/action handlers for session-key usage
2. add lane disambiguation labels for duplicate adapter keys
3. ensure selection state remains stable under rapid lane creation

### W4.8.c Multi-instance launch support validation
1. validate repeated same-adapter launches in one console lifecycle
2. refine create flow status/errors for parallel lane starts
3. preserve all safety and gating checks

### W4.8.d Running-lanes rail enrichment
1. improve lane card identity and lifecycle density
2. support quick lane switching and lane-local stop affordance
3. keep rail scalable for multiple concurrent lanes

### W4.8.e Focused terminal and stream isolation
1. ensure per-lane stream buffering and polling remain isolated
2. verify focus switching behavior and cursor continuity
3. preserve bounded buffer and auto-scroll behavior

### W4.8.f Per-lane intervention isolation
1. enforce selected-lane authority for input/stop/resize
2. validate isolation with duplicate adapter lanes
3. surface lane-local errors without global session collapse

### W4.8.g Artifact and cleanup validation
1. verify per-lane artifact segregation
2. verify stop/shutdown finalization under mixed lane outcomes
3. verify no orphan runtime session entries remain

### W4.8.h QA and regression gate
1. add duplicate-adapter + lane-isolation smoke coverage
2. keep race and existing interactive smoke coverage green
3. capture closure evidence pack

## Acceptance test matrix (must pass)

Backend:
1. `cargo test --workspace --locked --offline`
2. `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`

App/backend focus:
1. `cargo test --manifest-path crates/hydra-app/Cargo.toml --locked --offline`
2. `cargo clippy --manifest-path crates/hydra-app/Cargo.toml --all-targets --locked --offline -- -D warnings`

Frontend:
1. `npm run lint` in `crates/hydra-app/frontend`
2. `npm run test:smoke` in `crates/hydra-app/frontend`

Required new scenario coverage:
1. two lanes with same adapter can be spawned and independently selected
2. selected lane determines terminal focus
3. write action is lane-isolated under duplicate adapters
4. stop action is lane-isolated under duplicate adapters
5. lane-local poll error does not collapse healthy sibling lane
6. existing race cockpit and review flow remain regression-free

## Done evidence template (M4.8)

Attach on milestone closure:
1. commit list and touched files
2. before/after screenshots for target desktop viewports
3. short recording of duplicate-adapter multi-lane orchestration flow
4. artifact samples from two same-adapter sessions
5. validation command outputs
6. explicit race regression statement

## Explicit non-goals

1. Do not implement Phase 5 collaboration DAG/presets here.
2. Do not add cross-agent message passing semantics.
3. Do not add auto-merge or score-driven autonomous merge decisions.
4. Do not merge interactive artifact streams into race artifact timelines.

## Open product/design inputs still useful

1. Preferred lane naming policy:
   session-id suffix only vs user-editable alias.
2. Preferred default focused lane policy:
   newest lane vs first running lane vs persisted prior selection.
3. Maximum lane density target before rail virtualization is required.
