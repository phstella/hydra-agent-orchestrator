# Scoring Engine

Last updated: 2026-02-23

## 1. Objective

Hydra scoring converts "which diff feels better" into a reproducible ranking.

Scoring must be:
- deterministic from captured artifacts
- interpretable (dimension breakdown visible)
- configurable per repository
- resistant to common AI-agent failure patterns (scope creep, broken tests, noisy refactors)

## 2. Scoring Principles

1. Correctness over speed.
2. Regressions are penalized more than no-op outcomes.
3. Baseline normalization is mandatory.
4. Missing dimensions should renormalize, not silently bias totals.

## 3. Dimension Set (Default)

| Dimension | Default Weight | Purpose |
|---|---:|---|
| Build | 30 | Hard viability gate |
| Tests | 30 | Correctness and regression control |
| Lint | 15 | Maintainability signal |
| Diff Scope | 15 | Focus and reviewability |
| Speed | 10 | Throughput/cost proxy |

Total = 100.

## 4. Baseline Capture

Before agents run, Hydra captures baseline signals on base ref:

- `build_baseline`: pass/fail and duration
- `test_baseline`: pass/fail counts and failures
- `lint_baseline`: warning/error counts

Why this matters:
- some repos already fail tests/lint
- agents should not be punished for pre-existing failures unless they worsen them

## 5. Dimension Definitions

### 5.1 Build score

Binary by default:
- pass => 100
- fail => 0

Optional gradient mode (future):
- partial credit for specific build targets

### 5.2 Test score

Let:
- `B_pass`: baseline passed tests
- `A_pass`: agent passed tests
- `A_total`: agent total tests
- `new_tests = max(0, A_total - baseline_total)`

Formula:

```text
pass_rate = if A_total == 0 then 0 else A_pass / A_total
regression = max(0, B_pass - A_pass)
reg_penalty = if B_pass == 0 then 0 else (regression / B_pass) * 60
new_test_bonus = if new_tests > 0 then min(10, new_tests * 0.5) else 0
score = clamp((pass_rate * 100) - reg_penalty + new_test_bonus, 0, 100)
```

### 5.3 Lint score

Let:
- `new_errors = max(0, A_errors - B_errors)`
- `new_warnings = max(0, A_warnings - B_warnings)`
- `resolved = max(0, (B_errors + B_warnings) - (A_errors + A_warnings))`

Formula:

```text
score = clamp(100 - (new_errors * 12) - (new_warnings * 2) + (resolved * 1), 0, 100)
```

### 5.4 Diff scope score

Inputs:
- lines added/removed
- files touched
- optional path scope policy

Heuristics:
- modest churn scores highest
- broad unrelated edits penalized
- out-of-scope path edits can trigger hard penalty

Formula skeleton:

```text
scope_score = weighted(churn_score, files_score, scope_violation_score)
```

Recommended hard guard:
- if "protected paths" changed unexpectedly, cap diff scope score at 30.

### 5.5 Speed score

Relative to fastest successful agent:

```text
fastest = min(successful_agent_durations)
score = clamp((fastest / agent_duration) * 100, 0, 100)
```

If agent fails, speed is still computed but only used if policy allows (default: keep).

## 6. Composite Score

```text
composite = sum(dimension_score * dimension_weight) / sum(active_weights)
```

`active_weights` excludes missing/disabled dimensions.

## 7. Gating Rules (Recommended)

Before ranking, apply policy gates:

1. If build fails => mark `not_mergeable`.
2. If tests regress beyond threshold => mark `not_mergeable`.
3. If security check (optional command) fails => mark `not_mergeable`.

Ranking still shown, but merge action disabled by default for non-mergeable candidates.

## 8. Language/Repo Profiles

Hydra should ship profile presets:

- `js-node`:
  - build: `npm run build`
  - test: `npm test -- --runInBand`
  - lint: `npm run lint`
- `rust`:
  - build: `cargo build --all-targets`
  - test: `cargo test`
  - lint: `cargo clippy --all-targets -- -D warnings`
- `python`:
  - build: optional
  - test: `pytest -q`
  - lint: `ruff check .`

## 9. Determinism and Reproducibility

Store all scoring artifacts:
- raw command outputs
- parsed summaries
- formulas and effective weights
- scoring engine version

This supports exact replay and audit.

## 10. Anti-Gaming Controls

Potential gaming pattern: agent reduces tests to inflate pass rate.

Mitigations:
1. Compare total test count against baseline.
2. Penalize dropped tests.
3. Optionally fail score if test discovery drops unexpectedly.

Potential gaming pattern: formatting-only huge diff.

Mitigations:
1. Diff scope penalty for excessive churn.
2. Optional formatter-aware diff normalization (future).

## 11. Performance Model

Scoring execution plan:
- baseline commands once per run
- per-agent checks run in parallel with per-check timeout
- command stdout/stderr truncated for UI but full logs saved to artifact files

Recommended default timeouts:
- build: 300s
- test: 600s
- lint: 300s

## 12. Example `hydra.toml`

```toml
[scoring]
profile = "js-node"
timeout_per_check_seconds = 300

[scoring.weights]
build = 30
tests = 30
lint = 15
diff_scope = 15
speed = 10

[scoring.gates]
require_build_pass = true
max_test_regression_percent = 0

[scoring.diff_scope]
max_files_soft = 20
max_churn_soft = 800
protected_paths = ["infra/", "scripts/release/"]
```

## 13. Output Contract

CLI and GUI should receive:

```json
{
  "run_id": "...",
  "rankings": [
    {
      "agent": "codex",
      "mergeable": true,
      "total": 91.2,
      "breakdown": {
        "build": 100,
        "tests": 92,
        "lint": 88,
        "diff_scope": 90,
        "speed": 84
      }
    }
  ]
}
```

## 14. Open Scoring Questions

1. Should speed be replaced with direct cost-per-success once token pricing capture is stable?
2. Should we add security scan dimension by default (`npm audit`, `cargo audit`, `pip-audit`), or keep optional?
3. Should we support pairwise preference learning from user merge choices for weight auto-tuning?

## 15. Related Docs

- `research/architecture.md`
- `research/collaboration-workflows.md`
- `research/roadmap.md`
