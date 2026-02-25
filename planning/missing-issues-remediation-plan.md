# Missing Issues Remediation Plan

Date: 2026-02-25  
Related commit: `12b6965` (`Harden runtime termination and git ref validation`)

## Already Completed (Committed)

- Critical/High fixes implemented for:
  - `#1` test `panic!` assertions replaced with explicit assertions
  - `#2` parse-related `.unwrap()` in adapter tests replaced with `expect(...)` + context
  - `#3` PTY cleanup termination hardened with retry-based exit checks
  - `#4` process-group signal handling now distinguishes `EPERM` vs `ESRCH`
  - `#6` `agent_key` validation enforced for worktree creation
  - `#7` merge input validation enforced for `agent_key` and branch names

## Remaining Work

### 1) High Item `#5` Validation and Closure

Goal: validate whether the “PTY reader thread panic on I/O error” issue is reproducible and close it correctly.

Steps:
1. Add a targeted PTY test that simulates reader I/O failure.
2. Assert expected behavior:
   - `PtyEvent::Failed` is emitted.
   - session status is stable (`Failed`) and process cleanup still occurs.
3. If panic is non-reproducible (likely current behavior), downgrade/reclassify in tracking docs as covered/non-issue.

Acceptance:
- Deterministic test exists and passes.
- Severity updated with evidence.

Status update (2026-02-25):
- Added deterministic PTY failure-path test:
  - `crates/hydra-core/src/supervisor/pty.rs`
  - test: `pty_reader_io_failure_emits_failed_and_cleans_up`
- Evidence captured:
  - emits `PtyEvent::Failed` on injected reader I/O error
  - final status becomes `PtySessionStatus::Failed`
  - PTY resources are cleaned up (`inner` dropped)

---

### 2) Medium Item `#10` Panic Propagation in Race Tasks

Goal: ensure task panics cannot produce a successful run status.

Target file:
- `crates/hydra-cli/src/race.rs`

Steps:
1. Track `JoinError` panics from spawned agent tasks.
2. Mark run as failed when any task panics.
3. Emit a run-level failure event with panic context.
4. Add test proving a task panic cannot end as overall success.

Acceptance:
- Panic in any task forces `RunStatus::Failed`.

---

### 3) Medium Item `#11` Git Operation Timeouts

Goal: prevent indefinite hangs on git operations.

Target files:
- `crates/hydra-core/src/worktree/mod.rs`
- new shared git helper module in `hydra-core` (if needed)

Steps:
1. Add shared async git exec helper with timeout (default `300s`).
2. Apply helper to worktree git operations (`add`, `list`, `remove`, cleanup paths).
3. Add timeout-path tests (mocked/controlled command path) to verify timeout errors are surfaced.

Acceptance:
- All long-running git paths have bounded timeout behavior.

---

### 4) Medium Item `#12` Merge Report Directory Robustness

Goal: avoid merge report write failures when parent directory is missing.

Target file:
- `crates/hydra-cli/src/merge.rs`

Steps:
1. Before writing `merge_report.json`, call `create_dir_all` for parent directory.
2. Add test covering missing parent dir scenario.

Acceptance:
- Report write succeeds when parent directories are absent.

---

### 5) Medium Items `#8` and `#9` in `hydra-app`

Goals:
- `#8`: prevent potential shutdown freeze from unbounded block-in-place path.
- `#9`: bound interactive single-event payload memory impact.

Target files:
- `crates/hydra-app/src/main.rs`
- `crates/hydra-app/src/state.rs`

Steps:
1. Add bounded timeout around interactive shutdown path in Tauri window destroy handling.
2. Add payload clamp/truncation policy for oversized interactive events.
3. Add tests for:
   - shutdown timeout fallback behavior
   - large-event handling and bounded memory behavior

Acceptance:
- Shutdown path is timeout-bounded.
- Oversized events are safely bounded.

---

## Execution Order

1. High `#5` evidence/test closure
2. Medium `#10` panic propagation
3. Medium `#11` git timeouts
4. Medium `#12` merge report directory creation
5. Medium `#8` + `#9` hydra-app hardening

## Validation Gate (Run After Each Phase)

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. For app-specific changes: run tests from `crates/hydra-app`
