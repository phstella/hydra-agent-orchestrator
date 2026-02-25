# GitHub Issue Body Pack (Optional Sync)

Last updated: 2026-02-25

This repository maintains copy-paste issue bodies in per-phase files under
`planning/issues/` for optional sync to GitHub.
Primary implementation tracking is local-first via files under `planning/`
(for example `planning/m4.7-local-execution-pack.md`).

## Global Label Prefix

- `hydra` (add this label to every issue)

## Source of Truth

- Milestone definitions and dependencies: `planning/implementation-checklist.md`
- Detailed implementation guides for active phases:
  - `planning/p4-interactive-session-implementation-guide.md`
  - `planning/p4-race-cockpit-convergence-implementation-guide.md`
  - `planning/m4.7-desktop-ui-contract.md`
  - `planning/m4.7-local-execution-pack.md`
  - `planning/p5-collaboration-workflows-implementation-guide.md`
- Per-phase issue bodies:
  - `planning/issues/phase-0.md` (M0.1-M0.8)
  - `planning/issues/phase-1.md` (M1.1-M1.8)
  - `planning/issues/phase-2.md` (M2.1-M2.12)
  - `planning/issues/phase-3.md` (M3.1-M3.7)
  - `planning/issues/phase-4.md` (M4.1-M4.7)
  - `planning/issues/phase-5.md` (M5.1-M5.6)
  - `planning/issues/phase-6.md` (M6.1-M6.6)

## Milestone Range

- Current range: `M0.1` through `M6.6`
- Total milestones: 54

## Usage

For local-only execution, skip this file and update the local execution pack.

For GitHub sync:
1. Open the phase file matching the active roadmap phase.
2. Copy the issue body block for the target milestone.
3. Preserve milestone prefix in title: `[M#.##]`.
4. Apply labels from the issue header (`phase-*`, `area-*`, `type-*`, plus `hydra`).

## Notes

- If milestone content changes, update `planning/implementation-checklist.md` first.
- Regenerate/sync the per-phase issue files after checklist changes to prevent drift.
