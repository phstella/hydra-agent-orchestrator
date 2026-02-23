# Collaboration Workflows

## Overview

Racing agents against each other answers "which agent gives the best first-attempt answer?" Collaboration workflows answer a harder question: "what happens when agents work together?" Hydra supports three collaboration patterns, each modeling a real-world software engineering workflow.

---

## Workflow 1: Builder / Reviewer

### Concept

One agent writes the code. A second agent reviews the diff for bugs, security issues, and quality problems. Optionally, the builder applies the reviewer's suggestions in a third pass.

This mirrors how human teams work: a developer writes a PR, a reviewer catches issues, the developer addresses feedback.

### Sequence

```
Phase 1 — Build
    User provides task prompt → Builder agent runs in worktree-A
    Builder completes → Hydra captures the diff

Phase 2 — Review
    Hydra generates a review prompt:
        "Review this diff for bugs, security vulnerabilities,
         performance issues, and code quality. List specific
         issues with file paths and line numbers."
    + appends the full diff as context
    Reviewer agent runs in worktree-B (or stateless, reviewing only)
    Reviewer completes → Hydra captures the review as structured feedback

Phase 3 — Refinement (optional)
    Hydra generates a refinement prompt:
        "Apply the following review feedback to your implementation:"
    + appends the reviewer's feedback
    Builder agent runs again in worktree-A (session continuation if supported)
    Builder completes → Hydra runs the scoring engine on the final output
```

### Configuration

```toml
[collaboration.builder_reviewer]
builder_agent = "claude"
reviewer_agent = "codex"
max_refinement_rounds = 2
review_prompt_template = """
Review this diff for bugs, security vulnerabilities, performance issues, and code quality.
Be specific: reference file paths and line numbers.

Diff:
{diff}
"""
refinement_prompt_template = """
Apply the following review feedback to your implementation. Make only the changes
the reviewer requested — do not refactor unrelated code.

Review feedback:
{review}
"""
```

### Implementation Details

**Worktree management:** The builder runs in its own worktree. The reviewer can run in a separate worktree (if it needs to test changes) or statelessly (if it only analyzes the diff text). For stateless review, the reviewer doesn't need a worktree — the diff is passed as part of the prompt.

**Diff capture:** After the builder completes, Hydra runs `git diff <base>..<builder-branch>` and stores the output. This diff string becomes the primary input for the reviewer.

**Review parsing:** The reviewer's output is treated as free-form text. Hydra does not attempt to parse it into structured feedback — the raw text is passed to the builder as-is. This avoids brittle parsing and lets each agent express feedback in its natural format.

**Session continuation:** If the builder agent supports session continuation (Claude Code does via `--session-id`), the refinement phase reuses the same session so the agent has full context. If not, the full original task prompt + diff + review feedback are passed together.

**Scoring:** The scoring engine runs only on the final output (after refinement), not on intermediate states.

### When to Use

- Tasks where correctness matters more than speed (authentication, payment logic, data migration)
- When the user suspects a single agent might miss edge cases
- When the builder and reviewer are different models with complementary strengths (e.g., Claude for architecture, Codex for security patterns)

---

## Workflow 2: Specialization

### Concept

Different agents work on different parts of the codebase simultaneously, each scoped to a specific directory or set of files. A shared API contract or interface definition is passed to both agents as context so their outputs are compatible.

This mirrors how human teams split work: one developer handles the backend API, another handles the frontend UI, both agree on the API contract.

### Sequence

```
Phase 1 — Contract Definition
    User provides:
        - The overall task
        - A shared context document (API schema, interface definition, types file)
        - Agent-to-directory assignments

Phase 2 — Parallel Execution
    Agent A runs in worktree-A, scoped to backend/
        Prompt: "{task}\n\nYou are responsible for the backend implementation.
                 Use this API contract: {contract}\n
                 Only modify files in the backend/ directory."

    Agent B runs in worktree-B, scoped to frontend/
        Prompt: "{task}\n\nYou are responsible for the frontend implementation.
                 Use this API contract: {contract}\n
                 Only modify files in the frontend/ directory."

    Both run concurrently.

Phase 3 — Integration
    Hydra merges both agents' branches into an integration branch
    Runs the scoring engine (build + tests) on the integrated result
    Reports conflicts or test failures
```

### Configuration

```toml
[collaboration.specialization]
task = "Add user profile editing with avatar upload"

[[collaboration.specialization.agents]]
name = "claude"
scope = "backend/"
prompt_suffix = "You are responsible for the backend API. Implement REST endpoints."

[[collaboration.specialization.agents]]
name = "codex"
scope = "frontend/"
prompt_suffix = "You are responsible for the React frontend. Build the UI components."

[collaboration.specialization.shared_context]
files = ["shared/types.ts", "docs/api-spec.yaml"]
inline = """
API Contract:
POST /api/profile - Update user profile (name, bio)
POST /api/profile/avatar - Upload avatar image (multipart/form-data)
GET /api/profile/:id - Get user profile
"""
```

### Implementation Details

