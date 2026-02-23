# GitHub Issue Body Pack

Last updated: 2026-02-23

This repository now maintains canonical copy-paste issue bodies in per-phase files under `planning/issues/`.

## Global Label Prefix

- `hydra` (add this label to every issue)

## Source of Truth

- Milestone definitions and dependencies: `planning/implementation-checklist.md`
- Per-phase issue bodies:
  - `planning/issues/phase-0.md` (M0.1-M0.8)
  - `planning/issues/phase-1.md` (M1.1-M1.8)
  - `planning/issues/phase-2.md` (M2.1-M2.12)
  - `planning/issues/phase-3.md` (M3.1-M3.7)
  - `planning/issues/phase-4.md` (M4.1-M4.6)
  - `planning/issues/phase-5.md` (M5.1-M5.6)
  - `planning/issues/phase-6.md` (M6.1-M6.6)

## Milestone Range

- Current range: `M0.1` through `M6.6`
- Total milestones: 53

## Usage

1. Open the phase file matching the active roadmap phase.
2. Copy the issue body block for the target milestone.
3. Preserve milestone prefix in title: `[M#.##]`.
4. Apply labels from the issue header (`phase-*`, `area-*`, `type-*`, plus `hydra`).

## Notes

- If milestone content changes, update `planning/implementation-checklist.md` first.
- Regenerate/sync the per-phase issue files after checklist changes to prevent drift.
