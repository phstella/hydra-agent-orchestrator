# P3-UI-05 and P3-QA-01 Implementation Guide

Last updated: 2026-02-23

## Why this doc exists

`planning/issues/phase-3.md` defines acceptance criteria, but it does not fully define:

1. The diff/merge IPC contract for the Tauri app.
2. How GUI merge actions should map to existing CLI merge semantics.
3. The concrete smoke-test harness and CI wiring for `P3-QA-01`.

This guide fills those gaps so an implementation agent can execute without inventing behavior.

## Current repo reality (confirmed)

Implemented:

- Preflight, adapter selection, race start/poll/result IPC.
- Live output rail/panel.
- Results scoreboard and explicit winner selection.
- CLI merge implementation with `--dry-run`, `--confirm`, `--force`.

Missing for ticket closure:

- No diff review component in `crates/hydra-app/frontend/src/components/`.
- No merge/diff Tauri commands in `crates/hydra-app/src/main.rs`.
- No GUI smoke-test framework/scripts in `crates/hydra-app/frontend/package.json`.
- No guaranteed persisted `diff.patch` artifact generation in race flow.

## Required implementation scope

### 1) Persist agent diff patch artifact (backend prerequisite)

Goal: ensure diff is available even when worktrees/branches are cleaned up.

Implementation target:

- `crates/hydra-cli/src/race.rs`

Add behavior:

- During scoring (before cleanup), write `layout.agent_diff(agent_key)` for each agent.
- Generate patch from worktree against run base ref using git CLI.
- If command succeeds but patch is empty, still write an empty file (valid "no diff").
- If command fails, continue run; include failure evidence in logs/events and leave fallback path for GUI.

Suggested command:

- `git -C <worktree_path> diff --no-color --patch <base_ref>...HEAD`

Rationale:

- `RunLayout` already defines `agents/<agent>/diff.patch`.
- GUI diff view should not depend on branch existence post-cleanup.

### 2) Define and implement Tauri IPC contract for diff review and merge actions

Implementation targets:

- `crates/hydra-app/src/ipc_types.rs`
- `crates/hydra-app/src/commands.rs`
- `crates/hydra-app/src/main.rs`
- `crates/hydra-app/frontend/src/types.ts`
- `crates/hydra-app/frontend/src/ipc.ts`

Add commands:

1. `get_candidate_diff(run_id, agent_key) -> CandidateDiffPayload`
2. `preview_merge(run_id, agent_key, force) -> MergePreviewPayload`
3. `execute_merge(run_id, agent_key, force) -> MergeExecutionPayload`

Required semantics:

- `preview_merge` must map to CLI `hydra merge --run-id <id> --agent <key> --dry-run --json [--force]`.
- `execute_merge` must map to CLI `hydra merge --run-id <id> --agent <key> --confirm --json [--force]`.
- Non-mergeable candidates must be blocked by default unless explicit `force=true`.
- Error responses must preserve actionable CLI message text for UI display.

Command invocation strategy:

- Reuse the same binary resolution pattern already used by race command:
  - `HYDRA_CLI_BIN`, then `hydra`, then `cargo run -p hydra-cli --`.

Suggested payloads (camelCase):

- `CandidateDiffPayload`
  - `runId: string`
  - `agentKey: string`
  - `baseRef: string`
  - `branch: string | null`
  - `mergeable: boolean | null`
  - `gateFailures: string[]`
  - `diffText: string`
  - `files: DiffFile[]`
  - `diffAvailable: boolean`
  - `source: "artifact" | "git" | "none"`
  - `warning: string | null`

- `DiffFile`
  - `path: string`
  - `added: number`
  - `removed: number`

- `MergePreviewPayload`
  - `agentKey: string`
  - `branch: string`
  - `success: boolean`
  - `hasConflicts: boolean`
  - `stdout: string`
  - `stderr: string`
  - `reportPath: string | null`

- `MergeExecutionPayload`
  - `agentKey: string`
  - `branch: string`
  - `success: boolean`
  - `message: string`
  - `stdout: string | null`
  - `stderr: string | null`

Diff source resolution order:

1. `layout.agent_diff(agent_key)` if present.
2. If missing and branch still exists, generate live diff via git.
3. If neither available, return `diffAvailable=false` with warning text.

### 3) Build P3-UI-05 UI surface from mockup #3

Implementation targets:

- `crates/hydra-app/frontend/src/App.tsx`
- `crates/hydra-app/frontend/src/components/` (new files)
- Existing design-system primitives only; no hardcoded hex values.

Add view structure:

1. Candidate tab bar (one tab per candidate).
2. Side-by-side or split diff area (virtualized or tail-window safeguards on large text).
3. Modified-file list panel.
4. Merge action rail:
   - Status badge (`mergeable`, `gated`, `conflict`, `merged`).
   - `Preview Merge` action (dry-run).
   - `Accept Candidate` action with explicit confirmation step.
   - `Reject` action (explicit UI action, no implicit merge).

State requirements:

- Winner selection from scoreboard must feed default selected candidate in review view.
- Switching candidates refreshes diff and mergeability context.
- Non-mergeable candidates disable accept unless user toggles force override.
- Fallback state shown when `diffAvailable=false`.

Suggested components:

- `CandidateDiffReview.tsx`
- `CandidateTabs.tsx`
- `DiffViewerPane.tsx`
- `MergeActionRail.tsx`

### 4) Add P3-QA-01 smoke test pack

Implementation targets:

- `crates/hydra-app/frontend/package.json`
- `crates/hydra-app/frontend/` test config files
- `crates/hydra-app/frontend/src/**/__tests__/*`
- `.github/workflows/ci.yml`

Test framework baseline:

- Vitest + Testing Library (`jsdom`) for deterministic UI flow tests with mocked IPC.

Minimum smoke scenarios:

1. App startup renders tabs and preflight screen.
2. Preflight refresh action triggers IPC call and updates state.
3. Experimental adapter modal blocks confirm until acknowledgment checked.
4. Race flow transitions: start race -> live output visible -> result scoreboard visible.
5. Winner selection stays explicit and does not auto-merge.
6. Diff review candidate switching updates displayed diff/file list.
7. Merge rail dry-run path blocks/alerts on conflict and allows clean preview.

CI requirements:

- Add frontend smoke test command (`npm run test:smoke`) and run it on Linux + Windows.
- Keep existing lint/build checks.

## Ticket-to-deliverable mapping

P3-UI-05 acceptance criteria mapping:

- Candidate switching: candidate tabs + diff refresh logic.
- Large diff usability: virtualization/tail-window safeguards.
- Diff unavailable fallback: `diffAvailable=false` UI state.
- Merge rail reflects gate status: badge + disabled/force paths.
- Explicit accept/reject with confirmation aligned to CLI merge semantics.

P3-QA-01 acceptance criteria mapping:

- Startup/preflight/modal/race/results/diff/merge gating covered by smoke tests.
- Tests executed in CI matrix (Linux + Windows).

## Definition of done for this implementation

1. New Tauri commands compile and are registered in `main.rs`.
2. Diff review UI is reachable from current app flow and usable with real run artifacts.
3. Merge preview and execution paths work against existing CLI merge behavior.
4. Smoke tests pass locally and in CI on Linux + Windows.
5. `PROGRESS.md` updated to reflect completion of `P3-UI-05` and `P3-QA-01` when done.
