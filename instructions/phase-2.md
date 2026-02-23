# Phase 2: Multi-Agent Race + Scoring

**Goal**: Concurrent runs and objective ranking with merge support.

**Duration estimate**: 3-4 weeks

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M2.1 | Adapter Registry and Tier Policy | S | M1.6 |
| M2.2 | Parallel Spawn and Supervision | M | M1.4, M2.1 |
| M2.3 | Baseline Capture Engine | S | M1.2 |
| M2.4 | Scoring Dimension: Build | S | M2.3 |
| M2.5 | Scoring Dimension: Tests | M | M2.3 |
| M2.6 | Scoring Dimension: Lint and Diff Scope | M | M2.3 |
| M2.7 | Composite Ranking and Mergeability Gates | S | M2.4, M2.5, M2.6 |
| M2.8 | CLI Merge Command with Dry-Run | M | M2.7 |
| M2.9 | Experimental Cursor Opt-In Path | M | M0.4, M2.1, M2.2 |
| M2.10 | End-to-End Race Integration Test | M | M2.2-M2.8 |
| M2.11 | Cost and Budget Engine | M | M2.2, M2.7 |
| M2.12 | Observability Contract | S | M2.10 |

## Parallelization

Two major streams after Phase 1:

- **Orchestration stream**: M2.1 -> M2.2 -> (M2.9, feeds into M2.10)
- **Scoring stream**: M2.3 -> (M2.4, M2.5, M2.6 in parallel) -> M2.7 -> M2.8
- M2.10 is the integration gate merging both streams.
- M2.11 and M2.12 can follow after their dependencies.

## What to Build

- **Adapter registry** (M2.1): Central registry enforcing tier policy.
- **Parallel supervisor** (M2.2): Multi-agent concurrent execution with failure isolation.
- **Scoring engine** (M2.3-M2.7): Baseline capture, per-dimension scoring, composite
  ranking. See `docs/scoring-engine.md` for formulas and configuration.
- **Merge command** (M2.8): `hydra merge` with `--dry-run` and `--confirm`.
- **Cost engine** (M2.11): Token usage capture and budget stop conditions.
- **Observability** (M2.12): Versioned event schema and run health metrics.

## Exit Criteria

1. 2+ Tier-1 agents run concurrently without collisions.
2. Score output includes breakdown and artifacts.
3. Merge dry-run and real merge both tested.
4. Token usage captured and cost summary displayed.
5. Event schema versioned with `schema_version` field.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 5
- Scoring formulas: `docs/scoring-engine.md`
- Issue bodies: `planning/issues/phase-2.md`
