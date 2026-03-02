# Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity) Issue Bodies

Last updated: 2026-02-25

Generated from `planning/implementation-checklist.md`.
Local-first execution is tracked in `planning/m4.7-local-execution-pack.md`;
this file is optional sync output only.

Global label prefix: `hydra`

Implementation guides:
- `planning/p4-interactive-session-implementation-guide.md`
- `planning/p4-race-cockpit-convergence-implementation-guide.md`

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

## Implementation Reference
- `planning/p4-interactive-session-implementation-guide.md` (`M4.1`)
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

## Implementation Reference
- `planning/p4-interactive-session-implementation-guide.md` (`M4.2`)
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

## Implementation Reference
- `planning/p4-interactive-session-implementation-guide.md` (`M4.3`)
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

## Implementation Reference
- `planning/p4-interactive-session-implementation-guide.md` (`M4.4`)
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

## Implementation Reference
- `planning/p4-interactive-session-implementation-guide.md` (`M4.5`)
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

## Implementation Reference
- `planning/p4-interactive-session-implementation-guide.md` (`M4.6`)
```


## [M4.7] Unified Race Cockpit UX Convergence (Pre-Phase 5 Gate)

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-ui, type-feature
- Estimate: L
- Dependencies: M3.5, M4.4, M4.6

### Issue Body (Markdown)

```md
## Problem
The GUI currently requires context switching between separate tabs (`Race`, `Results`, `Review`, `Interactive`) to complete one end-to-end operator workflow. The target UX is a single cockpit where launch, live monitoring, intervention, and winner decision happen in one place. If we start Phase 5 workflows before converging the cockpit IA, we will likely rework major UI state surfaces twice.

## Scope
Build a unified race cockpit shell aligned with the target dashboard mock:

- persistent left tool rail and top status strip
- center workspace for race configuration and focused live terminal output
- right leaderboard/status rail with per-agent lifecycle and score signals
- inline intervention controls for the selected running agent
- completion summary with direct transition to diff review/merge flow

Reuse existing backend IPC/runtime contracts where possible; focus on UI/state composition and streaming UX quality.

## Acceptance Criteria
- [ ] Cockpit is the default execution surface and renders a stable 3-column layout (left nav rail, center workspace, right leaderboard).
- [ ] Race launch is available in cockpit center and uses configured workspace settings.
- [ ] Right leaderboard updates live for per-agent status, score snapshot, and elapsed runtime.
- [ ] Selecting an agent card switches center terminal focus without losing buffered stream context.
- [ ] Mid-flight input and stop/interrupt controls are available inline for selected running agent.
- [ ] Error states (launch failure, timeout, transport, parse) are visible in leaderboard + terminal header with actionable messaging.
- [ ] Completion summary shows winner + mergeability and offers one-click transition to detailed diff review.
- [ ] Large output streams remain responsive (bounded/tail buffering + stable auto-scroll behavior).
- [ ] New smoke tests cover cockpit render, race start, live leaderboard updates, focus switching, intervention path, and completion summary.
- [ ] Existing race/review/interactive tests remain green with no IPC contract regressions.
- [ ] New frontend code remains design-token compliant (no hardcoded hex colors).

## Out of Scope
Phase 5 workflow DAG/presets, multi-user collaboration, websocket transport migration, visual workflow editor.

## Dependencies
- M3.5, M4.4, M4.6

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
- This milestone is a pre-Phase-5 UX convergence gate.

## Implementation Reference
- `planning/p4-race-cockpit-convergence-implementation-guide.md` (`M4.7`)
- `planning/m4.7-local-execution-pack.md` (`M4.7` local tracking)
- `planning/m4.7-desktop-ui-contract.md` (`M4.7` desktop behavior)
```

## [M4.8] Interactive Orchestration Console (Multi-Instance, Race-Separated)

- Phase: Phase 4 Tickets (Interactive Session Mode)
- Labels: hydra, phase-4, area-ui, area-core, type-feature
- Estimate: L
- Dependencies: M4.2, M4.6, M4.7

### Issue Body (Markdown)

```md
## Problem
Interactive mode supports session control primitives, but the operator experience is still session-tab oriented and does not fully match orchestration-console behavior. We need a dedicated interactive orchestration flow where users can run multiple concurrent sessions, including multiple sessions of the same adapter type, without conflating this path with race mode.

## Scope
Implement an interactive orchestration console aligned with the mockup:

- create-session panel + focused terminal + running-agents rail
- lane identity based on `session_id` (not adapter key)
- support multiple concurrent sessions for the same adapter type
- per-lane lifecycle and intervention controls
- explicit separation from race/scoring semantics

## Acceptance Criteria
- [ ] Interactive console can spawn/manage multiple concurrent sessions from one surface.
- [ ] Duplicate adapter sessions are supported (same adapter key, unique session lanes).
- [ ] Focus, polling, and intervention actions are lane/session scoped.
- [ ] Per-session artifacts persist and remain replayable.
- [ ] Race IPC/scoring/merge behavior remains unchanged.
- [ ] Tests cover duplicate-adapter sessions and lane isolation.

