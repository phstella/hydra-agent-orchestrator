# Phase 4 Tickets (Interactive Session Mode) Issue Bodies

Last updated: 2026-02-23

Generated from `planning/implementation-checklist.md`.

Global label prefix: `hydra`

## [M4.1] PTY Supervisor Path for Interactive Sessions

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-core, type-feature
- Estimate: M
- Dependencies: M1.4

### Issue Body (Markdown)

```md
## Problem
Current process supervision is non-interactive (`stdin` is closed), so users cannot communicate with a running agent.

## Scope
Add a PTY-backed supervisor path with interactive stdin write support, terminal resize, cancellation, and normalized output streaming while preserving current non-PTY race path.

## Acceptance Criteria
- [ ] PTY session can spawn supported adapters and stream output.
- [ ] Runtime can write input to a running agent session.
- [ ] Terminal resize events are propagated to the process.
- [ ] Cancellation terminates child process group without orphans.

## Out of Scope
Windows parity hardening (Phase 6); replacing deterministic race-mode supervision.

## Dependencies
- M1.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M4.2] Interactive Session Runtime and IPC Surface

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-core, type-feature
- Estimate: M
- Dependencies: M4.1, M3.2

### Issue Body (Markdown)

```md
## Problem
The app has race-oriented IPC only; there is no session-oriented runtime for bidirectional interactive control.

## Scope
Implement interactive session manager in `hydra-app` state with IPC commands for `start`, `write`, `resize`, `poll`, and `stop`. Include per-session lifecycle state and cleanup hooks.

## Acceptance Criteria
- [ ] Multiple interactive sessions can coexist with isolated state.
- [ ] IPC commands validate session ownership and lifecycle transitions.
- [ ] Session cleanup runs on stop, failure, and app shutdown.

## Out of Scope
UI layout; scoring/merge integration.

## Dependencies
- M4.1, M3.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M4.3] Interactive UI Shell and Terminal Panel

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-ui, type-feature
- Estimate: M
- Dependencies: M4.2, M3.1

### Issue Body (Markdown)

```md
## Problem
There is no dedicated interactive workspace in the GUI for terminal-first collaboration with agents.

## Scope
Add a new Interactive mode/tab with session rail, selected-session terminal panel, and launch controls aligned with existing design system tokens.

## Acceptance Criteria
- [ ] Interactive tab is available and wired to session IPC.
- [ ] Terminal panel renders ANSI/color output in a readable format.
- [ ] Session rail reflects running/paused/completed/failed lifecycle.

## Out of Scope
Race scoreboard and merge review interactions.

## Dependencies
- M4.2, M3.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M4.4] Mid-Flight Intervention Controls

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-ui, type-feature
- Estimate: M
- Dependencies: M4.2, M4.3

### Issue Body (Markdown)

```md
## Problem
Even with a terminal panel, users need explicit UX controls to intervene safely during execution.

## Scope
Add intervention controls (send instruction/input, interrupt, resume where supported) and clear status feedback when commands are accepted/rejected.

## Acceptance Criteria
- [ ] User can send input while agent is running.
- [ ] Interrupt/cancel actions update lifecycle state and UI feedback.
- [ ] Intervention actions are logged as structured session events.

## Out of Scope
Branching conversation history; collaborative multi-user editing.

## Dependencies
- M4.2, M4.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M4.5] Interactive Safety and Capability Gating

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-core, type-feature
- Estimate: S
- Dependencies: M2.1, M4.2

### Issue Body (Markdown)

```md
## Problem
Interactive mode can accidentally bypass adapter safety assumptions unless capability gates and guardrails are explicit.

## Scope
Gate interactive mode by adapter capability and tier policy, enforce preflight checks (e.g., working tree readiness), and require explicit confirmation for experimental/unsafe modes.

## Acceptance Criteria
- [ ] Unsupported adapters are blocked with actionable reason.
- [ ] Experimental adapters require explicit risk confirmation.
- [ ] Safety checks run before session start and block unsafe launch by default.

## Out of Scope
Automatic policy override; remote execution sandboxing.

## Dependencies
- M2.1, M4.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M4.6] Interactive Transcript Artifacts and E2E Tests

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-test, type-test
- Estimate: M
- Dependencies: M4.3, M4.4, M4.5

### Issue Body (Markdown)

```md
## Problem
Interactive sessions are hard to debug and regressions are likely without persisted transcripts and automated test coverage.

## Scope
Persist interactive session transcripts/artifacts and add integration/smoke coverage for start, mid-flight input, interrupt, and cleanup paths.

## Acceptance Criteria
- [ ] Session transcripts are persisted under run/session artifacts.
- [ ] End-to-end tests validate interactive start/input/stop flows.
- [ ] Existing race-mode tests remain green and behavior remains unchanged.

## Out of Scope
Long-term analytics dashboard; transcript semantic search.

## Dependencies
- M4.3, M4.4, M4.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## Coverage Check

- Total issues generated: 6
- Expected range: `M4.1` through `M4.6`
