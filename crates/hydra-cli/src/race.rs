use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use tokio::sync::mpsc;
use uuid::Uuid;

use hydra_core::adapter::claude::ClaudeAdapter;
use hydra_core::adapter::codex::CodexAdapter;
use hydra_core::adapter::{AgentAdapter, AgentEvent, BuiltCommand, SpawnRequest};
use hydra_core::artifact::{
    AgentEntry, EventKind, EventWriter, RunEvent, RunLayout, RunManifest, RunStatus,
};
use hydra_core::config::HydraConfig;
use hydra_core::supervisor::{supervise, SupervisorEvent, SupervisorPolicy};
use hydra_core::worktree::{WorktreeInfo, WorktreeService};

pub struct RaceOpts {
    pub agent: String,
    pub prompt: String,
    pub base_ref: String,
    pub json: bool,
}

pub async fn run_race(opts: RaceOpts) -> Result<()> {
    let config = load_race_config()?;
    let repo_root = discover_repo_root()?;
    let run_id = Uuid::new_v4();

    let adapter = resolve_adapter(&opts.agent, &config)?;

    let hydra_root = repo_root.join(".hydra");
    let layout = RunLayout::new(&hydra_root, run_id);
    layout
        .create_dirs(&[adapter.key()])
        .context("failed to create run artifact directory")?;

    let wt_base = repo_root.join(&config.worktree.base_dir);
    let wt_service = WorktreeService::new(repo_root.clone(), wt_base);

    let wt_info = wt_service
        .create(run_id, adapter.key(), &opts.base_ref)
        .await
        .context("failed to create worktree")?;

    tracing::info!(
        run_id = %run_id,
        agent = adapter.key(),
        worktree = %wt_info.path.display(),
        branch = %wt_info.branch,
        "race started"
    );

    let mut manifest = RunManifest::new(
        run_id,
        repo_root.display().to_string(),
        opts.base_ref.clone(),
        sha256_short(&opts.prompt),
        vec![AgentEntry {
            agent_key: adapter.key().to_string(),
            tier: adapter.tier().to_string(),
            branch: wt_info.branch.clone(),
            worktree_path: Some(wt_info.path.display().to_string()),
        }],
    );
    manifest
        .write_to(&layout.manifest_path())
        .context("failed to write initial manifest")?;

    let mut event_writer =
        EventWriter::create(&layout.events_path()).context("failed to create event writer")?;

    event_writer.write_event(&RunEvent::new(
        EventKind::RunStarted,
        None,
        serde_json::json!({
            "run_id": run_id.to_string(),
            "agent": adapter.key(),
            "task_prompt": &opts.prompt,
        }),
    ))?;

    let result = run_agent(
        adapter.as_ref(),
        &opts,
        &config,
        &wt_info,
        &mut event_writer,
    )
    .await;

    let final_status = match &result {
        Ok(()) => RunStatus::Completed,
        Err(_) => RunStatus::Failed,
    };

    event_writer.write_event(&RunEvent::new(
        match final_status {
            RunStatus::Completed => EventKind::RunCompleted,
            _ => EventKind::RunFailed,
        },
        None,
        serde_json::json!({ "status": format!("{final_status:?}") }),
    ))?;

    manifest.mark_completed(final_status.clone());
    manifest.write_to(&layout.manifest_path())?;

    if opts.json {
        let summary = serde_json::json!({
            "run_id": run_id.to_string(),
            "status": format!("{final_status:?}"),
            "agent": adapter.key(),
            "branch": wt_info.branch,
            "worktree": wt_info.path.display().to_string(),
            "artifacts": layout.base_dir().display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!();
        println!("Run Summary");
        println!("===========");
        println!("  Run ID:    {run_id}");
        println!("  Agent:     {}", adapter.key());
        println!("  Status:    {final_status:?}");
        println!("  Branch:    {}", wt_info.branch);
        println!("  Worktree:  {}", wt_info.path.display());
        println!("  Artifacts: {}", layout.base_dir().display());
    }

    if let Err(e) = &result {
        if !opts.json {
            eprintln!("\nError: {e:#}");
        }
        std::process::exit(1);
    }

    Ok(())
}

async fn run_agent(
    adapter: &dyn AgentAdapter,
    opts: &RaceOpts,
    config: &HydraConfig,
    wt_info: &WorktreeInfo,
    event_writer: &mut EventWriter,
) -> Result<()> {
    let req = SpawnRequest {
        task_prompt: opts.prompt.clone(),
        worktree_path: wt_info.path.clone(),
        timeout_seconds: config.supervisor.hard_timeout_seconds,
        allow_network: false,
        force_edit: true,
        output_json_stream: true,
    };

    let cmd: BuiltCommand = adapter
        .build_command(&req)
        .context("failed to build agent command")?;

    let policy = SupervisorPolicy::from_hydra_config(&config.supervisor);
    let (event_tx, mut event_rx) = mpsc::channel::<SupervisorEvent>(256);

    let agent_key = adapter.key().to_string();
    let line_parser = {
        let adapter_key = agent_key.clone();
        move |line: &str| -> Option<AgentEvent> {
            match adapter_key.as_str() {
                "claude" => {
                    hydra_core::adapter::claude::ClaudeAdapter::parse_stream_json_line(line)
                }
                "codex" => hydra_core::adapter::codex::CodexAdapter::parse_json_line(line),
                _ => None,
            }
        }
    };

    let _handle = supervise(cmd, policy, event_tx, line_parser)
        .await
        .context("failed to supervise agent process")?;

    event_writer.write_event(&RunEvent::new(
        EventKind::AgentStarted,
        Some(agent_key.clone()),
        serde_json::json!({}),
    ))?;

    let mut agent_exit_ok = false;
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
                agent_exit_ok = true;
                break;
            }
            SupervisorEvent::Failed { error, duration } => {
                tracing::warn!(
                    error = %error,
                    duration_ms = duration.as_millis() as u64,
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
                break;
            }
            SupervisorEvent::TimedOut { kind, duration } => {
                tracing::warn!(
                    kind = %kind,
                    duration_ms = duration.as_millis() as u64,
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
                break;
            }
        }
    }

    if !agent_exit_ok {
        bail!("agent did not complete successfully");
    }
    Ok(())
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

fn resolve_adapter(agent_key: &str, config: &HydraConfig) -> Result<Box<dyn AgentAdapter>> {
    match agent_key {
        "claude" => Ok(Box::new(ClaudeAdapter::new(config.adapters.claude.clone()))),
        "codex" => Ok(Box::new(CodexAdapter::new(config.adapters.codex.clone()))),
        other => bail!("unknown agent '{}'. Supported agents: claude, codex", other),
    }
}

fn sha256_short(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