## Out of Scope
Workflow DAG/presets, cross-agent artifact handoff, auto-merge, multi-user collaboration.

## Dependencies
- M4.2, M4.6, M4.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
- Race mode remains a distinct feature and should not be repurposed to implement this milestone.

## Implementation Reference
- `planning/m4.8-interactive-orchestration-pack.md` (`M4.8` local execution tracker)
- `planning/p4-interactive-orchestration-console-implementation-guide.md` (`M4.8` implementation contract)
- `planning/m4.8-interactive-desktop-ui-contract.md` (`M4.8` desktop behavior contract)
- `planning/p4-interactive-session-implementation-guide.md` (`M4.1-M4.6` baseline contracts)
- `planning/p4-race-cockpit-convergence-implementation-guide.md` (`M4.7` shell context)
```

## [P4.9.1] Orchestration IA Rename and Default Landing

- Phase: Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity)
- Labels: hydra, phase-4, area-ui, type-feature
- Estimate: M
- Dependencies: M4.8

### Issue Body (Markdown)

```md
## Problem
The product still exposes legacy `Interactive` naming and does not default to orchestration-first operation, which conflicts with the updated UX direction.

## Scope
Rename user-facing `Interactive` labels/routes/surfaces to `Orchestration` and make Orchestration the always-on default landing view at app startup.

## Acceptance Criteria
- [ ] App launches to `Orchestration` by default on every startup.
- [ ] UI labels and route/test identifiers are migrated from `Interactive` to `Orchestration`.
- [ ] Existing race/results/review/settings flows remain reachable and behaviorally unchanged.
- [ ] Smoke coverage validates default landing + navigation transitions.

## Out of Scope
File explorer implementation; terminal renderer fidelity work.

## Dependencies
- M4.8

## Notes
- Align naming with roadmap section 18 (`P4.9` track).

## Implementation Reference
- `planning/roadmap.md` (Section 18, `P4.9.1`)
```

## [P4.9.2] File Explorer Tab with Real-Time Filesystem Watch

- Phase: Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity)
- Labels: hydra, phase-4, area-ui, area-core, type-feature
- Estimate: L
- Dependencies: P4.9.1, M1.3

### Issue Body (Markdown)

```md
## Problem
Operators lack a reliable live repository tree inside Hydra while agents are changing files, leading to stale context and external-tool dependency.

## Scope
Add a `File Explorer` tab showing the full workspace tree. Auto-update from filesystem watcher events only, with a manual `Refresh` button for explicit resync.

## Acceptance Criteria
- [ ] Explorer renders full repository tree rooted at active workspace (`workspaceCwd`) with no default hiding.
- [ ] Tree updates on watcher events (`create/modify/delete/rename`) without manual page refresh.
- [ ] Manual `Refresh` triggers full tree resync.
- [ ] Watcher lifecycle is correctly handled on workspace switch and app shutdown.
- [ ] UI remains responsive for large repositories and event bursts.

## Out of Scope
Symbol indexing, semantic code graph, built-in diff viewer inside explorer.

## Dependencies
- P4.9.1, M1.3

## Notes
- Integration decision: watcher events are primary source; manual refresh is fallback.

## Implementation Reference
- `planning/roadmap.md` (Section 18, `P4.9.2`)
```

## [P4.9.3] High-Fidelity Terminal Rendering (ANSI Parity)

- Phase: Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity)
- Labels: hydra, phase-4, area-ui, area-core, type-feature
- Estimate: L
- Dependencies: P4.9.1, M4.1, M4.2

### Issue Body (Markdown)

```md
## Problem
Current orchestration terminal rendering normalizes output and does not match native terminal fidelity expected for Claude Code/Codex CLI use.

## Scope
Upgrade terminal rendering to preserve raw PTY stream and render full ANSI behavior (color/style/cursor control/clear semantics/scrollback) with stable streaming performance.

## Acceptance Criteria
- [ ] Orchestration display path preserves raw PTY output (no destructive ANSI stripping).
- [ ] ANSI fixtures verify 24-bit color, style, cursor movement, and clear-line/screen behavior.
- [ ] Stream performance remains stable under sustained output with bounded memory.
- [ ] Copy/select/scrollback behavior remains usable.
- [ ] Existing lane/session focus and isolation behavior remains correct.

## Out of Scope
Terminal multiplexing, session recording UI, remote terminal protocol support.

## Dependencies
- P4.9.1, M4.1, M4.2

## Notes
- UX parity target: native terminal behavior for supported ANSI features.

