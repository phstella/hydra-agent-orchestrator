# Phase 5: Collaboration Workflows

**Goal**: Move beyond race mode into structured multi-agent cooperation.

**Duration estimate**: 2-3 weeks

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M5.1 | Workflow Engine Core | M | M2.10 |
| M5.2 | Builder-Reviewer-Refiner Preset | M | M5.1 |
| M5.3 | Specialization Preset | M | M5.1 |
| M5.4 | Iterative Refinement Preset | M | M5.1, M2.7 |
| M5.5 | Workflow CLI and GUI Timeline | M | M5.2, M5.3, M5.4 |
| M5.6 | Workflow Integration Tests | M | M5.2, M5.3, M5.4 |

## Parallelization

After M5.1 (workflow engine core), the three presets build in parallel:

- M5.2, M5.3, M5.4 can be developed concurrently.
- M5.5 and M5.6 follow once all presets are complete.

## What to Build

- **Workflow engine** (M5.1): DAG step executor with artifact passing and
  per-node timeout/retry. See `docs/collaboration-workflows.md` section 2
  for runtime model.

- **Presets** (M5.2-M5.4): Builder/reviewer/refiner, specialization (parallel
  domain ownership), iterative refinement with convergence guard.
  See `docs/collaboration-workflows.md` sections 4-6 for specifications.

- **Timeline UI** (M5.5): CLI step timeline and GUI node timeline with
  artifact links.

- **Integration tests** (M5.6): One golden-path and one failure-path test
  per preset.

## Exit Criteria

1. Each preset has one golden integration test.
2. Workflow failures degrade gracefully with clear status.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 8
- Workflow specifications: `docs/collaboration-workflows.md`
- Issue bodies: `planning/issues/phase-5.md`
- Detailed implementation contract: `planning/p5-collaboration-workflows-implementation-guide.md`
