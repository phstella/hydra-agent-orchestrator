# Phase Instructions

Per-phase implementation guidance for AI agents working on Hydra.

Each file provides: goal, milestones, parallelization strategy, what to build,
exit criteria, and pointers to detailed docs.

| File | Phase | Goal |
|------|-------|------|
| `phase-0.md` | Validation and Guardrails | Lock assumptions, verify architecture, security baseline |
| `phase-1.md` | Core Orchestrator + Single Agent | Stable end-to-end loop for one agent |
| `phase-2.md` | Multi-Agent Race + Scoring | Concurrent runs and objective ranking |
| `phase-3.md` | GUI Alpha (Tauri) | Visual monitoring and result review |
| `phase-4.md` | Interactive Session Mode (PTY) | Mid-flight user intervention workflow |
| `phase-5.md` | Collaboration Workflows | Structured multi-agent cooperation |
| `phase-6.md` | Windows Parity + Hardening | Cross-platform stability and release |

## How to Use

1. Read `CLAUDE.md` and `PROGRESS.md` first.
2. Open the phase file matching the current phase in `PROGRESS.md`.
3. Follow the milestone order and parallelization guidance.
4. Check acceptance criteria in `planning/implementation-checklist.md`.
5. Open any linked implementation guide in `planning/` for milestone-level contracts.
6. Update `PROGRESS.md` as you complete milestones.