**Scope enforcement:** Hydra cannot truly restrict which files an agent modifies (CLI agents don't support directory scoping). Instead, scope is enforced via the prompt ("Only modify files in backend/") and verified post-completion. If an agent modified files outside its scope, Hydra flags this in the results and can optionally revert out-of-scope changes using `git checkout` on those files.

**Shared context:** The `shared_context` block specifies files and/or inline text that gets appended to every agent's prompt. This ensures all agents work against the same interface.

**Integration merge:** After both agents complete, Hydra creates an integration branch and cherry-picks or merges both agents' changes:
1. Create branch `hydra/integration-<timestamp>` from the base
2. Merge agent A's branch — this should apply cleanly
3. Merge agent B's branch — this may produce conflicts if scope was violated
4. If conflicts exist, report them and let the user resolve (or feed to a third agent)

**Cross-agent conflicts:** If both agents modified the same file (despite scoping instructions), Hydra reports the conflict with the specific files and lets the user choose how to resolve: manual merge, re-run one agent with additional context, or feed the conflict to a "resolver" agent.

### When to Use

- Full-stack features where frontend and backend can be developed in parallel
- Monorepo projects where different modules are largely independent
- Tasks with clear domain boundaries (e.g., database migration + application code)

---

## Workflow 3: Iterative Refinement

### Concept

A single agent runs, the scoring engine evaluates the result, and if the score is below a threshold, the scoring feedback is fed back to the agent as a follow-up prompt. The loop repeats until the score meets the threshold or the maximum iteration count is reached.

This mirrors test-driven development: write code, run tests, fix failures, repeat.

### Sequence

```
Iteration 1:
    Agent runs with original task prompt
    Scoring engine evaluates → score = 62 (below threshold of 85)
    Failures captured:
        - Build: PASS
        - Tests: 3 failing (test_auth_expired, test_auth_invalid, test_auth_refresh)
        - Lint: 4 new warnings

Iteration 2:
    Agent runs with refinement prompt:
        "Your previous implementation scored 62/100. Fix these issues:

         Failing tests:
         - test_auth_expired: expected 401, got 200
         - test_auth_invalid: expected error message, got null
         - test_auth_refresh: timeout after 5000ms

         Lint warnings:
         - src/auth.rs:45: unused variable 'token_data'
         - src/auth.rs:78: unnecessary clone
         - src/auth.rs:112: missing error handling
         - src/auth.rs:130: implicit return

         Do not change anything that currently passes tests."

    Scoring engine evaluates → score = 91 (above threshold) → DONE
```

### Configuration

```toml
[collaboration.iterative]
agent = "claude"
max_iterations = 3
score_threshold = 85.0
refinement_prompt_template = """
Your previous implementation scored {score}/100. Fix these issues:

{failures}

Do not change anything that currently works. Focus only on the failures above.
"""
```

### Implementation Details

**Worktree reuse:** The agent runs in the same worktree across iterations. Each iteration builds on the previous state, so the agent is refining its own work, not starting from scratch.

**Session continuation:** If the agent supports session continuation (Claude Code's `--session-id`), use it so the agent retains full context from previous iterations. If not, the prompt includes the original task + all accumulated feedback.

**Failure extraction:** The scoring engine produces structured failure data:
- Build errors: captured from stderr of the build command
- Test failures: extracted test names and assertion messages from test runner output
- Lint issues: file paths, line numbers, and messages from linter output

This structured data is formatted into the refinement prompt template.

**Convergence guard:** If the score does not improve between iterations (or decreases), Hydra stops early to prevent infinite loops where the agent oscillates between fixes. Specifically:

```rust
if current_score <= previous_score {
    // Agent is not converging — stop and report
    return IterativeResult::Stalled {
        final_score: current_score,
        iterations_completed: i,
    };
}
```

**Cost tracking:** Each iteration costs tokens. Hydra tracks cumulative token usage across all iterations and reports the total, so the user can evaluate whether the refinement was worth the cost.

### When to Use

- Tasks where getting to "all tests pass" matters and first attempts often have edge-case failures
- When running a single strong agent is preferred over racing multiple agents
- Prototyping or exploratory coding where the agent might need guidance on project conventions

---

## Workflow Comparison

| Attribute | Builder/Reviewer | Specialization | Iterative Refinement |
|---|---|---|---|
| Agents required | 2 (different roles) | 2+ (parallel, same role) | 1 |
| Parallelism | Sequential (build → review → refine) | Parallel (agents run concurrently) | Sequential (run → score → run) |
| Best for | Correctness-critical code | Full-stack / multi-domain tasks | Test-driven convergence |
| Cost | 2-3x single agent | 2x+ single agent (parallel) | 1-3x single agent (iterations) |
| Time | Longest (sequential phases) | Shortest (parallel work) | Medium |
| Worktrees needed | 1-2 | N (one per agent) | 1 |

---

## Combining Workflows

Workflows can be composed. Examples:

**Race + Review:** Race three agents, then feed the winner's diff through a Builder/Reviewer pass with a fourth agent as reviewer.

**Specialization + Iterative:** Run two agents in specialization mode, then apply iterative refinement on the integrated result if the integration score is below threshold.

**Race + Iterative:** Race three agents, pick the winner, then iteratively refine the winner's output until it passes all tests.

These compositions are configured as multi-step workflow definitions:

```toml
[[workflow.steps]]
type = "race"
agents = ["claude", "codex", "cursor"]
task = "{task}"

[[workflow.steps]]
type = "iterative"
agent = "claude"
input = "winner_of_previous"
max_iterations = 2
score_threshold = 90.0
```

The `TaskRouter` processes steps sequentially, passing outputs between them.
