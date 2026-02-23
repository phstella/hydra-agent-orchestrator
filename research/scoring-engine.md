# Scoring Engine

## Purpose

The scoring engine is Hydra's core differentiator. After agents complete a task in their isolated worktrees, the engine evaluates each solution across multiple quality dimensions, computes a weighted score, and ranks them. This transforms a subjective "which diff looks better" into an objective, reproducible comparison.

---

## Scoring Dimensions

### 1. Build (default weight: 30)

Does the agent's code compile/build successfully?

**Process:**
1. Run the configured `build_command` inside the agent's worktree directory
2. Capture exit code, stdout, and stderr
3. Score:
   - Exit code 0 → 100 points
   - Exit code non-zero → 0 points

**Rationale:** A solution that doesn't build is fundamentally broken. This is a binary pass/fail gate with the highest default weight.

```rust
pub async fn score_build(worktree: &Path, command: &str) -> BuildScore {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(worktree)
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => BuildScore { passed: true, score: 100.0 },
        Ok(o) => BuildScore {
            passed: false,
            score: 0.0,
            // stderr captured for display in results
        },
        Err(e) => BuildScore { passed: false, score: 0.0 },
    }
}
```

### 2. Tests (default weight: 30)

How many tests pass in the agent's worktree?

**Process:**
1. Run the baseline test suite in the original repo to get `baseline_pass_count` and `baseline_total_count`
2. Run the configured `test_command` inside the agent's worktree
3. Parse test output for pass/fail counts (adapter-specific parsers for common frameworks)
4. Score formula:

```
pass_rate = agent_pass_count / agent_total_count

If agent_total_count > baseline_total_count:
    bonus = (new_tests / agent_total_count) * 10  // reward adding tests
Else:
    bonus = 0

If agent_pass_count >= baseline_pass_count:
    regression_penalty = 0
Else:
    regression_penalty = ((baseline_pass_count - agent_pass_count) / baseline_pass_count) * 50

score = (pass_rate * 100) + bonus - regression_penalty
score = clamp(score, 0, 100)
```

**Rationale:** Tests are the strongest signal for correctness. Agents that break existing tests are heavily penalized. Agents that add new passing tests get a small bonus.

**Test output parsing:** Hydra ships parsers for common test runners:
- `npm test` / `jest` → look for "Tests: X passed, Y failed"
- `cargo test` → look for "test result: ok. X passed; Y failed"
- `pytest` → look for "X passed, Y failed"
- `go test` → look for "ok" or "FAIL" lines
- Fallback: exit code only (pass = 100, fail = 0)

### 3. Lint (default weight: 15)

How clean is the agent's code compared to the baseline?

**Process:**
1. Run the configured `lint_command` in the original repo → `baseline_warnings`, `baseline_errors`
2. Run the same command in the agent's worktree → `agent_warnings`, `agent_errors`
3. Score formula:

```
new_errors = max(0, agent_errors - baseline_errors)
new_warnings = max(0, agent_warnings - baseline_warnings)
resolved_issues = max(0, (baseline_errors + baseline_warnings) - (agent_errors + agent_warnings))

error_penalty = new_errors * 10
warning_penalty = new_warnings * 2
resolution_bonus = resolved_issues * 1

score = 100 - error_penalty - warning_penalty + resolution_bonus
score = clamp(score, 0, 100)
```

**Rationale:** New lint errors indicate sloppy code. Agents that resolve pre-existing lint issues get a small bonus. Errors weigh 5x more than warnings.

### 4. Diff Size (default weight: 15)

How focused is the agent's change?

**Process:**
1. Run `git diff --stat <base_branch>..<agent_branch>` in the worktree
2. Extract lines added, lines removed, files changed
3. Score formula:

```
total_churn = lines_added + lines_removed
files_touched = files_changed

// Penalize excessive churn (more than 500 lines is suspicious for most tasks)
if total_churn <= 100:
    churn_score = 100
elif total_churn <= 500:
    churn_score = 100 - ((total_churn - 100) / 400) * 40  // linear decay 100→60
else:
    churn_score = max(20, 60 - ((total_churn - 500) / 1000) * 40)

// Penalize touching too many files
if files_touched <= 5:
    file_score = 100
elif files_touched <= 15:
    file_score = 100 - ((files_touched - 5) / 10) * 30
else:
    file_score = max(30, 70 - ((files_touched - 15) / 20) * 40)

score = (churn_score * 0.6) + (file_score * 0.4)
```

**Rationale:** AI agents have a tendency to make unnecessary changes, refactor unrelated code, or produce verbose implementations. Smaller, focused diffs are generally better — they are easier to review, less likely to introduce regressions, and indicate the agent understood the task scope.

### 5. Speed (default weight: 10)

How fast did the agent complete the task?

**Process:**
1. Record `started_at` and `completed_at` for each agent
2. Compute `duration_seconds` for each
3. Rank agents by duration: fastest gets 100, others scaled relative to fastest

```
fastest = min(all agent durations)

for each agent:
    ratio = fastest / agent.duration
    score = ratio * 100
    // Agent twice as slow as fastest → 50 points
    // Agent same speed as fastest → 100 points
```

