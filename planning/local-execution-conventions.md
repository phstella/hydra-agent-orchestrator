# Local Execution Conventions

Last updated: 2026-02-25

## Purpose

Define how milestones are executed and tracked locally when GitHub tickets are
not part of the workflow.

## Rules

1. Milestone definitions still come from `planning/implementation-checklist.md`.
2. Active execution tracking happens in local packs under `planning/` (example:
   `planning/m4.7-local-execution-pack.md`).
3. Every local task keeps:
   - `ID` (milestone or sub-milestone, e.g., `M4.7.3`)
   - `Status` (`TODO`, `IN_PROGRESS`, `DONE`, `BLOCKED`)
   - `Owner` (or `unassigned`)
   - `Evidence` (tests, screenshots, log files, commit hash)
4. Commit messages should retain milestone prefixes (example:
   `M4.7.3: upgrade leaderboard card model`).
5. `PROGRESS.md` remains the session-level source of truth for completion and
   current priorities.

## Desktop-First Policy (Current Cycle)

For `M4.7` cockpit convergence:
1. Primary viewport target: `>= 1280px`.
2. Minimum supported viewport in scope: `>= 1024px`.
3. Mobile/touch optimization is explicitly out of scope until Phase 5+.

## Local Pack Template

```md
# M<phase>.<milestone> Local Execution Pack

## Completion Gate
- Observable completion criteria.

## Local Tracking Table
| ID | Title | Status | Owner | Evidence |
|---|---|---|---|---|

## Sub-Milestones
### M<phase>.<milestone>.<n> <Title>
- Dependencies:
- Problem:
- Scope:
- Acceptance Criteria:
- Out of Scope:

## Validation Commands
- cargo test --workspace --locked --offline
- cargo clippy --workspace --all-targets --locked --offline -- -D warnings
- npm run lint
- npm run test:smoke
```

