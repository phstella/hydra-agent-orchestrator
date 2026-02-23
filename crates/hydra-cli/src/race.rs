use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use uuid::Uuid;

use hydra_core::adapter::{AdapterRegistry, AgentAdapter, BuiltCommand, SpawnRequest};
use hydra_core::artifact::{
    AgentEntry, EventKind, EventWriter, RunEvent, RunLayout, RunManifest, RunStatus,
};
use hydra_core::config::{HydraConfig, RetentionPolicy};
use hydra_core::security::{SandboxPolicy, SandboxResult};
use hydra_core::supervisor::{supervise, SupervisorEvent, SupervisorPolicy};
use hydra_core::worktree::{WorktreeInfo, WorktreeService};

pub struct RaceOpts {
    pub agents: Vec<String>,
    pub prompt: String,
    pub base_ref: String,
    pub json: bool,
    pub unsafe_mode: bool,
    pub allow_experimental_adapters: bool,
}

pub async fn run_race(opts: RaceOpts) -> Result<()> {
    let config = load_race_config()?;
    let repo_root = discover_repo_root()?;
    let run_id = Uuid::new_v4();

    let registry = AdapterRegistry::from_config(&config.adapters);
    let adapters = registry
        .resolve_many(&opts.agents, opts.allow_experimental_adapters)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    for adapter in &adapters {
        let detect = adapter.detect();
        if !detect.status.is_available() {
            let detail = detect
                .error
                .clone()
                .unwrap_or_else(|| "probe failed with no detail".to_string());
            bail!(
                "adapter '{}' is not ready ({}): {}",
                adapter.key(),
                detect.status_label(),
                detail
            );
        }
    }

    let agent_keys: Vec<&str> = adapters.iter().map(|a| a.key()).collect();
    let hydra_root = repo_root.join(".hydra");
    let layout = RunLayout::new(&hydra_root, run_id);
    layout
        .create_dirs(&agent_keys)
        .context("failed to create run artifact directory")?;

    let wt_base = repo_root.join(&config.worktree.base_dir);
    let wt_service = Arc::new(WorktreeService::new(repo_root.clone(), wt_base));

    let mut worktrees: Vec<WorktreeInfo> = Vec::new();
    let mut supported_flags_map: HashMap<String, Vec<String>> = HashMap::new();

    for adapter in &adapters {
        let wt_info = wt_service
            .create(run_id, adapter.key(), &opts.base_ref)
            .await
            .with_context(|| format!("failed to create worktree for {}", adapter.key()))?;
        worktrees.push(wt_info);

        let detect = adapter.detect();
        supported_flags_map.insert(adapter.key().to_string(), detect.supported_flags);
    }

    let agent_entries: Vec<AgentEntry> = adapters
        .iter()
        .zip(worktrees.iter())
        .map(|(adapter, wt)| AgentEntry {
            agent_key: adapter.key().to_string(),
            tier: adapter.tier().to_string(),
            branch: wt.branch.clone(),
            worktree_path: Some(wt.path.display().to_string()),
        })
        .collect();

    let mut manifest = RunManifest::new(
        run_id,
        repo_root.display().to_string(),
        opts.base_ref.clone(),
        sha256_short(&opts.prompt),
        agent_entries,
    );
    manifest
        .write_to(&layout.manifest_path())
        .context("failed to write initial manifest")?;

    let mut run_event_writer =
        EventWriter::create(&layout.events_path()).context("failed to create event writer")?;

    let agents_json: Vec<&str> = adapters.iter().map(|a| a.key()).collect();
    run_event_writer.write_event(&RunEvent::new(
        EventKind::RunStarted,
        None,
        serde_json::json!({
            "run_id": run_id.to_string(),
            "agents": agents_json,
            "task_prompt": &opts.prompt,
            "unsafe_mode": opts.unsafe_mode,
        }),
    ))?;

    tracing::info!(
        run_id = %run_id,
        agents = ?agents_json,
        "race started with {} agent(s)",
        adapters.len()
    );

    let mut join_set = JoinSet::new();

    for (adapter, wt_info) in adapters.iter().zip(worktrees.iter()) {
        let adapter = Arc::clone(adapter);
        let config = config.clone();
        let wt_info = wt_info.clone();
        let prompt = opts.prompt.clone();
        let unsafe_mode = opts.unsafe_mode;
        let flags = supported_flags_map
            .get(adapter.key())
            .cloned()
            .unwrap_or_default();
        let agent_events_path = layout.agent_dir(adapter.key()).join("events.jsonl");

        join_set.spawn(async move {
            let agent_key = adapter.key().to_string();
            let start = Instant::now();
            let result = run_single_agent(
                adapter,
                &prompt,
                unsafe_mode,
                &config,
                &wt_info,
                agent_events_path,
                flags,
            )
            .await;
            let duration = start.elapsed();
            (agent_key, result, duration)
        });
    }

    let mut results: Vec<(String, Result<AgentRunResult>, Duration)> = Vec::new();
    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok(tuple) => results.push(tuple),
            Err(e) => {
                tracing::error!(error = %e, "agent task panicked");
            }
        }
    }

    let mut any_completed = false;
    let mut overall_status = RunStatus::Failed;

    for (agent_key, result, _duration) in &results {
        let (status, error) = match result {
            Ok(outcome) => (outcome.status.clone(), outcome.error.clone()),
            Err(e) => (RunStatus::Failed, Some(format!("{e:#}"))),
        };

        run_event_writer.write_event(&RunEvent::new(
            match &status {
                RunStatus::Completed => EventKind::AgentCompleted,
                _ => EventKind::AgentFailed,
            },
            Some(agent_key.clone()),
            serde_json::json!({
                "status": format!("{status:?}"),
                "error": error,
            }),
        ))?;

        if status == RunStatus::Completed {
            any_completed = true;
        }
    }

    if any_completed {
        overall_status = RunStatus::Completed;
    }

    // Cleanup worktrees
    let mut cleanup_results: HashMap<String, bool> = HashMap::new();
    for (adapter, wt_info) in adapters.iter().zip(worktrees.iter()) {
        let agent_status = results
            .iter()
            .find(|(k, _, _)| k == adapter.key())
            .map(|(_, r, _)| match r {
                Ok(o) => o.status.clone(),
                Err(_) => RunStatus::Failed,
            })
            .unwrap_or(RunStatus::Failed);

        let cleanup_requested = should_cleanup_worktree(config.worktree.retain, &agent_status);
        if cleanup_requested {
            match wt_service.force_cleanup(wt_info).await {
                Ok(()) => {
                    cleanup_results.insert(adapter.key().to_string(), true);
                }
                Err(e) => {
                    tracing::warn!(
                        agent = adapter.key(),
                        error = %e,
                        "worktree cleanup failed"
                    );
                    cleanup_results.insert(adapter.key().to_string(), false);
                }
            }
        } else {
            cleanup_results.insert(adapter.key().to_string(), false);
        }
    }

    run_event_writer.write_event(&RunEvent::new(
        match overall_status {
            RunStatus::Completed => EventKind::RunCompleted,
            _ => EventKind::RunFailed,
        },
        None,
        serde_json::json!({
            "status": format!("{overall_status:?}"),
        }),
    ))?;

    manifest.mark_completed(overall_status.clone());
    manifest.write_to(&layout.manifest_path())?;

    // Output
    if opts.json {
        let agent_summaries: Vec<serde_json::Value> = results
            .iter()
            .map(|(key, result, duration)| {
                let (status, error) = match result {
                    Ok(o) => (format!("{:?}", o.status), o.error.clone()),
                    Err(e) => ("Failed".to_string(), Some(format!("{e:#}"))),
                };
                let wt = worktrees.iter().find(|w| w.agent_key == *key);
                let cleaned = cleanup_results.get(key).copied().unwrap_or(false);
                serde_json::json!({
                    "agent": key,
                    "status": status,
                    "error": error,
                    "duration_ms": duration.as_millis() as u64,
                    "branch": wt.map(|w| w.branch.clone()),
                    "worktree_cleanup": cleaned,
                })
            })
            .collect();

        let summary = serde_json::json!({
            "run_id": run_id.to_string(),
            "status": format!("{overall_status:?}"),
            "agents": agent_summaries,
            "artifacts": layout.base_dir().display().to_string(),
            "unsafe_mode": opts.unsafe_mode,
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!();
        println!("Run Summary");
        println!("===========");
        println!("  Run ID:    {run_id}");
        println!("  Status:    {overall_status:?}");
        println!("  Artifacts: {}", layout.base_dir().display());
        println!();
        for (key, result, duration) in &results {
            let (status, error) = match result {
                Ok(o) => (format!("{:?}", o.status), o.error.clone()),
                Err(e) => ("Failed".to_string(), Some(format!("{e:#}"))),
            };
            let wt = worktrees.iter().find(|w| w.agent_key == *key);
            let cleaned = cleanup_results.get(key).copied().unwrap_or(false);
            let tier_label = adapters
                .iter()
                .find(|a| a.key() == key)
                .map(|a| {
                    if a.tier() == hydra_core::adapter::AdapterTier::Experimental {
                        " [experimental]"
                    } else {
                        ""
                    }
                })
                .unwrap_or("");
            println!("  Agent:     {key}{tier_label}");
            println!("    Status:    {status}");
            println!("    Duration:  {:.1}s", duration.as_secs_f64());
            if let Some(wt) = wt {
                println!("    Branch:    {}", wt.branch);
            }
            println!(
                "    Cleanup:   {}",
                if cleaned { "performed" } else { "retained" }
            );
            if let Some(e) = &error {
                println!("    Error:     {e}");
            }
            println!();
        }
    }

    if overall_status != RunStatus::Completed {
        if !opts.json {
            eprintln!("Error: no agent completed successfully");
        }
        std::process::exit(1);
    }

    Ok(())
}

async fn run_single_agent(
    adapter: Arc<dyn AgentAdapter>,
    prompt: &str,
    unsafe_mode: bool,
    config: &HydraConfig,
    wt_info: &WorktreeInfo,
    events_path: PathBuf,
    supported_flags: Vec<String>,
) -> Result<AgentRunResult> {
    let mut event_writer =
        EventWriter::create(&events_path).context("failed to create per-agent event writer")?;

    let sandbox = if unsafe_mode {
        SandboxPolicy::unsafe_mode(wt_info.path.clone())
    } else {
        SandboxPolicy::strict(wt_info.path.clone())
    };

    let req = SpawnRequest {
        task_prompt: prompt.to_string(),
        worktree_path: wt_info.path.clone(),
        timeout_seconds: config.supervisor.hard_timeout_seconds,
        allow_network: unsafe_mode,
        force_edit: true,
        output_json_stream: true,
        unsafe_mode,
        supported_flags,
    };

    let cmd: BuiltCommand = adapter
        .build_command(&req)
        .context("failed to build agent command")?;
    match sandbox.check_path(&cmd.cwd) {
        SandboxResult::Allowed => {}
        SandboxResult::Blocked { path, allowed_root } => {
            bail!(
                "sandbox blocked command cwd '{}' (allowed root '{}')",
                path.display(),
                allowed_root.display()
            );
        }
    }

    let policy = SupervisorPolicy::from_hydra_config(&config.supervisor);
    let (event_tx, mut event_rx) = mpsc::channel::<SupervisorEvent>(256);

    let agent_key = adapter.key().to_string();
    let line_parser = {
        let parser_adapter = Arc::clone(&adapter);
        move |line: &str| parser_adapter.parse_line(line)
    };

    let _handle = supervise(cmd, policy, event_tx, line_parser)
        .await
        .context("failed to supervise agent process")?;

    event_writer.write_event(&RunEvent::new(
        EventKind::AgentStarted,
        Some(agent_key.clone()),
        serde_json::json!({}),
    ))?;

    let mut outcome = AgentRunResult {
        status: RunStatus::Failed,
        error: None,
    };
    while let Some(evt) = event_rx.recv().await {
        match &evt {
            SupervisorEvent::Started { pid } => {
                tracing::info!(pid = pid, agent = %agent_key, "agent process started");
            }
            SupervisorEvent::Stdout(line) => {
                event_writer.write_event(&RunEvent::new(
                    EventKind::AgentStdout,
                    Some(agent_key.clone()),
                    serde_json::json!({ "line": line }),
                ))?;
            }
            SupervisorEvent::Stderr(line) => {
                event_writer.write_event(&RunEvent::new(
                    EventKind::AgentStderr,
                    Some(agent_key.clone()),
                    serde_json::json!({ "line": line }),
                ))?;
            }
            SupervisorEvent::AgentEvent(agent_evt) => {
                event_writer.write_event(&RunEvent::new(
                    EventKind::AgentStdout,
                    Some(agent_key.clone()),
                    serde_json::to_value(agent_evt).unwrap_or_default(),
                ))?;
            }
            SupervisorEvent::Completed {
                exit_code,
                duration,
            } => {
                tracing::info!(
                    exit_code = exit_code,
                    duration_ms = duration.as_millis() as u64,
                    agent = %agent_key,
                    "agent completed"
                );
                event_writer.write_event(&RunEvent::new(
                    EventKind::AgentCompleted,
                    Some(agent_key.clone()),
                    serde_json::json!({
                        "exit_code": exit_code,
                        "duration_ms": duration.as_millis() as u64,
                    }),
                ))?;
                outcome.status = RunStatus::Completed;
                break;
            }
            SupervisorEvent::Failed { error, duration } => {
                tracing::warn!(
                    error = %error,
                    duration_ms = duration.as_millis() as u64,
                    agent = %agent_key,
                    "agent failed"
                );
                event_writer.write_event(&RunEvent::new(
                    EventKind::AgentFailed,
                    Some(agent_key.clone()),
                    serde_json::json!({
                        "error": error,
                        "duration_ms": duration.as_millis() as u64,
                    }),
                ))?;
                if error.contains("cancelled") {
                    outcome.status = RunStatus::Interrupted;
                } else {
                    outcome.status = RunStatus::Failed;
                }
                outcome.error = Some(error.clone());
                break;
            }
            SupervisorEvent::TimedOut { kind, duration } => {
                tracing::warn!(
                    kind = %kind,
                    duration_ms = duration.as_millis() as u64,
                    agent = %agent_key,
                    "agent timed out"
                );
                event_writer.write_event(&RunEvent::new(
                    EventKind::AgentFailed,
                    Some(agent_key.clone()),
                    serde_json::json!({
                        "error": format!("timed out ({kind})"),
                        "duration_ms": duration.as_millis() as u64,
                    }),
                ))?;
                outcome.status = RunStatus::TimedOut;
                outcome.error = Some(format!("timed out ({kind})"));
                break;
            }
        }
    }

    Ok(outcome)
}

fn load_race_config() -> Result<HydraConfig> {
    let path = Path::new("hydra.toml");
    hydra_core::config::load_config(path).context("failed to load hydra.toml")
}

fn discover_repo_root() -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse")?;

    if !output.status.success() {
        bail!("not inside a git repository");
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

fn sha256_short(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    let mut out = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn should_cleanup_worktree(retain: RetentionPolicy, status: &RunStatus) -> bool {
    match retain {
        RetentionPolicy::None => true,
        RetentionPolicy::Failed => matches!(status, RunStatus::Completed),
        RetentionPolicy::All => false,
    }
}

struct AgentRunResult {
    status: RunStatus,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_short_matches_known_vector() {
        assert_eq!(sha256_short("abc"), "ba7816bf8f01cfea");
    }

    #[test]
    fn retention_policy_cleanup_behavior() {
        assert!(should_cleanup_worktree(
            RetentionPolicy::None,
            &RunStatus::Completed
        ));
        assert!(should_cleanup_worktree(
            RetentionPolicy::None,
            &RunStatus::Failed
        ));

        assert!(should_cleanup_worktree(
            RetentionPolicy::Failed,
            &RunStatus::Completed
        ));
        assert!(!should_cleanup_worktree(
            RetentionPolicy::Failed,
            &RunStatus::Failed
        ));

        assert!(!should_cleanup_worktree(
            RetentionPolicy::All,
            &RunStatus::Completed
        ));
    }
}
