# Phase 2 Tickets (Multi-Agent Race + Scoring) Issue Bodies

Last updated: 2026-02-22

Generated from `research/implementation-checklist.md`.

Global label prefix: `hydra`

## [M2.1] Adapter Registry and Tier Policy

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-adapter, type-feature
- Estimate: S
- Dependencies: M1.6

### Issue Body (Markdown)

```md
## Problem
Phase 1 implements individual adapters, but there is no central registry that enforces tier policy (Tier-1 vs experimental). Without a registry, the system cannot programmatically select which adapters to use in a run or block experimental adapters by default.

## Scope
Implement an adapter registry that discovers available adapters, applies tier policy, and exposes the filtered set to the orchestrator. Default runs select only Tier-1 adapters; experimental adapters require `--allow-experimental-adapters`.

## Acceptance Criteria
- [ ] Registry supports Tier-1 and experimental tiers.
- [ ] Default run selects only Tier-1 adapters.
- [ ] Experimental adapters require explicit opt-in flag.

## Out of Scope
Dynamic adapter loading; third-party adapter registration.

## Dependencies
- M1.6

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.2] Parallel Spawn and Supervision

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-core, type-feature
- Estimate: M
- Dependencies: M1.4, M2.1

### Issue Body (Markdown)

```md
## Problem
Phase 1 supervisor handles one agent at a time. Race mode requires spawning multiple agents concurrently with independent lifecycle management so that one agent's failure does not kill the others.

## Scope
Extend the process supervisor to manage multiple concurrent agent processes. Implement independent failure isolation, aggregate status computation, and concurrent event stream merging.

## Acceptance Criteria
- [ ] Two Tier-1 agents run concurrently.
- [ ] One agent failure does not kill others.
- [ ] Aggregate run status is deterministic.

## Out of Scope
Resource throttling; agent priority scheduling.

## Dependencies
- M1.4, M2.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.3] Baseline Capture Engine

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: S
- Dependencies: M1.2

### Issue Body (Markdown)

```md
## Problem
Scoring requires a baseline to compare agent outputs against. Without capturing build/test/lint state on the base ref before agents run, the scoring engine cannot distinguish pre-existing failures from agent-introduced regressions.

## Scope
Run configured build/test/lint commands on the base ref before agent execution. Persist baseline results as artifacts. Handle missing commands gracefully with explicit unavailable status.

## Acceptance Criteria
- [ ] Build/test/lint baseline captured once per run.
- [ ] Baseline outputs persisted as artifacts.
- [ ] Missing commands handled with explicit status.

## Out of Scope
Baseline caching across runs; custom baseline commands.

## Dependencies
- M1.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.4] Scoring Dimension: Build

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: S
- Dependencies: M2.3

### Issue Body (Markdown)

```md
## Problem
No build score dimension exists yet. Build pass/fail is the most fundamental viability gate: a broken build means the agent output cannot be used.

## Scope
Implement the build scoring dimension (pass=100, fail=0). Run the configured build command in each agent's worktree. Handle timeouts and command failures. Include raw evidence references in score payload.

## Acceptance Criteria
- [ ] Build score computed per candidate.
- [ ] Timeout and command failure paths tested.
- [ ] Score payload includes raw evidence references.

## Out of Scope
Partial build credit; incremental build support.

## Dependencies
- M2.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.5] Scoring Dimension: Tests

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: M
- Dependencies: M2.3

### Issue Body (Markdown)

```md
## Problem
Test pass rate alone is insufficient for scoring. Agents can game scores by deleting tests or introducing regressions that were masked by other failures. The scoring dimension needs to be regression-aware and resistant to test-drop manipulation.

## Scope
Implement the test scoring formula with regression penalty, new-test bonus, and baseline comparison. Add parser fallback to exit-code mode for test frameworks that do not produce structured output. Include anti-gaming checks for dropped test counts.

## Acceptance Criteria
- [ ] Regression-aware formula implemented.
- [ ] Parser fallback to exit-code mode works.
- [ ] Test-drop anti-gaming checks included.

## Out of Scope
Per-test-case tracking; flaky test detection.

## Dependencies
- M2.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.6] Scoring Dimension: Lint and Diff Scope

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: M
- Dependencies: M2.3

### Issue Body (Markdown)

```md
## Problem
Agents may introduce lint violations or make broad, unfocused changes that touch files outside the task scope. Without lint and diff scope scoring, there is no signal for code maintainability or change reviewability.

## Scope
Implement lint delta scoring (new errors/warnings vs baseline). Implement diff scope scoring (files touched, lines churned, protected path violations). Make protected path penalty configurable via `hydra.toml`.

## Acceptance Criteria
- [ ] Lint delta scoring implemented.
- [ ] Diff scope scoring includes file/churn checks.
- [ ] Protected path penalty is configurable.

## Out of Scope
Formatter-aware diff normalization; semantic diff analysis.

## Dependencies
- M2.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.7] Composite Ranking and Mergeability Gates

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: S
- Dependencies: M2.4, M2.5, M2.6

### Issue Body (Markdown)

