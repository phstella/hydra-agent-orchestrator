# Phase 3: GUI Alpha (Tauri)

**Goal**: Visual monitoring and result review via desktop application.

**Duration estimate**: 3-4 weeks

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M3.1 | Tauri App Bootstrap | S | M1.1 |
| M3.2 | IPC Command Surface (Race) | M | M2.10, M3.1 |
| M3.3 | Live Agent Output Panels | M | M3.2 |
| M3.4 | Scoreboard and Mergeability UI | S | M2.7, M3.2 |
| M3.5 | Diff Viewer Integration | M | M3.2 |
| M3.6 | Experimental Adapter UX Warnings | S | M2.1, M2.9, M3.2 |
| M3.7 | GUI Smoke Test Pack | M | M3.3, M3.4, M3.5 |

## Parallelization

After M3.1 and M3.2 (bootstrap + IPC), three UI components build in parallel:

- M3.3 (output panels), M3.4 (scoreboard), M3.5 (diff viewer)
- M3.6 can be done alongside any UI work.
- M3.7 is the smoke test gate.

## What to Build

- **Tauri scaffold** (M3.1): Tauri v2 + React + TypeScript. Shared type generation.
- **IPC surface** (M3.2): Commands for starting races, streaming events, fetching results.
- **UI components** (M3.3-M3.6): Live output panels (xterm.js), score cards,
  diff viewer (Monaco), experimental adapter warnings.
- **Smoke tests** (M3.7): Startup, race flow, merge action validation.

See `docs/architecture.md` section 2 for runtime topology and
`docs/tech-stack.md` section 7 for frontend library choices.

## Exit Criteria

1. Linux GUI can start and monitor a multi-agent race.
2. Results are equivalent to CLI data.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 6
- Issue bodies: `planning/issues/phase-3.md`
