# Phase 4 Interactive Session Implementation Guide

Last updated: 2026-02-23

## Why this doc exists

`planning/issues/phase-4.md` defines acceptance criteria, but implementation agents still need concrete guidance for:

1. PTY architecture and process lifecycle boundaries.
2. Exact IPC contract for interactive sessions.
3. Artifact format and runtime invariants.
4. Test matrix and closure evidence.

This guide is the implementation-level contract for `M4.1` through `M4.6`.

## Non-negotiable invariants

1. Race mode remains deterministic and unchanged.
2. Interactive mode is a separate execution path.
3. No interactive session result is auto-merged or auto-scored as race output.
4. All session output persisted to artifacts must pass secret redaction.
5. Session cleanup must leave no orphan processes/worktrees.

## Architecture boundary

- Core PTY and session lifecycle lives in `hydra-core` + `hydra-app` backend.
- GUI only consumes typed IPC and never controls OS process primitives directly.
- Existing race event channel (`poll_race_events`) remains intact.
- Interactive mode uses a parallel event channel with its own cursor.

## Recommended implementation touchpoints

Backend core/runtime:
- `crates/hydra-core/src/supervisor/mod.rs`
- `crates/hydra-core/src/supervisor/` (add PTY-specific module)
- `crates/hydra-app/src/state.rs`
- `crates/hydra-app/src/ipc_types.rs`
- `crates/hydra-app/src/commands.rs`
- `crates/hydra-app/src/main.rs`

Frontend:
- `crates/hydra-app/frontend/src/App.tsx`
- `crates/hydra-app/frontend/src/types.ts`
- `crates/hydra-app/frontend/src/ipc.ts`
- `crates/hydra-app/frontend/src/components/` (new interactive components)
- `crates/hydra-app/frontend/src/__tests__/smoke.test.tsx`

Artifacts:
- `crates/hydra-core/src/artifact/` (new session layout/helpers)

## IPC contract (target)

Use snake_case command names in Rust, camelCase payload fields in TS.

Commands:
1. `start_interactive_session(request) -> InteractiveSessionStarted`
2. `poll_interactive_events(session_id, cursor) -> InteractiveEventBatch`
3. `write_interactive_input(session_id, input) -> InteractiveWriteAck`
4. `resize_interactive_terminal(session_id, cols, rows) -> InteractiveResizeAck`
5. `stop_interactive_session(session_id) -> InteractiveStopResult`
6. `list_interactive_sessions() -> Vec<InteractiveSessionSummary>`

Suggested request/response types:

`InteractiveSessionRequest`
- `agentKey: string`
- `taskPrompt: string`
- `allowExperimental: boolean`
- `unsafeMode: boolean`
- `cwd: string | null`
- `cols: number | null`
- `rows: number | null`

`InteractiveSessionStarted`
- `sessionId: string`
- `agentKey: string`
- `status: string`
- `startedAt: string`

`InteractiveStreamEvent`
- `sessionId: string`
- `agentKey: string`
- `eventType: string`
- `data: unknown`
- `timestamp: string`

`InteractiveEventBatch`
- `sessionId: string`
- `events: InteractiveStreamEvent[]`
- `nextCursor: number`
- `done: boolean`
- `status: string`
- `error: string | null`

## Artifact contract (target)

Persist interactive sessions outside race artifacts:
- `.hydra/sessions/<session_id>/session.json`
- `.hydra/sessions/<session_id>/events.jsonl`
- `.hydra/sessions/<session_id>/transcript.ansi.log`
- `.hydra/sessions/<session_id>/summary.json`

`session.json` minimum fields:
- `schema_version`
- `session_id`
- `agent_key`
- `started_at`
- `ended_at`
- `status`
- `cwd`
- `unsafe_mode`
- `experimental`

## Milestone execution details

### M4.1 PTY Supervisor Path for Interactive Sessions

Implementation details:
1. Add PTY-backed spawn mode in supervisor that supports write and resize.
2. Preserve existing non-PTY supervisor path for race mode.
3. Normalize PTY output into line events and terminal chunks.
4. Ensure cancellation kills process groups, not only parent PID.

Negative tests:
- PTY spawn failure returns structured error.
- Write after session stop returns validation error.
- Resize on invalid session returns validation error.

### M4.2 Interactive Session Runtime and IPC Surface

Implementation details:
1. Add interactive session registry in app state.
2. Track session lifecycle (`running`, `completed`, `failed`, `stopped`).
3. Implement cursor-based polling with bounded buffers.
4. Ensure shutdown hook stops active sessions and flushes artifacts.

Negative tests:
- Unknown `session_id` on poll/write/resize/stop.
- Duplicate stop calls are idempotent.
- Poll after done returns stable `done=true` behavior.

### M4.3 Interactive UI Shell and Terminal Panel

Implementation details:
1. Add `Interactive` top-level tab in `App.tsx`.
2. Add session rail (select/create/stop).
3. Add terminal panel capable of ANSI rendering and auto-scroll behavior.
4. Keep design token compliance; no hardcoded colors.

Recommended components:
- `InteractiveWorkspace.tsx`
- `InteractiveSessionRail.tsx`
- `InteractiveTerminalPanel.tsx`

Negative tests:
- Empty state when no session exists.
- Fallback message for disconnected IPC.
- Large stream remains responsive.

### M4.4 Mid-Flight Intervention Controls

Implementation details:
1. Add input composer for active session.
2. Add interrupt/stop controls with explicit confirmation.
3. Add clear error feedback for rejected writes.
4. Log intervention actions as session events.

Negative tests:
- Input rejected when session is not running.
- Rapid input does not freeze UI.
- Interrupt action transitions lifecycle correctly.

### M4.5 Interactive Safety and Capability Gating

Implementation details:
1. Check adapter capability before start.
2. Enforce experimental adapter confirmation.
3. Reuse working-tree cleanliness checks before launch when policy requires.
4. Block unsupported/unsafe launches with actionable reasons.

Negative tests:
- Experimental adapter denied without confirmation.
- Unsupported adapter denied with reason.
- Dirty working tree policy block message appears.

### M4.6 Interactive Transcript Artifacts and E2E Tests

Implementation details:
1. Persist session transcript and summary artifacts.
2. Add Rust integration tests for start/input/stop flows.
3. Add frontend smoke tests for interactive tab and intervention loop.
4. Assert race mode tests still pass unchanged.

Required test coverage:
- Golden path: start -> stream -> input -> stop.
- Failure path: spawn error and cleanup.
- Cleanup path: app shutdown with active sessions.

## Test matrix (must pass)

Backend:
1. `cargo test --workspace --locked --offline`
2. `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`

GUI:
1. `cargo test --manifest-path crates/hydra-app/Cargo.toml --locked --offline`
2. `npm run lint` in `crates/hydra-app/frontend`
3. `npm run test:smoke` in `crates/hydra-app/frontend`

## Done evidence template

Attach to each Phase 4 ticket closure:
1. Commit(s) and touched files.
2. IPC payload example (request + response JSON).
3. One artifact sample from `.hydra/sessions/<session_id>/`.
4. Test command outputs (pass/fail + duration).
5. Screenshot or short recording for UI milestones (`M4.3`, `M4.4`).

## Explicit non-goals for Phase 4

1. Do not merge interactive sessions into race scoring pipeline.
2. Do not introduce automatic agent-to-agent freeform messaging yet.
3. Do not implement Windows-specific ConPTY hardening here (Phase 6).
