# Collaboration Workflows

Last updated: 2026-02-23

## 1. Why Workflows Matter

Race mode answers: "who produced the best independent attempt?"

Workflows answer: "how do agents cooperate to improve quality and convergence?"

Hydra should support both.

## 2. Workflow Runtime Model

A workflow is a directed acyclic graph (DAG) of nodes.

Node types:
- `agent_task`: run an adapter with prompt + context
- `score_gate`: run scoring and branch based on threshold
- `merge_step`: integrate selected branch/artifact
- `human_gate`: explicit user approval point

Every node must declare:
- inputs (task text, diffs, prior outputs)
- expected artifact outputs
- timeout and retries

## 3. Artifact Passing Contract

Standard artifacts:
- `diff_unified`
- `review_text`
- `score_json`
- `test_failures_json`
- `lint_findings_json`
- `final_branch_ref`

Rules:
1. Artifacts are immutable once published.
2. Nodes consume artifact IDs, not mutable in-memory references.
3. Every artifact is persisted under run namespace.

## 4. Workflow A: Builder -> Reviewer -> Refiner

### 4.1 Goal

Improve correctness and code quality through explicit critique loop.

### 4.2 Sequence

1. Builder agent implements task.
2. Hydra generates unified diff artifact.
3. Reviewer agent critiques diff with strict rubric.
4. Refiner (often same builder) applies reviewer guidance.
5. Score final output.

### 4.3 Reviewer prompt contract

Reviewer prompt should require:
- issue severity (`critical|high|medium|low`)
- file path and line hint when possible
- concrete fix recommendation

### 4.4 Exit criteria

- no unresolved critical issues
- mergeable score threshold reached

## 5. Workflow B: Specialization (Parallel Domain Ownership)

### 5.1 Goal

Split one feature across bounded scopes (e.g., backend/frontend).

### 5.2 Sequence

1. Create shared contract artifact (API schema/types).
2. Launch specialized agent tasks in parallel, each with scope instructions.
3. Merge specialized branches into integration branch.
4. Run integration tests and scoring.

### 5.3 Scope enforcement

Prompt-level scope is soft; git-level validation is hard:
- detect edits outside assigned paths
- flag or block depending on policy

## 6. Workflow C: Iterative Refinement Loop

### 6.1 Goal

Use scoring feedback as structured correction signal until threshold achieved.

### 6.2 Sequence

1. Agent implements task.
2. Score result.
3. If below threshold, synthesize refinement prompt from failures.
4. Repeat until threshold met or max iterations reached.

### 6.3 Convergence guard

Stop early if:
- score decreases twice consecutively
- no net improvement after `N` iterations
- repeated identical failure signatures

## 7. Workflow Composition

Hydra should allow chaining presets.

Example:
1. race
2. builder-reviewer on winning branch
3. iterative refinement if score still below target

## 8. Workflow DSL (Proposed)

```toml
[[workflow.steps]]
id = "race1"
type = "race"
agents = ["claude", "codex"]

[[workflow.steps]]
id = "review1"
type = "builder_reviewer"
input = "race1.winner"
reviewer = "codex"
refiner = "claude"

[[workflow.steps]]
id = "refine1"
type = "iterative"
input = "review1.output"
agent = "claude"
max_iterations = 2
score_threshold = 90
```

Use `cursor-agent` only when experimental adapters are explicitly enabled for the run.

## 9. Policy Controls

Per workflow:
- max parallel agents
- per-node timeout
- retry policy
- required human gates
- max estimated cost

## 10. Human-in-the-Loop Points

Recommended default gates:
1. Before merge to user branch.
2. When conflict resolution is required.
3. When workflow exceeds cost/time budget.

## 11. Failure Handling

| Failure | Handling |
|---|---|
| Agent timeout | mark node failed, branch to fallback node or stop |
| Parse failure for structured output | store raw text artifact, continue with reduced automation |
| Integration conflict | open conflict report artifact, require human gate |
| Score command unavailable | skip affected dimension and renormalize weights |

## 12. Cost and Budget Controls

Workflow-level budget model:
- `max_tokens_total`
- `max_cost_usd`
- `max_runtime_minutes`

If budget exceeded:
- stop non-critical remaining nodes
- preserve artifacts
- emit actionable summary

## 13. UX Requirements

CLI:
- concise step timeline
- live node status updates
- final artifact table with paths

GUI:
- node graph view
- per-node logs
- artifact drilldown
- explicit gate actions (approve/reject/retry)

## 14. Testing Strategy

For each workflow preset:
1. golden-path integration test
2. timeout-path test
3. malformed-output parser resilience test
4. deterministic artifact graph snapshot test

## 15. Open Workflow Questions

1. Should workflow graph editing ship in v1 GUI, or presets only?
2. Should reviewers be restricted to read-only mode where supported?
3. Should specialization support automatic path-revert for out-of-scope edits, or only warnings?

## 16. Related Docs

- `research/architecture.md`
- `research/scoring-engine.md`
- `research/roadmap.md`
