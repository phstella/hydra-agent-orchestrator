# Phase 0: Validation and Guardrails

**Goal**: Lock adapter assumptions, verify local run architecture on Linux,
establish security baseline, and finalize architecture decisions.

**Duration estimate**: ~1 week

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M0.1 | Adapter Probe Framework | M | none |
| M0.2 | Claude Probe Implementation | S | M0.1 |
| M0.3 | Codex Probe Implementation | S | M0.1 |
| M0.4 | Cursor Experimental Probe | M | M0.1 |
| M0.5 | Run Artifact Convention | S | none |
| M0.6 | Doctor Command MVP | S | M0.1, M0.2, M0.3, M0.4 |
| M0.7 | Security Baseline Implementation | M | M0.5 |
| M0.8 | Architecture Decision Lock | S | none |

## Parallelization

Three independent lanes can run concurrently:

- **Lane A** (adapter readiness): M0.1 -> (M0.2, M0.3, M0.4) -> M0.6
- **Lane B** (core scaffolding): M0.5 -> M0.7
- **Lane C** (architecture governance): M0.8 (no dependencies)

Cross-lane sync: M0.8 should complete before Phase 1 begins.

## What to Build

- **Probe framework** (M0.1): Trait-based probe interface returning structured
  results (binary path, version, flags, confidence). See `docs/agent-adapters.md`
  sections 2-3 for the adapter contract and capability model.

- **Adapter probes** (M0.2-M0.4): Per-adapter implementations. Claude and Codex
  are Tier-1; Cursor is experimental. See `docs/agent-adapters.md` sections 4-6
  for CLI surfaces and flags.

- **Artifact convention** (M0.5): Define `.hydra/runs/<run_id>/` layout with
  `manifest.json` and `events.jsonl`. See `docs/architecture.md` section 10 for
  observability requirements.

- **Doctor command** (M0.6): `hydra doctor` aggregates probe results and repo
  checks into JSON output.

- **Security baseline** (M0.7): Secret redaction rules, worktree sandbox
  enforcement, unsafe-mode guardrails.

- **ADR lock** (M0.8): Confirm ADR 6 (process model) and ADR 7 (storage model)
  are documented in `docs/architecture.md`.

## Exit Criteria

1. `hydra doctor` reports adapter readiness with stable JSON output.
2. Cursor probe is classified experimental and never default-enabled.
3. `.hydra/runs/<run_id>/manifest.json` and `events.jsonl` are consistently produced.
4. Secret redaction tests pass with known fixture patterns.
5. ADR entries for process model and storage model are finalized.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 3
- Sprint plan: `planning/sprint-1-cut.md`
- Issue bodies: `planning/issues/phase-0.md`
