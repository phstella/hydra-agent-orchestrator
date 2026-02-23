use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::State;
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};

use hydra_core::artifact::RunEvent;

use crate::ipc_types::*;
use crate::state::{AppState, AppStateHandle};

const MAX_EVENTS_PER_POLL: usize = 512;

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn health_check() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ---------------------------------------------------------------------------
// Preflight / Doctor
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn run_preflight(state: State<'_, AppState>) -> Result<PreflightResult, String> {
    let report = state.run_probes().await;

    let adapters: Vec<AdapterInfo> = report.results.iter().map(AdapterInfo::from).collect();

    let mut checks = Vec::new();
    let mut warnings = Vec::new();

    // Check: Git repository
    let git_repo_ok = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    checks.push(DiagnosticCheck {
        name: "Git Repository".to_string(),
        description: if git_repo_ok {
            "Working inside a valid git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
        status: if git_repo_ok {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        evidence: None,
    });

    // Check: Environment variables
    let has_env = std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok();
    checks.push(DiagnosticCheck {
        name: "Environment Variables Check".to_string(),
        description: "Found system configuration".to_string(),
        status: if has_env {
            CheckStatus::Passed
        } else {
            CheckStatus::Warning
        },
        evidence: None,
    });

    // Check: Adapter validation
    let tier1_count = adapters
        .iter()
        .filter(|a| a.tier == hydra_core::adapter::AdapterTier::Tier1)
        .count();
    let tier1_ready = adapters
        .iter()
        .filter(|a| a.tier == hydra_core::adapter::AdapterTier::Tier1 && a.status.is_available())
        .count();

    checks.push(DiagnosticCheck {
        name: "Validating Adapters".to_string(),
        description: format!("{}/{} tier-1 adapters ready", tier1_ready, tier1_count),
        status: if tier1_ready == tier1_count {
            CheckStatus::Passed
        } else if tier1_ready > 0 {
            CheckStatus::Warning
        } else {
            CheckStatus::Failed
        },
        evidence: Some(format!(
            "Connected to {} adapter(s)",
            adapters.iter().filter(|a| a.status.is_available()).count()
        )),
    });

    // Check: working tree cleanliness
    let git_status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output();
    let (git_status_ok, worktree_clean) = match git_status {
        Ok(output) if output.status.success() => (true, output.stdout.is_empty()),
        _ => (false, false),
    };

    checks.push(DiagnosticCheck {
        name: "Working Tree Cleanliness".to_string(),
        description: if !git_status_ok {
            "Unable to inspect working tree status".to_string()
        } else if worktree_clean {
            "Working tree is clean".to_string()
        } else {
            "Working tree has uncommitted changes".to_string()
        },
        status: if !git_status_ok {
            CheckStatus::Warning
        } else if worktree_clean {
            CheckStatus::Passed
        } else {
            CheckStatus::Warning
        },
        evidence: None,
    });

    // Warnings for experimental adapters
    for adapter in &adapters {
        if adapter.tier == hydra_core::adapter::AdapterTier::Experimental
            && adapter.status.is_available()
        {
            warnings.push(format!(
                "{} adapter is experimental. Inference might be slow during race simulation.",
                adapter.key
            ));
        }
    }

    let passed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Passed)
        .count() as u32;
    let failed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Failed)
        .count() as u32;
    let total = checks.len() as u32;
    let health_score = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    Ok(PreflightResult {
        system_ready: failed == 0 && report.all_tier1_ready,
        all_tier1_ready: report.all_tier1_ready,
        passed_count: passed,
        failed_count: failed,
        total_count: total,
        health_score,
        checks,
        adapters,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// List adapters (runtime-driven, not hardcoded)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_adapters(state: State<'_, AppState>) -> Result<Vec<AdapterInfo>, String> {
    let report = state.run_probes().await;
    Ok(report.results.iter().map(AdapterInfo::from).collect())
}

// ---------------------------------------------------------------------------
// Race commands (M3.2)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_race(
    state: State<'_, AppState>,
    request: RaceRequest,
) -> Result<RaceStarted, String> {
    if request.task_prompt.trim().is_empty() {
        return Err(IpcError::validation("Task prompt cannot be empty").to_string());
    }
    if request.agents.is_empty() {
        return Err(IpcError::validation("At least one agent must be selected").to_string());
    }

    let run_id = uuid::Uuid::new_v4().to_string();
    let agents = request.agents.clone();
    let state_handle = state.handle();
    state_handle.register_race(&run_id).await;

    let run_id_for_task = run_id.clone();
    tokio::spawn(async move {
        execute_race(state_handle, request, run_id_for_task).await;
    });

    Ok(RaceStarted { run_id, agents })
}

#[tauri::command]
pub async fn poll_race_events(
    state: State<'_, AppState>,
    run_id: String,
    cursor: u64,
) -> Result<RaceEventBatch, String> {
    let cursor = usize::try_from(cursor)
        .map_err(|_| IpcError::validation("Invalid event cursor").to_string())?;

    let state_handle = state.handle();
    let Some((events, next_cursor, done, status, error)) = state_handle
        .poll_events(&run_id, cursor, MAX_EVENTS_PER_POLL)
        .await
    else {
        return Err(IpcError::validation("Unknown run ID").to_string());
    };

    Ok(RaceEventBatch {
        run_id,
        events,
        next_cursor: next_cursor as u64,
        done,
        status,
        error,
    })
}

#[tauri::command]
pub async fn get_race_result(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Option<RaceResult>, String> {
    let state_handle = state.handle();
    Ok(state_handle.race_result(&run_id).await)
}

async fn execute_race(state: AppStateHandle, request: RaceRequest, run_id: String) {
    let repo_root = match discover_repo_root() {
        Ok(path) => path,
        Err(err) => {
            state.mark_failed(&run_id, err.message).await;
            return;
        }
    };

    let events_path = repo_root
        .join(".hydra")
        .join("runs")
        .join(&run_id)
        .join("events.jsonl");

    let mut cmd = build_race_command(&repo_root, &run_id, &request);
    let stop_tail = Arc::new(AtomicBool::new(false));
    let tail_handle = tokio::spawn(tail_run_events_file(
        state.clone(),
        run_id.clone(),
        events_path,
        Arc::clone(&stop_tail),
    ));

    emit_orchestrator_event(
        &state,
        &run_id,
        "race_process_started",
        serde_json::json!({ "agents": request.agents }),
    )
    .await;

    let output = cmd.output().await;

    stop_tail.store(true, Ordering::Relaxed);
    let _ = tail_handle.await;

    match output {
        Ok(output) if output.status.success() => match parse_cli_race_summary(&output.stdout) {
            Ok(result) => {
                state.mark_completed(&run_id, result).await;
                emit_orchestrator_event(
                    &state,
                    &run_id,
                    "race_process_completed",
                    serde_json::json!({}),
                )
                .await;
            }
            Err(err) => {
                state.mark_failed(&run_id, err.message.clone()).await;
                emit_orchestrator_event(
                    &state,
                    &run_id,
                    "race_process_failed",
                    serde_json::json!({ "error": err.message }),
                )
                .await;
            }
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!("race command exited with status {}", output.status)
            } else {
                format!("race command failed: {}", stderr)
            };
            state.mark_failed(&run_id, message.clone()).await;
            emit_orchestrator_event(
                &state,
                &run_id,
                "race_process_failed",
                serde_json::json!({ "error": message }),
            )
            .await;
        }
        Err(err) => {
            let message = format!("failed to execute race command: {err}");
            state.mark_failed(&run_id, message.clone()).await;
            emit_orchestrator_event(
                &state,
                &run_id,
                "race_process_failed",
                serde_json::json!({ "error": message }),
            )
            .await;
        }
    }
}

fn parse_cli_race_summary(stdout: &[u8]) -> Result<RaceResult, IpcError> {
    let json: serde_json::Value = serde_json::from_slice(stdout)
        .map_err(|e| IpcError::internal(format!("failed to parse race JSON output: {e}")))?;

    let run_id = json
        .get("run_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IpcError::internal("race JSON output missing run_id"))?
        .to_string();

    let status = json
        .get("status")
        .and_then(|v| v.as_str())
        .map(normalize_status)
        .unwrap_or_else(|| "unknown".to_string());

    let race_duration_ms = json
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            json.get("health")
                .and_then(|h| h.get("duration_ms"))
                .and_then(|v| v.as_u64())
        });

    let total_cost = json
        .get("total_cost")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            json.get("cost")
                .and_then(|c| c.get("estimated_cost_usd"))
                .and_then(|v| v.as_f64())
        });

    let agents = json
        .get("agents")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let score_obj = item.get("score");

                    let dimensions = score_obj
                        .and_then(|s| s.get("dimensions"))
                        .and_then(|d| d.as_array())
                        .map(|dims| {
                            dims.iter()
                                .filter_map(|dim| {
                                    let name = dim.get("name")?.as_str()?.to_string();
                                    let score = dim.get("score")?.as_f64()?;
                                    let evidence = dim
                                        .get("evidence")
                                        .cloned()
                                        .unwrap_or(serde_json::json!({}));
                                    Some(DimensionScoreIpc { name, score, evidence })
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let gate_failures = score_obj
                        .and_then(|s| s.get("gate_failures"))
                        .and_then(|g| g.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    AgentResult {
                        agent_key: item
                            .get("agent")
                            .or_else(|| item.get("agent_key"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        status: item
                            .get("status")
                            .and_then(|v| v.as_str())
                            .map(normalize_status)
                            .unwrap_or_else(|| "unknown".to_string()),
                        duration_ms: item.get("duration_ms").and_then(|v| v.as_u64()),
                        score: score_obj
                            .and_then(|v| v.get("composite"))
                            .and_then(|v| v.as_f64()),
                        mergeable: score_obj
                            .and_then(|v| v.get("mergeable"))
                            .and_then(|v| v.as_bool()),
                        gate_failures,
                        dimensions,
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(RaceResult {
        run_id,
        status,
        agents,
        duration_ms: race_duration_ms,
        total_cost,
    })
}

fn normalize_status(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }

    let mut out = String::with_capacity(trimmed.len() + 4);
    let mut prev_was_lower_or_digit = false;

    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && prev_was_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_was_lower_or_digit = false;
        }
    }

    while out.ends_with('_') {
        out.pop();
    }

    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn build_race_command(repo_root: &Path, run_id: &str, request: &RaceRequest) -> TokioCommand {
    let mut cmd = if let Ok(bin) = std::env::var("HYDRA_CLI_BIN") {
        TokioCommand::new(bin)
    } else if binary_available("hydra") {
        TokioCommand::new("hydra")
    } else {
        let mut cargo = TokioCommand::new("cargo");
        cargo.args(["run", "-p", "hydra-cli", "--"]);
        cargo
    };

    let mut args = vec![
        "race".to_string(),
        "--json".to_string(),
        "--prompt".to_string(),
        request.task_prompt.clone(),
        "--base-ref".to_string(),
        "HEAD".to_string(),
        "--agents".to_string(),
        request.agents.join(","),
        "--run-id".to_string(),
        run_id.to_string(),
    ];
    if request.allow_experimental {
        args.push("--allow-experimental-adapters".to_string());
    }

    cmd.args(args);
    cmd.current_dir(repo_root);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd
}

fn binary_available(program: &str) -> bool {
    std::process::Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn discover_repo_root() -> Result<PathBuf, IpcError> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| IpcError::internal(format!("failed to execute git: {e}")))?;

    if !output.status.success() {
        return Err(IpcError::validation(
            "Not inside a git repository; cannot start race",
        ));
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return Err(IpcError::internal("git returned empty repository root"));
    }

    Ok(PathBuf::from(root))
}

async fn tail_run_events_file(
    state: AppStateHandle,
    run_id: String,
    events_path: PathBuf,
    stop: Arc<AtomicBool>,
) {
    let mut consumed_lines = 0usize;

    loop {
        consumed_lines =
            emit_new_events_from_file(&state, &run_id, &events_path, consumed_lines).await;

        if stop.load(Ordering::Relaxed) {
            break;
        }

        sleep(Duration::from_millis(120)).await;
    }

    let _ = emit_new_events_from_file(&state, &run_id, &events_path, consumed_lines).await;
}

async fn emit_new_events_from_file(
    state: &AppStateHandle,
    run_id: &str,
    events_path: &Path,
    consumed_lines: usize,
) -> usize {
    let Ok(content) = tokio::fs::read_to_string(events_path).await else {
        return consumed_lines;
    };

    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= consumed_lines {
        return lines.len();
    }

    for line in lines.iter().skip(consumed_lines) {
        if let Some(event) = parse_run_event_line(run_id, line) {
            state.append_event(run_id, event).await;
        }
    }

    lines.len()
}

fn parse_run_event_line(run_id: &str, line: &str) -> Option<AgentStreamEvent> {
    if line.trim().is_empty() {
        return None;
    }

    let event: RunEvent = serde_json::from_str(line).ok()?;
    let event_type = serde_json::to_value(&event.kind)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string());

    Some(AgentStreamEvent {
        run_id: run_id.to_string(),
        agent_key: event.agent_key.unwrap_or_else(|| "system".to_string()),
        event_type,
        data: event.data,
        timestamp: event.timestamp.to_rfc3339(),
    })
}

async fn emit_orchestrator_event(
    state: &AppStateHandle,
    run_id: &str,
    event_type: &str,
    data: serde_json::Value,
) {
    state
        .append_event(
            run_id,
            AgentStreamEvent {
                run_id: run_id.to_string(),
                agent_key: "system".to_string(),
                event_type: event_type.to_string(),
                data,
                timestamp: format!(
                    "{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                ),
            },
        )
        .await;
}

// ---------------------------------------------------------------------------
// Diff review commands (P3-UI-05)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_candidate_diff(
    run_id: String,
    agent_key: String,
) -> Result<CandidateDiffPayload, String> {
    let repo_root =
        discover_repo_root().map_err(|e| format!("[internal_error] {}", e.message))?;
    let hydra_root = repo_root.join(".hydra");
    let run_uuid = uuid::Uuid::parse_str(&run_id)
        .map_err(|e| format!("[validation_error] invalid run_id: {e}"))?;
    let layout = hydra_core::artifact::RunLayout::new(&hydra_root, run_uuid);

    if !layout.base_dir().exists() {
        return Err(format!(
            "[validation_error] run {} not found",
            run_id
        ));
    }

    let manifest =
        hydra_core::artifact::RunManifest::read_from(&layout.manifest_path())
            .map_err(|e| format!("[internal_error] failed to read manifest: {e}"))?;

    let entry = manifest
        .agents
        .iter()
        .find(|a| a.agent_key == agent_key)
        .ok_or_else(|| {
            format!(
                "[validation_error] agent '{}' not found in run {}",
                agent_key, run_id
            )
        })?;

    let base_ref = manifest.base_ref.clone();
    let branch = Some(entry.branch.clone());

    let (mergeable, gate_failures) = load_agent_mergeability(&layout, &agent_key);

    let diff_artifact = layout.agent_diff(&agent_key);
    if diff_artifact.exists() {
        let diff_text = std::fs::read_to_string(&diff_artifact)
            .map_err(|e| format!("[internal_error] failed to read diff artifact: {e}"))?;
        let files = parse_diff_numstat_from_patch(&diff_text);
        return Ok(CandidateDiffPayload {
            run_id,
            agent_key,
            base_ref,
            branch,
            mergeable,
            gate_failures,
            diff_text: diff_text.clone(),
            files,
            diff_available: true,
            source: "artifact".to_string(),
            warning: None,
        });
    }

    if let Some(branch_name) = &branch {
        if branch_exists(&repo_root, branch_name) {
            match generate_live_diff(&repo_root, branch_name, &base_ref) {
                Ok(diff_text) => {
                    let files = parse_diff_numstat_from_patch(&diff_text);
                    return Ok(CandidateDiffPayload {
                        run_id,
                        agent_key,
                        base_ref,
                        branch,
                        mergeable,
                        gate_failures,
                        diff_text,
                        files,
                        diff_available: true,
                        source: "git".to_string(),
                        warning: Some(
                            "Diff generated live from branch (artifact was not persisted)"
                                .to_string(),
                        ),
                    });
                }
                Err(e) => {
                    tracing::warn!(error = %e, "live diff generation failed");
                }
            }
        }
    }

    Ok(CandidateDiffPayload {
        run_id,
        agent_key,
        base_ref,
        branch,
        mergeable,
        gate_failures,
        diff_text: String::new(),
        files: Vec::new(),
        diff_available: false,
        source: "none".to_string(),
        warning: Some(
            "Diff unavailable: artifact not persisted and branch no longer exists".to_string(),
        ),
    })
}

#[tauri::command]
pub async fn preview_merge(
    run_id: String,
    agent_key: String,
    force: bool,
) -> Result<MergePreviewPayload, String> {
    let (cli_parts, repo_root) = resolve_cli_and_repo()?;

    let mut args: Vec<String> = cli_parts[1..].to_vec();
    args.extend([
        "merge".to_string(),
        "--run-id".to_string(),
        run_id,
        "--agent".to_string(),
        agent_key.clone(),
        "--dry-run".to_string(),
        "--json".to_string(),
    ]);
    if force {
        args.push("--force".to_string());
    }

    let output = std::process::Command::new(&cli_parts[0])
        .args(&args)
        .current_dir(&repo_root)
        .output()
        .map_err(|e| format!("[internal_error] failed to execute merge command: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    parse_merge_preview_payload(output.status.success(), &agent_key, &stdout, &stderr)
}

#[tauri::command]
pub async fn execute_merge(
    run_id: String,
    agent_key: String,
    force: bool,
) -> Result<MergeExecutionPayload, String> {
    let (cli_parts, repo_root) = resolve_cli_and_repo()?;

    let mut args: Vec<String> = cli_parts[1..].to_vec();
    args.extend([
        "merge".to_string(),
        "--run-id".to_string(),
        run_id,
        "--agent".to_string(),
        agent_key.clone(),
        "--confirm".to_string(),
        "--json".to_string(),
    ]);
    if force {
        args.push("--force".to_string());
    }

    let output = std::process::Command::new(&cli_parts[0])
        .args(&args)
        .current_dir(&repo_root)
        .output()
        .map_err(|e| format!("[internal_error] failed to execute merge command: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
        let branch_val = parsed
            .get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let message = parsed
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Merge completed successfully")
            .to_string();
        Ok(MergeExecutionPayload {
            agent_key,
            branch: branch_val,
            success: true,
            message,
            stdout: Some(stdout),
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    } else {
        Ok(MergeExecutionPayload {
            agent_key,
            branch: String::new(),
            success: false,
            message: if stderr.is_empty() {
                format!("merge exited with status {}", output.status)
            } else {
                stderr.trim().to_string()
            },
            stdout: if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            },
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    }
}

fn resolve_cli_and_repo() -> Result<(Vec<String>, PathBuf), String> {
    let repo_root =
        discover_repo_root().map_err(|e| format!("[internal_error] {}", e.message))?;

    if let Ok(bin) = std::env::var("HYDRA_CLI_BIN") {
        Ok((vec![bin], repo_root))
    } else if binary_available("hydra") {
        Ok((vec!["hydra".to_string()], repo_root))
    } else {
        Ok((
            vec![
                "cargo".to_string(),
                "run".to_string(),
                "-p".to_string(),
                "hydra-cli".to_string(),
                "--".to_string(),
            ],
            repo_root,
        ))
    }
}

fn parse_merge_preview_payload(
    status_success: bool,
    agent_key: &str,
    stdout: &str,
    stderr: &str,
) -> Result<MergePreviewPayload, String> {
    if let Some(payload) = try_parse_merge_preview_payload(agent_key, stdout, stderr) {
        if status_success || payload.has_conflicts {
            return Ok(payload);
        }

        return Err(format!(
            "[validation_error] {}",
            best_merge_error_message(stderr, stdout)
        ));
    }

    if status_success {
        return Err(
            "[internal_error] merge preview did not return expected JSON payload".to_string()
        );
    }

    Err(format!(
        "[validation_error] {}",
        best_merge_error_message(stderr, stdout)
    ))
}

fn try_parse_merge_preview_payload(
    agent_key: &str,
    stdout: &str,
    stderr: &str,
) -> Option<MergePreviewPayload> {
    let parsed: serde_json::Value = serde_json::from_str(stdout).ok()?;
    let has_conflicts = parsed
        .get("has_conflicts")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let success = parsed
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(!has_conflicts);

    Some(MergePreviewPayload {
        agent_key: parsed
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or(agent_key)
            .to_string(),
        branch: parsed
            .get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        success,
        has_conflicts,
        stdout: parsed
            .get("stdout")
            .and_then(|v| v.as_str())
            .unwrap_or(stdout)
            .to_string(),
        stderr: parsed
            .get("stderr")
            .and_then(|v| v.as_str())
            .unwrap_or(stderr)
            .to_string(),
        report_path: parsed
            .get("report_path")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

fn best_merge_error_message(stderr: &str, stdout: &str) -> String {
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        return stderr.to_string();
    }

    let stdout = stdout.trim();
    if !stdout.is_empty() {
        return stdout.to_string();
    }

    "merge preview command failed".to_string()
}

fn load_agent_mergeability(
    layout: &hydra_core::artifact::RunLayout,
    agent_key: &str,
) -> (Option<bool>, Vec<String>) {
    let score_path = layout.agent_score(agent_key);
    if !score_path.exists() {
        return (None, Vec::new());
    }
    let Ok(data) = std::fs::read_to_string(&score_path) else {
        return (None, Vec::new());
    };
    let Ok(score) = serde_json::from_str::<serde_json::Value>(&data) else {
        return (None, Vec::new());
    };
    let mergeable = score.get("mergeable").and_then(|v| v.as_bool());
    let gate_failures = score
        .get("gate_failures")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    (mergeable, gate_failures)
}

fn parse_diff_numstat_from_patch(patch: &str) -> Vec<DiffFile> {
    let mut files: Vec<DiffFile> = Vec::new();
    for line in patch.lines() {
        if let Some(path) = line.strip_prefix("diff --git a/") {
            if let Some(bpath) = path.split(" b/").nth(1) {
                if !files.iter().any(|f| f.path == bpath) {
                    files.push(DiffFile {
                        path: bpath.to_string(),
                        added: 0,
                        removed: 0,
                    });
                }
            }
        }
        if line.starts_with("@@") {
            continue;
        }
        if let Some(last) = files.last_mut() {
            if line.starts_with('+') && !line.starts_with("+++") {
                last.added += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                last.removed += 1;
            }
        }
    }
    files
}

fn branch_exists(repo_root: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(repo_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn generate_live_diff(repo_root: &Path, branch: &str, base_ref: &str) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args([
            "diff",
            "--no-color",
            "--patch",
            &format!("{base_ref}...{branch}"),
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to run git diff: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git diff failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_error_display_format() {
        let err = IpcError::validation("bad input");
        assert_eq!(err.to_string(), "[validation_error] bad input");
    }

    #[test]
    fn ipc_error_serializes() {
        let err = IpcError::internal("something broke");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("internal_error"));
        assert!(json.contains("something broke"));
    }

    #[test]
    fn check_status_serde_roundtrip() {
        let statuses = vec![
            CheckStatus::Passed,
            CheckStatus::Failed,
            CheckStatus::Warning,
            CheckStatus::Running,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let back: CheckStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn adapter_info_from_probe_result() {
        use hydra_core::adapter::*;
        let probe = ProbeResult {
            adapter_key: "claude".to_string(),
            tier: AdapterTier::Tier1,
            detect: DetectResult {
                status: DetectStatus::Ready,
                binary_path: Some("/usr/bin/claude".into()),
                version: Some("1.0.0".to_string()),
                supported_flags: vec!["--json".to_string()],
                confidence: CapabilityConfidence::Verified,
                error: None,
            },
            capabilities: CapabilitySet {
                json_stream: CapabilityEntry::verified(true),
                plain_text: CapabilityEntry::verified(true),
                force_edit_mode: CapabilityEntry::verified(false),
                sandbox_controls: CapabilityEntry::unknown(),
                approval_controls: CapabilityEntry::unknown(),
                session_resume: CapabilityEntry::unknown(),
                emits_usage: CapabilityEntry::unknown(),
            },
        };
        let info = AdapterInfo::from(&probe);
        assert_eq!(info.key, "claude");
        assert_eq!(info.tier, AdapterTier::Tier1);
        assert_eq!(info.status, DetectStatus::Ready);
    }

    #[test]
    fn preflight_result_serializes() {
        let result = PreflightResult {
            system_ready: true,
            all_tier1_ready: true,
            passed_count: 4,
            failed_count: 0,
            total_count: 4,
            health_score: 100.0,
            checks: vec![DiagnosticCheck {
                name: "Test".to_string(),
                description: "Test check".to_string(),
                status: CheckStatus::Passed,
                evidence: None,
            }],
            adapters: vec![],
            warnings: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("systemReady"));
        assert!(json.contains("healthScore"));
    }

    #[test]
    fn race_request_deserializes() {
        let json = r#"{"taskPrompt":"fix bug","agents":["claude"],"allowExperimental":false}"#;
        let req: RaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.task_prompt, "fix bug");
        assert_eq!(req.agents, vec!["claude"]);
        assert!(!req.allow_experimental);
    }

    #[test]
    fn parse_cli_summary_extracts_agents() {
        let payload = serde_json::json!({
            "run_id": "abc",
            "status": "Completed",
            "duration_ms": 5000,
            "total_cost": 0.42,
            "agents": [
                {
                    "agent": "claude",
                    "status": "Completed",
                    "duration_ms": 42,
                    "score": {
                        "composite": 95.1,
                        "mergeable": true,
                        "gate_failures": [],
                        "dimensions": [
                            { "name": "build", "score": 100.0, "evidence": {} },
                            { "name": "tests", "score": 90.0, "evidence": { "passed": 14, "failed": 0 } }
                        ]
                    }
                }
            ]
        });

        let parsed = parse_cli_race_summary(payload.to_string().as_bytes()).unwrap();
        assert_eq!(parsed.run_id, "abc");
        assert_eq!(parsed.status, "completed");
        assert_eq!(parsed.duration_ms, Some(5000));
        assert_eq!(parsed.total_cost, Some(0.42));
        assert_eq!(parsed.agents.len(), 1);
        assert_eq!(parsed.agents[0].agent_key, "claude");
        assert_eq!(parsed.agents[0].score, Some(95.1));
        assert_eq!(parsed.agents[0].mergeable, Some(true));
        assert!(parsed.agents[0].gate_failures.is_empty());
        assert_eq!(parsed.agents[0].dimensions.len(), 2);
        assert_eq!(parsed.agents[0].dimensions[0].name, "build");
        assert!((parsed.agents[0].dimensions[0].score - 100.0).abs() < 0.01);
    }

    #[test]
    fn parse_cli_summary_supports_nested_cost_and_camel_statuses() {
        let payload = serde_json::json!({
            "run_id": "run-2",
            "status": "TimedOut",
            "duration_ms": 2200,
            "cost": {
                "estimated_cost_usd": 1.25
            },
            "agents": [
                {
                    "agent": "codex",
                    "status": "TimedOut",
                    "duration_ms": 2200,
                    "score": {
                        "composite": 0.0,
                        "mergeable": false,
                        "gate_failures": ["timed_out"],
                        "dimensions": []
                    }
                }
            ]
        });

        let parsed = parse_cli_race_summary(payload.to_string().as_bytes()).unwrap();
        assert_eq!(parsed.status, "timed_out");
        assert_eq!(parsed.duration_ms, Some(2200));
        assert_eq!(parsed.total_cost, Some(1.25));
        assert_eq!(parsed.agents[0].status, "timed_out");
    }

    #[test]
    fn normalize_status_handles_mixed_delimiters_and_camel_case() {
        assert_eq!(normalize_status("TimedOut"), "timed_out");
        assert_eq!(normalize_status("timed-out"), "timed_out");
        assert_eq!(normalize_status("Already Fine"), "already_fine");
    }

    #[test]
    fn parse_diff_numstat_from_patch_counts_added_removed_per_file() {
        let patch = r#"diff --git a/src/a.rs b/src/a.rs
index 1111111..2222222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,2 +1,3 @@
 fn a() {
-  old
+  new
+  more
 }
diff --git a/src/b.rs b/src/b.rs
index 3333333..4444444 100644
--- a/src/b.rs
+++ b/src/b.rs
@@ -1,3 +1,2 @@
-removed
 stay
"#;
        let files = parse_diff_numstat_from_patch(patch);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "src/a.rs");
        assert_eq!(files[0].added, 2);
        assert_eq!(files[0].removed, 1);
        assert_eq!(files[1].path, "src/b.rs");
        assert_eq!(files[1].added, 0);
        assert_eq!(files[1].removed, 1);
    }

    #[test]
    fn parse_merge_preview_payload_conflict_json_is_not_clean() {
        let payload_json = serde_json::json!({
            "agent": "claude",
            "branch": "hydra/test/agent/claude",
            "success": false,
            "has_conflicts": true,
            "stdout": "",
            "stderr": "CONFLICT in src/main.rs",
            "report_path": ".hydra/runs/test/merge_report.json"
        })
        .to_string();

        let payload = parse_merge_preview_payload(true, "claude", &payload_json, "").unwrap();
        assert!(!payload.success);
        assert!(payload.has_conflicts);
    }

    #[test]
    fn parse_merge_preview_payload_non_conflict_failure_returns_error() {
        let err = parse_merge_preview_payload(
            false,
            "claude",
            "",
            "working tree has uncommitted changes",
        )
        .unwrap_err();
        assert!(err.contains("validation_error"));
        assert!(err.contains("working tree has uncommitted changes"));
    }
}