```md
## Problem
Individual dimension scores exist but there is no composite ranking or mergeability decision. Users need a single ranked list with clear merge/no-merge signals.

## Scope
Implement weighted composite score calculation with dimension renormalization for missing dimensions. Apply mergeability gates (build must pass, test regression below threshold). Expose ranking and gate results in structured output.

## Acceptance Criteria
- [ ] Weighted composite scores are reproducible.
- [ ] Missing dimensions renormalize weights.
- [ ] Mergeability gates are exposed in output.

## Out of Scope
User-adjustable weights at runtime; pairwise preference learning.

## Dependencies
- M2.4, M2.5, M2.6

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.8] CLI Merge Command with Dry-Run

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-core, type-feature
- Estimate: M
- Dependencies: M2.7

### Issue Body (Markdown)

```md
## Problem
After scoring, users need a safe way to merge the winning agent's branch. Without dry-run support, users must manually run git merge and may encounter unexpected conflicts.

## Scope
Implement `hydra merge` command with `--dry-run` mode that reports potential conflicts without modifying the working tree. Real merge requires explicit `--confirm` flag. Write conflict report artifact on merge failure.

## Acceptance Criteria
- [ ] Dry-run reports potential conflicts.
- [ ] Real merge requires explicit confirmation flag.
- [ ] Conflict report artifact is written on failure.

## Out of Scope
Automatic conflict resolution; cherry-pick mode.

## Dependencies
- M2.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.9] Experimental Cursor Opt-In Path

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.4, M2.1, M2.2

### Issue Body (Markdown)

```md
## Problem
Cursor's CLI stability is not at Tier-1 level, but users may want to include it in race runs for comparison. The experimental adapter path must enforce opt-in gating to prevent accidental use and clearly communicate risk.

## Scope
Wire the Cursor adapter through the registry with experimental tier classification. Require `--allow-experimental-adapters` flag for inclusion. Label all Cursor output as experimental. Block runtime activation if probe fails.

## Acceptance Criteria
- [ ] Cursor can run only with --allow-experimental-adapters.
- [ ] Output labels include experimental warning.
- [ ] Failing probe blocks runtime activation.

## Out of Scope
Cursor Tier-1 promotion; Cursor-specific output parsing improvements.

## Dependencies
- M0.4, M2.1, M2.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.10] End-to-End Race Integration Test

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-test, type-test
- Estimate: M
- Dependencies: M2.2, M2.3, M2.4, M2.5, M2.6, M2.7, M2.8

### Issue Body (Markdown)

```md
## Problem
Individual scoring dimensions and parallel execution are tested in isolation, but no test validates the full race flow from spawn to ranked output with complete artifacts.

## Scope
Write an end-to-end integration test that starts a multi-agent race, verifies scoring output shape, checks artifact completeness, and confirms reproducibility from saved artifacts.

## Acceptance Criteria
- [ ] Full race test verifies ranking output shape.
- [ ] Artifacts are complete and replayable.
- [ ] Linux and Windows CI jobs pass.

## Out of Scope
GUI integration; workflow mode testing.

## Dependencies
- M2.2, M2.3, M2.4, M2.5, M2.6, M2.7, M2.8

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.11] Cost and Budget Engine

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-scoring, type-feature
- Estimate: M
- Dependencies: M2.2, M2.7

### Issue Body (Markdown)

```md
## Problem
Race mode runs multiple agents on the same task, multiplying API cost. Agents that report token usage (via `emits_usage` capability) produce cost-relevant data, but Hydra has no system to capture, normalize, aggregate, or display this data. Users cannot make cost-informed decisions about which agent to prefer or when to stop a run.

## Scope
Implement token usage capture from agent event streams. Normalize usage data across adapters. Aggregate per-run and per-agent cost estimates. Add budget stop conditions (`max_tokens_total`, `max_cost_usd`) that terminate agents when limits are exceeded. Display cost summary in CLI race output.

## Acceptance Criteria
- [ ] Token usage from adapters that emit it is captured and persisted in run artifacts.
- [ ] Per-agent and per-run cost estimates are included in scoring output.
- [ ] Budget limits in `hydra.toml` stop agents when exceeded.
- [ ] Adapters that do not emit usage data produce explicit `unavailable` status rather than silent omission.

## Out of Scope
Real-time cost streaming to GUI; historical cost trend analysis.

## Dependencies
- M2.2, M2.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M2.12] Observability Contract

- Phase: Phase 2 Tickets (Multi-Agent Race + Scoring)
- Labels: hydra, phase-2, area-core, type-feature
- Estimate: S
- Dependencies: M2.10

### Issue Body (Markdown)

```md
## Problem
The event schema (events.jsonl) is not versioned, making it fragile across Hydra updates. Run health indicators exist informally in logs but are not structured for programmatic consumption. Without a stable observability contract, the GUI and external tooling cannot reliably consume run data.

## Scope
Define and version the event schema. Add a schema version field to `manifest.json`. Implement minimum run health indicators (success rate, overhead timing, adapter error counts) as structured output. Ensure CLI `--json` output and artifact schema are documented and stable.

## Acceptance Criteria
- [ ] `manifest.json` includes a `schema_version` field.
- [ ] Event types are enumerated in a versioned schema definition.
- [ ] Run health metrics (success rate, orchestration overhead, adapter error rate) are computable from persisted artifacts.
- [ ] Breaking schema changes require version bump and migration note.

## Out of Scope
Prometheus/Grafana export; GUI dashboard integration.

## Dependencies
- M2.10

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## Coverage Check

- Total issues generated: 12
- Expected range: `M2.1` through `M2.12`
