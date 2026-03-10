# Phase 4: Interactive Session Mode (PTY)

**Goal**: Add a dedicated interactive mode where users can intervene mid-flight.

**Duration estimate**: 3-4 weeks (including pre-Phase-5 cockpit convergence gate)
**Execution mode**: Local-first (no GitHub issue creation required for implementation).
**Desktop focus**: `M4.7` is desktop-first (`>=1280px` primary, `>=1024px` minimum).

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M4.1 | PTY Supervisor Path for Interactive Sessions | M | M1.4 |
| M4.2 | Interactive Session Runtime and IPC Surface | M | M4.1, M3.2 |
| M4.3 | Interactive UI Shell and Terminal Panel | M | M4.2, M3.1 |
| M4.4 | Mid-Flight Intervention Controls | M | M4.2, M4.3 |
| M4.5 | Interactive Safety and Capability Gating | S | M2.1, M4.2 |
| M4.6 | Interactive Transcript Artifacts and E2E Tests | M | M4.3, M4.4, M4.5 |
| M4.7 | Unified Race Cockpit UX Convergence (Pre-Phase 5 Gate) | L | M3.5, M4.4, M4.6 |

## Implementation Order

0. **Execution contract**
- Use `planning/m4.7-local-execution-pack.md` as the active workboard for M4.7 (status, owner, evidence).
- Keep `planning/issues/phase-4.md` as optional synchronization material only.

1. **Core PTY lane**
- M4.1 first: build PTY spawn/write/resize/cancel path in supervisor without touching deterministic race path.
- M4.2 second: expose PTY sessions through Tauri IPC and runtime session manager.

2. **UI lane**
- M4.3 after M4.2: ship Interactive tab + terminal panel + session rail.
- M4.4 after M4.3: add intervention controls (send input, interrupt, resume/status feedback).

3. **Safety lane**
- M4.5 in parallel with M4.3/M4.4 once M4.2 exists: capability gating, preflight guardrails, explicit risk confirmation.

4. **Closeout lane**
- M4.6 first: persist transcripts/session artifacts and land integration + smoke coverage.
- M4.7 final gate: converge split tabs into a single operator cockpit before Phase 5 starts.

## Parallelization

After M4.2, run in parallel:

- UI lane: M4.3 -> M4.4
- Safety lane: M4.5

Then complete M4.6 and M4.7 as final convergence milestones.

## What to Build

- **PTY supervisor path** (M4.1): Add an interactive supervisor mode with stdin write, terminal resize, and robust cancellation.

- **Session runtime + IPC** (M4.2): Introduce session-oriented commands (`start`, `write`, `resize`, `poll`, `stop`) and lifecycle-safe cleanup in `hydra-app`.

- **Interactive GUI shell** (M4.3): Add dedicated Interactive tab that resembles terminal-first workflows and can manage multiple sessions.

- **Mid-flight controls** (M4.4): Let users intervene during execution with clear command feedback and lifecycle transitions.

- **Safety gating** (M4.5): Keep interactive mode policy-bound (adapter capability checks, experimental warnings, launch preflight).

- **Artifacts + tests** (M4.6): Persist transcript artifacts and add regression coverage for session start/input/interrupt/cleanup flows.

- **Cockpit convergence** (M4.7): Ship a unified dashboard (left rail + center race/terminal + right leaderboard) with inline intervention and completion summary to eliminate cross-tab operator flow.
  Implement this milestone desktop-first per local pack breakpoints and defer mobile/touch polish.

## Exit Criteria

1. User can launch interactive session and send input while the agent is running.
2. Terminal output is human-readable and stable under streaming load.
3. Session transcripts are persisted in artifacts and can be replayed.
4. Existing race/scoring/merge paths remain deterministic and unaffected.
5. Cockpit becomes the default operator flow surface, with race launch, live monitoring, intervention, and review transition in one place.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 7
- Local execution conventions: `planning/local-execution-conventions.md`
- Local execution pack (primary): `planning/m4.7-local-execution-pack.md`
- Issue bodies (optional sync only): `planning/issues/phase-4.md`
- Detailed implementation contract: `planning/p4-interactive-session-implementation-guide.md`
- Cockpit convergence contract: `planning/p4-race-cockpit-convergence-implementation-guide.md`
- Desktop UI contract: `planning/m4.7-desktop-ui-contract.md`
- Design direction: `docs/ui-mocks/main_screen.png`