**Rationale:** Speed matters when you're paying per-token. A slower agent is burning more money for the same task. However, speed has the lowest default weight because a fast wrong answer is worse than a slow right one.

---

## Composite Score Calculation

```rust
pub fn calculate_composite(breakdown: &ScoreBreakdown, weights: &Weights) -> f64 {
    let total_weight = weights.build + weights.tests + weights.lint
        + weights.diff_size + weights.speed;

    let weighted_sum =
        breakdown.build.unwrap_or(0.0) * weights.build as f64
        + breakdown.tests.unwrap_or(0.0) * weights.tests as f64
        + breakdown.lint.unwrap_or(0.0) * weights.lint as f64
        + breakdown.diff_size.unwrap_or(0.0) * weights.diff_size as f64
        + breakdown.speed.unwrap_or(0.0) * weights.speed as f64;

    weighted_sum / total_weight as f64
}
```

### Example Calculation

Three agents complete a task. Weights: build=30, tests=30, lint=15, diff_size=15, speed=10.

| Dimension | Claude Code | Codex CLI | Cursor CLI |
|---|---|---|---|
| Build | 100 (passes) | 100 (passes) | 0 (fails) |
| Tests | 95 (1 new test, all pass) | 80 (all pass, no new) | 0 (can't run, build failed) |
| Lint | 90 (2 new warnings) | 100 (clean) | 0 |
| Diff size | 75 (moderate churn) | 95 (minimal changes) | 60 (scattered changes) |
| Speed | 80 (45s) | 100 (36s, fastest) | 70 (51s) |

Composite:
- **Claude Code:** (100×30 + 95×30 + 90×15 + 75×15 + 80×10) / 100 = **91.0**
- **Codex CLI:** (100×30 + 80×30 + 100×15 + 95×15 + 100×10) / 100 = **93.3** (winner)
- **Cursor CLI:** (0×30 + 0×30 + 0×15 + 60×15 + 70×10) / 100 = **16.0**

---

## Evaluation Pipeline

The scoring engine runs sequentially through each dimension to avoid resource contention:

```
1. Baseline capture (once per race)
   ├── Run build_command in original repo → baseline build state
   ├── Run test_command in original repo → baseline test counts
   └── Run lint_command in original repo → baseline lint counts

2. Per-agent evaluation (parallel across agents)
   ├── Build score     (run build_command in worktree)
   ├── Test score      (run test_command in worktree)
   ├── Lint score      (run lint_command in worktree)
   ├── Diff size score (git diff --stat)
   └── Speed score     (from AgentProcess metadata)

3. Ranking
   ├── Calculate composite scores
   ├── Sort descending
   └── Emit ScoringResult event
```

Steps within "per-agent evaluation" run sequentially per agent (build before tests, since a failed build makes tests meaningless), but different agents can be evaluated in parallel.

### Short-Circuit Optimization

If an agent's build fails (score = 0), skip tests and lint for that agent — they would all fail anyway. This saves time and compute:

```rust
let build = score_build(worktree, &config.build_command).await;
if !build.passed {
    return AgentScore {
        breakdown: ScoreBreakdown {
            build: Some(0.0),
            tests: Some(0.0),
            lint: Some(0.0),
            diff_size: Some(score_diff_size(worktree, base_branch).await),
            speed: Some(speed_score),
        },
        ..
    };
}
```

---

## Configuration

Via `hydra.toml`:

```toml
[scoring]
weights = { build = 30, tests = 30, lint = 15, diff_size = 15, speed = 10 }
build_command = "npm run build"
test_command = "npm test"
lint_command = "npm run lint"
timeout_per_check_seconds = 120

[scoring.thresholds]
auto_merge_minimum = 85.0    # Auto-merge if winner scores above this
fail_maximum = 30.0          # Flag as "all agents failed" below this
```

### Defaults When Commands Are Missing

If a project doesn't configure a `build_command`, `test_command`, or `lint_command`, the corresponding dimension is excluded from scoring and the remaining weights are renormalized.

Example: no `test_command` configured → weights become build=30, lint=15, diff_size=15, speed=10 → total=70 → normalized to 100.

---

## Output Format

### CLI Output

```
Race Results — Task: "implement JWT auth"
============================================

  #1  Codex CLI      93.3 / 100    [BUILD: ✓] [TESTS: 80] [LINT: 100] [DIFF: 95] [SPEED: 100]
  #2  Claude Code    91.0 / 100    [BUILD: ✓] [TESTS: 95] [LINT: 90]  [DIFF: 75] [SPEED: 80]
  #3  Cursor CLI     16.0 / 100    [BUILD: ✗] [TESTS: --] [LINT: --]  [DIFF: 60] [SPEED: 70]

Winner: Codex CLI (branch: hydra/codex-1708646400)
Run `hydra merge codex-1708646400` to merge into your branch.
```

### GUI Output

The `ScoreBoard` React component receives `ScoringResult` events and renders:
- Ranked cards with agent name, total score, and a breakdown bar chart
- Color coding: green (>80), yellow (50-80), red (<50)
- Expandable detail panels showing raw build/test/lint output
- "Merge Winner" button on the top-ranked card
