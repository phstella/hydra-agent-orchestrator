# Phase Issue Packs

Last updated: 2026-02-25

This folder contains per-phase issue body packs generated from
`planning/implementation-checklist.md`.
They are optional synchronization artifacts when external issue trackers are used.
Local execution should be tracked in `planning/*-local-*.md` packs.

## Files

| File | Phase | Tickets | Count |
|---|---|---|---|
| `phase-0.md` | Validation and Guardrails | M0.1-M0.8 | 8 |
| `phase-1.md` | Core Orchestrator + Single Agent | M1.1-M1.8 | 8 |
| `phase-2.md` | Multi-Agent Race + Scoring | M2.1-M2.12 | 12 |
| `phase-3.md` | GUI Alpha | M3.1-M3.7 | 7 |
| `phase-4.md` | Interactive Session Mode | M4.1-M4.7 | 7 |
| `phase-5.md` | Collaboration Workflows | M5.1-M5.6 | 6 |
| `phase-6.md` | Windows Parity and Release Hardening | M6.1-M6.6 | 6 |

**Total: 54 milestones**

## Notes

- Global label prefix for all issues: `hydra`
- Source of truth for milestone definitions: `planning/implementation-checklist.md`
- Regenerate phase files after any checklist changes to avoid drift
- Local-first execution tracking reference: `planning/local-execution-conventions.md`
- For implementation detail beyond issue bodies, use:
  - `planning/p3-ui05-p3-qa01-implementation-guide.md` (Phase 3 closeout)
  - `planning/p4-interactive-session-implementation-guide.md` (Phase 4)
  - `planning/p4-race-cockpit-convergence-implementation-guide.md` (Phase 4.7 gate)
  - `planning/m4.7-desktop-ui-contract.md` (M4.7 desktop contract)
  - `planning/p5-collaboration-workflows-implementation-guide.md` (Phase 5)
