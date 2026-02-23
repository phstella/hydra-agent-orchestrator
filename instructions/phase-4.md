# Phase 4: Interactive Session Mode (PTY)

**Goal**: Add a dedicated interactive mode where users can intervene mid-flight.

**Duration estimate**: 2-3 weeks

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M4.1 | PTY Supervisor Path for Interactive Sessions | M | M1.4 |
| M4.2 | Interactive Session Runtime and IPC Surface | M | M4.1, M3.2 |
| M4.3 | Interactive UI Shell and Terminal Panel | M | M4.2, M3.1 |
| M4.4 | Mid-Flight Intervention Controls | M | M4.2, M4.3 |
| M4.5 | Interactive Safety and Capability Gating | S | M2.1, M4.2 |
| M4.6 | Interactive Transcript Artifacts and E2E Tests | M | M4.3, M4.4, M4.5 |

## Implementation Order

1. **Core PTY lane**
- M4.1 first: build PTY spawn/write/resize/cancel path in supervisor without touching deterministic race path.
- M4.2 second: expose PTY sessions through Tauri IPC and runtime session manager.

2. **UI lane**
- M4.3 after M4.2: ship Interactive tab + terminal panel + session rail.
- M4.4 after M4.3: add intervention controls (send input, interrupt, resume/status feedback).

3. **Safety lane**
- M4.5 in parallel with M4.3/M4.4 once M4.2 exists: capability gating, preflight guardrails, explicit risk confirmation.

4. **Closeout lane**
- M4.6 final: persist transcripts/session artifacts and land integration + smoke coverage.

## Parallelization

After M4.2, run in parallel:

- UI lane: M4.3 -> M4.4
- Safety lane: M4.5

Then complete M4.6 as the final convergence milestone.

## What to Build

- **PTY supervisor path** (M4.1): Add an interactive supervisor mode with stdin write, terminal resize, and robust cancellation.

- **Session runtime + IPC** (M4.2): Introduce session-oriented commands (`start`, `write`, `resize`, `poll`, `stop`) and lifecycle-safe cleanup in `hydra-app`.

- **Interactive GUI shell** (M4.3): Add dedicated Interactive tab that resembles terminal-first workflows and can manage multiple sessions.

- **Mid-flight controls** (M4.4): Let users intervene during execution with clear command feedback and lifecycle transitions.

- **Safety gating** (M4.5): Keep interactive mode policy-bound (adapter capability checks, experimental warnings, launch preflight).

- **Artifacts + tests** (M4.6): Persist transcript artifacts and add regression coverage for session start/input/interrupt/cleanup flows.

## Exit Criteria

1. User can launch interactive session and send input while the agent is running.
2. Terminal output is human-readable and stable under streaming load.
3. Session transcripts are persisted in artifacts and can be replayed.
4. Existing race/scoring/merge paths remain deterministic and unaffected.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 7
- Issue bodies: `planning/issues/phase-4.md`
- Detailed implementation contract: `planning/p4-interactive-session-implementation-guide.md`
- Design direction: `docs/ui-mocks/main_screen.png`