## Implementation Reference
- `planning/roadmap.md` (Section 18, `P4.9.3`)
```

## [P4.9.4] Direct External CLI Invocation and Deploy Trigger Simplification

- Phase: Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity)
- Labels: hydra, phase-4, area-adapter, area-core, area-ui, type-feature
- Estimate: L
- Dependencies: P4.9.1, M2.1, M4.5

### Issue Body (Markdown)

```md
## Problem
Reimplementing advanced features from vendor CLIs adds unnecessary surface area and slows parity with existing coding tools.

## Scope
Invoke `claude`/`codex` directly from orchestration lanes with no normalization layer. Simplify `Deploy Agent` into a trigger that launches the selected adapter CLI from the dropdown/selector.

## Acceptance Criteria
- [ ] Deploy trigger launches selected external CLI in lane PTY using active workspace context.
- [ ] Tool selection is automatically derived from orchestration adapter selector.
- [ ] Pass-through strategy enables tool-native features without Hydra-side duplication.
- [ ] Existing safety/capability gates remain enforced before launch.
- [ ] Launch failures (missing binary, unsupported flags, auth/session issues) are surfaced with actionable messaging.

## Out of Scope
Cross-tool normalization schema, abstraction shim, remote execution backend.

## Dependencies
- P4.9.1, M2.1, M4.5

## Notes
- Integration decision: direct wrapping only, least implementation overhead.

## Implementation Reference
- `planning/roadmap.md` (Section 18, `P4.9.4`)
```

## [P4.9.5] Terminal-Only Input Model (Native CLI Parity)

- Phase: Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity)
- Labels: hydra, phase-4, area-ui, area-core, type-feature
- Estimate: M
- Dependencies: P4.9.3, P4.9.4

### Issue Body (Markdown)

```md
## Problem
Dual input surfaces (terminal + side composer) diverge from native terminal workflows and create ambiguity in orchestration operation.

## Scope
Adopt terminal-only input for orchestration sessions and remove side `InputComposer` from steady-state UX (debug-only retention allowed if explicitly gated).

## Acceptance Criteria
- [ ] Normal orchestration input is performed directly in terminal.
- [ ] Side `InputComposer` is removed from primary orchestration UX (or hidden behind explicit debug guard).
- [ ] Concurrent-session input isolation remains correct per lane/session.
- [ ] Stop/interrupt semantics remain available and discoverable.
- [ ] Smoke coverage validates terminal-only input and no-regression behavior.

## Out of Scope
Chat-style side panels, rich prompt editor, multimodal input workflows.

## Dependencies
- P4.9.3, P4.9.4

## Notes
- UX target: same interaction model users expect in normal terminals with Claude Code/Codex CLI.

## Implementation Reference
- `planning/roadmap.md` (Section 18, `P4.9.5`)
```

## [P4.9.6] Orchestration Terminal Streaming Performance and Responsiveness

- Phase: Phase 4 Tickets (Interactive Session Mode + Orchestration UX/Parity)
- Labels: hydra, phase-4, area-ui, area-core, type-performance
- Estimate: L
- Dependencies: P4.9.3, P4.9.4, P4.9.5

### Issue Body (Markdown)

```md
## Problem
Even with ANSI parity and terminal-only input, orchestration still feels laggy under sustained CLI/TUI output, especially when frequent stream updates trigger excessive UI work.

## Scope
Implement a low-latency interactive stream pipeline using push transport where available, keep polling as fallback, and reduce render-path churn through batched ingestion and coalesced terminal writes.

## Acceptance Criteria
- [x] Interactive PTY output can be consumed via push stream (event listener) in Tauri runtime.
- [x] Polling remains available as fallback when push transport is unavailable.
- [x] Terminal output ingestion is batched/coalesced to avoid per-event rerenders.
- [x] Lane switching still preserves bounded stream history and isolation semantics.
- [x] Duplicate/replayed lines are prevented under overlap/retry scenarios.
- [x] Existing interactive smoke coverage remains green and includes overlap/no-dup regression checks.
- [ ] Manual release-build stress QA confirms acceptable responsiveness under sustained real CLI/TUI sessions.

## Out of Scope
Remote terminal protocol support, terminal session recording UI, transport encryption changes.

## Dependencies
- P4.9.3, P4.9.4, P4.9.5

## Notes
- Responsiveness target: make orchestration terminal feel close to native Claude Code/Codex CLI usage under sustained output.
- Implementation now includes push-stream transport, batched/coalesced ingestion, imperative selected-lane writes, and callback-paced xterm queue draining.
- Selected-lane path now has low-latency direct append (when terminal is ready) with replay-safe fallback to avoid missing early stream chunks.
- Remaining risk: full-screen/high-churn TUI workloads may still require additional tuning after manual release QA.

## Implementation Reference
- `planning/roadmap.md` (Section 18, `P4.9.6`)
- `planning/p4.9.6-streaming-performance-pack.md`
```


## Coverage Check

- Total issues generated: 14
- Expected range: `M4.1` through `M4.8`, plus `P4.9.1` through `P4.9.6`
