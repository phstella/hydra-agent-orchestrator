use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use tokio::process::Command as TokioCommand;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinSet;
use uuid::Uuid;

use hydra_core::adapter::{AdapterRegistry, AdapterTier, AgentAdapter, BuiltCommand, SpawnRequest};
use hydra_core::artifact::{
    AgentEntry, EventKind, EventReader, EventWriter, RunEvent, RunHealthMetrics, RunLayout,
    RunManifest, RunStatus,
};
use hydra_core::config::{BudgetConfig, HydraConfig, RetentionPolicy};
use hydra_core::scoring::baseline::{
    capture_baseline, parse_lint_output, parse_test_output, persist_baseline, resolve_commands,
    run_command, BaselineResult, CommandResult, ResolvedCommands,
};
use hydra_core::scoring::build::score_build;
use hydra_core::scoring::cost::{CostEstimate, UsageAccumulator};
use hydra_core::scoring::diff_scope::{compute_diff_stats, score_diff_scope};
use hydra_core::scoring::lint::score_lint;
use hydra_core::scoring::ranking::{rank_agents, AgentScore};
use hydra_core::scoring::tests::score_tests;
use hydra_core::scoring::DimensionScore;
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
    pub run_id: Option<Uuid>,
}

pub async fn run_race(opts: RaceOpts) -> Result<()> {
    let run_started_at = Instant::now();

    let config = load_race_config()?;
    let repo_root = discover_repo_root()?;
    let run_id = opts.run_id.unwrap_or_else(Uuid::new_v4);

    let registry = AdapterRegistry::from_config(&config.adapters);
    let requested_agents = normalize_requested_agents(&opts.agents);
    let selected_agents = if requested_agents.is_empty() {
        default_tier1_keys(&registry)
    } else {
        requested_agents
    };
    if selected_agents.is_empty() {
        bail!("no adapters selected for race");
    }

    let adapters = registry
        .resolve_many(&selected_agents, opts.allow_experimental_adapters)
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
        let wt_info = match wt_service
            .create(run_id, adapter.key(), &opts.base_ref)
            .await
        {
            Ok(info) => info,
            Err(e) => {
                rollback_worktrees(&wt_service, &worktrees).await;
                return Err(anyhow::Error::new(e))
                    .with_context(|| format!("failed to create worktree for {}", adapter.key()));
            }
        };
        worktrees.push(wt_info);

        let detect = adapter.detect();
        supported_flags_map.insert(adapter.key().to_string(), detect.supported_flags);
    }

    let resolved_commands = resolve_commands(&config.scoring);
    let baseline = match capture_baseline(&worktrees[0].path, &config.scoring).await {
        Ok(result) => result,
        Err(e) => {
            rollback_worktrees(&wt_service, &worktrees).await;
            return Err(anyhow::Error::new(e)).context("failed to capture baseline");
        }
    };
    if let Err(e) = persist_baseline(&baseline, &layout.baseline_result()) {
        rollback_worktrees(&wt_service, &worktrees).await;
        return Err(anyhow::Error::new(e)).context("failed to persist baseline artifact");
    }
    if let Err(e) = persist_baseline_logs(&layout, &baseline) {
        rollback_worktrees(&wt_service, &worktrees).await;
        return Err(e).context("failed to persist baseline logs");
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
            "baseline_commands": {
                "build": resolved_commands.build.is_some(),
                "test": resolved_commands.test.is_some(),
                "lint": resolved_commands.lint.is_some(),
            },
        }),
    ))?;

    tracing::info!(
        run_id = %run_id,
        agents = ?agents_json,
        "race started with {} agent(s)",
        adapters.len()
    );

    let shared_budget = Arc::new(SharedBudgetState::default());
    let mut join_set = JoinSet::new();

    for (adapter, wt_info) in adapters.iter().zip(worktrees.iter()) {
        run_event_writer.write_event(&RunEvent::new(
            EventKind::AgentStarted,
            Some(adapter.key().to_string()),
            serde_json::json!({
                "tier": adapter.tier().to_string(),
            }),
        ))?;

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
        let expects_usage = adapter.capabilities().emits_usage.supported;
        let shared_budget = Arc::clone(&shared_budget);
        let budget = config.scoring.budget.clone();

        join_set.spawn(async move {
            let agent_key = adapter.key().to_string();
            let start = Instant::now();
            let run_ctx = SingleAgentRunCtx {
                prompt: &prompt,
                unsafe_mode,
                config: &config,
                wt_info: &wt_info,
                events_path: agent_events_path,
                supported_flags: flags,
                expects_usage,
                budget,
                shared_budget,
            };
            let result = run_single_agent(adapter, run_ctx).await;
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
        let (status, error, usage_status, usage_total_tokens, usage_cost) = match result {
            Ok(outcome) => (
                outcome.status.clone(),
                outcome.error.clone(),
                outcome.usage_status.as_str(),
                outcome.usage.total_tokens,
                outcome.usage.estimated_cost_usd,
            ),
            Err(e) => (
                RunStatus::Failed,
                Some(format!("{e:#}")),
                "unavailable",
                0,
                None,
            ),
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
                "usage_status": usage_status,
                "total_tokens": usage_total_tokens,
                "estimated_cost_usd": usage_cost,
            }),
        ))?;

        if status == RunStatus::Completed {
            any_completed = true;
        }
    }

    if any_completed {
        overall_status = RunStatus::Completed;
    }
    if shared_budget.should_stop() && !any_completed {
        overall_status = RunStatus::Interrupted;
    }

    let mut durations: HashMap<String, Duration> = HashMap::new();
    for (agent_key, _, duration) in &results {
        durations.insert(agent_key.clone(), *duration);
    }

    run_event_writer.write_event(&RunEvent::new(
        EventKind::ScoreStarted,
        None,
        serde_json::json!({}),
    ))?;

    let score_ctx = ScoreRunCtx {
        layout: &layout,
        base_ref: &opts.base_ref,
        config: &config,
        baseline: &baseline,
        commands: &resolved_commands,
        durations: &durations,
    };
    let (ranked_scores, scoring_error) = match score_agents(&adapters, &worktrees, &score_ctx).await
    {
        Ok(scores) => (scores, None),
        Err(err) => {
            tracing::error!(error = %err, "scoring failed");
            (Vec::new(), Some(format!("{err:#}")))
        }
    };

    run_event_writer.write_event(&RunEvent::new(
        EventKind::ScoreFinished,
        None,
        serde_json::json!({
            "ranked_agents": ranked_scores.len(),
            "error": scoring_error,
        }),
    ))?;

    if scoring_error.is_some() {
        overall_status = RunStatus::Failed;
    }

    let score_map: HashMap<String, AgentScore> = ranked_scores
        .iter()
        .map(|score| (score.agent_key.clone(), score.clone()))
        .collect();

    // Persist diff.patch for each agent before worktree cleanup.
    // The diff artifact must survive cleanup so the GUI can display it later.
    for (adapter, wt_info) in adapters.iter().zip(worktrees.iter()) {
        let diff_path = layout.agent_diff(adapter.key());
        match generate_diff_patch(&wt_info.path, &opts.base_ref).await {
            Ok(patch) => {
                if let Err(e) = std::fs::write(&diff_path, &patch) {
                    tracing::warn!(
                        agent = adapter.key(),
                        error = %e,
                        "failed to write diff.patch artifact"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    agent = adapter.key(),
                    error = %e,
                    "failed to generate diff.patch; skipping"
                );
            }
        }
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

    let budget_reason = shared_budget.stop_reason().await;
    run_event_writer.write_event(&RunEvent::new(
        match overall_status {
            RunStatus::Completed => EventKind::RunCompleted,
            _ => EventKind::RunFailed,
        },
        None,
        serde_json::json!({
            "status": format!("{overall_status:?}"),
            "budget_stop_reason": budget_reason,
        }),
    ))?;

    manifest.mark_completed(overall_status.clone());
    manifest.write_to(&layout.manifest_path())?;

    let health_metrics = EventReader::read_all(&layout.events_path())
        .ok()
        .map(|events| RunHealthMetrics::from_events(&events));

    let (run_input_tokens, run_output_tokens, run_total_tokens, run_estimated_cost) =
        aggregate_run_cost(&results);

    let run_duration_ms = run_started_at.elapsed().as_millis() as u64;

    // Output
    if opts.json {
        let agent_summaries: Vec<serde_json::Value> = results
            .iter()
            .map(|(key, result, duration)| {
                let (status, error, usage, usage_status) = match result {
                    Ok(o) => (
                        format!("{:?}", o.status),
                        o.error.clone(),
                        Some(o.usage.clone()),
                        o.usage_status.as_str(),
                    ),
                    Err(e) => (
                        "Failed".to_string(),
                        Some(format!("{e:#}")),
                        None,
                        "unavailable",
                    ),
                };
                let wt = worktrees.iter().find(|w| w.agent_key == *key);
                let cleaned = cleanup_results.get(key).copied().unwrap_or(false);
                let tier = adapters
                    .iter()
                    .find(|a| a.key() == key)
                    .map(|a| a.tier().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let score = score_map.get(key);
                serde_json::json!({
                    "agent": key,
                    "tier": tier,
                    "status": status,
                    "error": error,
                    "duration_ms": duration.as_millis() as u64,
                    "branch": wt.map(|w| w.branch.clone()),
                    "worktree_cleanup": cleaned,
                    "cost": usage.map(|c| serde_json::json!({
                        "status": usage_status,
                        "input_tokens": c.input_tokens,
                        "output_tokens": c.output_tokens,
                        "total_tokens": c.total_tokens,
                        "estimated_cost_usd": c.estimated_cost_usd,
                    })).unwrap_or_else(|| serde_json::json!({
                        "status": usage_status,
                        "input_tokens": null,
                        "output_tokens": null,
                        "total_tokens": null,
                        "estimated_cost_usd": null,
                    })),
                    "score": score.map(|s| serde_json::json!({
                        "composite": s.composite,
                        "mergeable": s.mergeable,
                        "gate_failures": s.gate_failures,
                        "dimensions": s.dimensions,
                    })),
                })
            })
            .collect();

        let summary = serde_json::json!({
            "run_id": run_id.to_string(),
            "status": format!("{overall_status:?}"),
            "duration_ms": run_duration_ms,
            "total_cost": run_estimated_cost,
            "agents": agent_summaries,
            "rankings": ranked_scores,
            "artifacts": layout.base_dir().display().to_string(),
            "unsafe_mode": opts.unsafe_mode,
            "baseline": {
                "path": layout.baseline_result().display().to_string(),
                "commands": {
                    "build": resolved_commands.build.is_some(),
                    "test": resolved_commands.test.is_some(),
                    "lint": resolved_commands.lint.is_some(),
                }
            },
            "cost": {
                "input_tokens": run_input_tokens,
                "output_tokens": run_output_tokens,
                "total_tokens": run_total_tokens,
                "estimated_cost_usd": run_estimated_cost,
            },
            "budget": {
                "max_tokens_total": config.scoring.budget.max_tokens_total,
                "max_cost_usd": config.scoring.budget.max_cost_usd,
                "stop_triggered": shared_budget.should_stop(),
                "stop_reason": shared_budget.stop_reason().await,
            },
            "health": health_metrics,
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!();
        println!("Run Summary");
        println!("===========");
        println!("  Run ID:    {run_id}");
        println!("  Status:    {overall_status:?}");
        println!("  Duration:  {:.1}s", run_duration_ms as f64 / 1000.0);
        if let Some(cost) = run_estimated_cost {
            println!("  Cost:      ${cost:.4}");
        }
        println!("  Artifacts: {}", layout.base_dir().display());
        println!("  Baseline:  {}", layout.baseline_result().display());
        println!();
        for (key, result, duration) in &results {
            let (status, error, usage, usage_status) = match result {
                Ok(o) => (
                    format!("{:?}", o.status),
                    o.error.clone(),
                    Some(o.usage.clone()),
                    o.usage_status.as_str(),
                ),
                Err(e) => (
                    "Failed".to_string(),
                    Some(format!("{e:#}")),
                    None,
                    "unavailable",
                ),
            };
            let wt = worktrees.iter().find(|w| w.agent_key == *key);
            let cleaned = cleanup_results.get(key).copied().unwrap_or(false);
            let tier_label = adapters
                .iter()
                .find(|a| a.key() == key)
                .map(|a| {
                    if a.tier() == AdapterTier::Experimental {
                        " [experimental]"
                    } else {
                        ""
                    }
                })
                .unwrap_or("");
            println!("  Agent:     {key}{tier_label}");
            println!("    Status:    {status}");
            println!("    Duration:  {:.1}s", duration.as_secs_f64());
            if let Some(score) = score_map.get(key) {
                println!(
                    "    Score:     {:.1} ({})",
                    score.composite,
                    if score.mergeable {
                        "mergeable"
                    } else {
                        "not mergeable"
                    }
                );
                if !score.gate_failures.is_empty() {
                    println!("    Gates:     {}", score.gate_failures.join("; "));
                }
            }
            if let Some(wt) = wt {
                println!("    Branch:    {}", wt.branch);
            }
            println!(
                "    Cleanup:   {}",
                if cleaned { "performed" } else { "retained" }
            );
            if let Some(usage) = usage {
                if usage_status == "captured" {
                    println!(
                        "    Cost:      tokens={} (in={}, out={}), est=${}",
                        usage.total_tokens,
                        usage.input_tokens,
                        usage.output_tokens,
                        usage
                            .estimated_cost_usd
                            .map(|c| format!("{c:.4}"))
                            .unwrap_or_else(|| "n/a".to_string())
                    );
                } else {
                    println!("    Cost:      {usage_status}");
                }
            } else {
                println!("    Cost:      unavailable");
            }
            if let Some(e) = &error {
                println!("    Error:     {e}");
            }
            println!();
        }

        println!("  Rankings:");
        if ranked_scores.is_empty() {
            println!("    (none)");
        } else {
            for (idx, score) in ranked_scores.iter().enumerate() {
                println!(
                    "    {}. {} {:.1} {}",
                    idx + 1,
                    score.agent_key,
                    score.composite,
                    if score.mergeable {
                        "(mergeable)"
                    } else {
                        "(not mergeable)"
                    }
                );
            }
        }
        println!();
        println!(
            "  Cost total: tokens={} (in={}, out={}), est=${}",
            run_total_tokens,
            run_input_tokens,
            run_output_tokens,
            run_estimated_cost
                .map(|c| format!("{c:.4}"))
                .unwrap_or_else(|| "n/a".to_string())
        );
        if shared_budget.should_stop() {
            println!(
                "  Budget stop: {}",
                shared_budget
                    .stop_reason()
                    .await
                    .unwrap_or_else(|| "triggered".to_string())
            );
        }
        if let Some(health) = health_metrics {
            println!(
                "  Health: success_rate={:.2}, adapter_errors={}, overhead_ms={}",
                health.success_rate,
                health.adapter_errors,
                health
                    .orchestration_overhead_ms
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "n/a".to_string())
            );
        }
    }

    if overall_status != RunStatus::Completed {
        if !opts.json {
            eprintln!("Error: race did not complete successfully");
        }
        std::process::exit(1);
    }

    Ok(())
}

struct SingleAgentRunCtx<'a> {
    prompt: &'a str,
    unsafe_mode: bool,
    config: &'a HydraConfig,
    wt_info: &'a WorktreeInfo,
    events_path: PathBuf,
    supported_flags: Vec<String>,
    expects_usage: bool,
    budget: BudgetConfig,
    shared_budget: Arc<SharedBudgetState>,
}

async fn run_single_agent(
    adapter: Arc<dyn AgentAdapter>,
    ctx: SingleAgentRunCtx<'_>,
) -> Result<AgentRunResult> {
    let mut event_writer =
        EventWriter::create(&ctx.events_path).context("failed to create per-agent event writer")?;

    let sandbox = if ctx.unsafe_mode {
        SandboxPolicy::unsafe_mode(ctx.wt_info.path.clone())
    } else {
        SandboxPolicy::strict(ctx.wt_info.path.clone())
    };

    let req = SpawnRequest {
        task_prompt: ctx.prompt.to_string(),
        worktree_path: ctx.wt_info.path.clone(),
        timeout_seconds: ctx.config.supervisor.hard_timeout_seconds,
        allow_network: ctx.unsafe_mode,
        force_edit: true,
        output_json_stream: true,
        unsafe_mode: ctx.unsafe_mode,
        supported_flags: ctx.supported_flags,
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

    let policy = SupervisorPolicy::from_hydra_config(&ctx.config.supervisor);
    let (event_tx, mut event_rx) = mpsc::channel::<SupervisorEvent>(256);

    let agent_key = adapter.key().to_string();
    let line_parser = {
        let parser_adapter = Arc::clone(&adapter);
        move |line: &str| parser_adapter.parse_line(line)
    };

    let handle = supervise(cmd, policy, event_tx, line_parser)
        .await
        .context("failed to supervise agent process")?;

    event_writer.write_event(&RunEvent::new(
        EventKind::AgentStarted,
        Some(agent_key.clone()),
        serde_json::json!({}),
    ))?;

    let mut usage = UsageAccumulator::new();
    let mut cancel_sent = false;
    let mut outcome = AgentRunResult {
        status: RunStatus::Failed,
        error: None,
        usage: CostEstimate {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            estimated_cost_usd: None,
        },
        usage_status: if ctx.expects_usage {
            UsageCaptureStatus::Missing
        } else {
            UsageCaptureStatus::Unavailable
        },
    };
    loop {
        tokio::select! {
            evt = event_rx.recv() => {
                let Some(evt) = evt else {
                    break;
                };
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
                        usage.process_event(agent_evt);
                        if let hydra_core::adapter::AgentEvent::Usage {
                            input_tokens,
                            output_tokens,
                            extra,
                        } = agent_evt
                        {
                            let delta_tokens = *input_tokens + *output_tokens;
                            let delta_cost = extra.get("cost_usd").and_then(|v| v.as_f64());
                            if let Some(_reason) = ctx
                                .shared_budget
                                .note_usage(delta_tokens, delta_cost, &ctx.budget)
                                .await
                            {
                                if !cancel_sent {
                                    handle.cancel().await;
                                    cancel_sent = true;
                                }
                            }
                        }
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
                            outcome.error = if ctx.shared_budget.should_stop() {
                                Some(
                                    ctx.shared_budget
                                        .stop_reason()
                                        .await
                                        .unwrap_or_else(|| "budget exceeded".to_string()),
                                )
                            } else {
                                Some(error.clone())
                            };
                        } else {
                            outcome.status = RunStatus::Failed;
                            outcome.error = Some(error.clone());
                        }
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
            _ = tokio::time::sleep(Duration::from_millis(100)), if !cancel_sent => {
                if ctx.shared_budget.should_stop() {
                    handle.cancel().await;
                    cancel_sent = true;
                }
            }
        }
    }

    outcome.usage = usage.to_estimate();
    outcome.usage_status = if usage.has_usage_data() {
        UsageCaptureStatus::Captured
    } else if ctx.expects_usage {
        UsageCaptureStatus::Missing
    } else {
        UsageCaptureStatus::Unavailable
    };
    Ok(outcome)
}

struct ScoreRunCtx<'a> {
    layout: &'a RunLayout,
    base_ref: &'a str,
    config: &'a HydraConfig,
    baseline: &'a BaselineResult,
    commands: &'a ResolvedCommands,
    durations: &'a HashMap<String, Duration>,
}

async fn score_agents(
    adapters: &[Arc<dyn AgentAdapter>],
    worktrees: &[WorktreeInfo],
    ctx: &ScoreRunCtx<'_>,
) -> Result<Vec<AgentScore>> {
    let mut agent_dimensions: Vec<(String, Vec<DimensionScore>)> = Vec::new();

    for (adapter, wt_info) in adapters.iter().zip(worktrees.iter()) {
        let dimensions = evaluate_agent_dimensions(
            ctx.layout,
            adapter.key(),
            wt_info,
            ctx.base_ref,
            ctx.config,
            ctx.baseline,
            ctx.commands,
        )
        .await
        .with_context(|| format!("failed scoring candidate for agent '{}'", adapter.key()))?;
        agent_dimensions.push((adapter.key().to_string(), dimensions));
    }

    let ranked = rank_agents(
        agent_dimensions,
        &ctx.config.scoring.weights,
        &ctx.config.scoring.gates,
        ctx.durations,
    );
    for score in &ranked {
        let path = ctx.layout.agent_score(&score.agent_key);
        let data = serde_json::to_string_pretty(score)?;
        std::fs::write(path, data)?;
    }
    Ok(ranked)
}

async fn evaluate_agent_dimensions(
    layout: &RunLayout,
    agent_key: &str,
    wt_info: &WorktreeInfo,
    base_ref: &str,
    config: &HydraConfig,
    baseline: &BaselineResult,
    commands: &ResolvedCommands,
) -> Result<Vec<DimensionScore>> {
    let mut dimensions = Vec::new();
    let timeout = config.scoring.timeout_per_check_seconds;
    let agent_dir = layout.agent_dir(agent_key);

    if let Some(build_cmd) = commands.build.as_deref() {
        match run_command(build_cmd, &wt_info.path, timeout).await {
            Ok(build_result) => {
                let build_log = agent_dir.join("build.log");
                write_command_artifact(&build_log, &build_result)?;
                let mut dim = score_build(baseline.build.as_ref(), &build_result);
                dim.evidence["artifact"] =
                    serde_json::Value::String(build_log.display().to_string());
                dimensions.push(dim);
            }
            Err(err) => {
                dimensions.push(failed_dimension("build", Some(build_cmd), &err.to_string()))
            }
        }
    }

    if let Some(test_cmd) = commands.test.as_deref() {
        match run_command(test_cmd, &wt_info.path, timeout).await {
            Ok(test_result_raw) => {
                let test_log = agent_dir.join("test.log");
                write_command_artifact(&test_log, &test_result_raw)?;
                let test_result = parse_test_output(&test_result_raw);
                let mut dim = score_tests(baseline.test.as_ref(), &test_result);
                dim.evidence["artifact"] =
                    serde_json::Value::String(test_log.display().to_string());
                dimensions.push(dim);
            }
            Err(err) => {
                dimensions.push(failed_dimension("tests", Some(test_cmd), &err.to_string()))
            }
        }
    }

    if let Some(lint_cmd) = commands.lint.as_deref() {
        match run_command(lint_cmd, &wt_info.path, timeout).await {
            Ok(lint_result_raw) => {
                let lint_log = agent_dir.join("lint.log");
                write_command_artifact(&lint_log, &lint_result_raw)?;
                let lint_result = parse_lint_output(&lint_result_raw);
                let mut dim = score_lint(baseline.lint.as_ref(), &lint_result);
                dim.evidence["artifact"] =
                    serde_json::Value::String(lint_log.display().to_string());
                dimensions.push(dim);
            }
            Err(err) => dimensions.push(failed_dimension("lint", Some(lint_cmd), &err.to_string())),
        }
    }

    match compute_diff_stats(&wt_info.path, base_ref).await {
        Ok(stats) => dimensions.push(score_diff_scope(&stats, &config.scoring.diff_scope)),
        Err(err) => dimensions.push(failed_dimension("diff_scope", None, &err.to_string())),
    }

    Ok(dimensions)
}

fn failed_dimension(name: &str, command: Option<&str>, error: &str) -> DimensionScore {
    DimensionScore {
        name: name.to_string(),
        score: 0.0,
        evidence: serde_json::json!({
            "command": command,
            "error": error,
        }),
    }
}

fn write_command_artifact(path: &Path, result: &CommandResult) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = format!(
        "command: {}\nexit_code: {}\nsuccess: {}\nduration_ms: {}\n\n[stdout]\n{}\n\n[stderr]\n{}\n",
        result.command,
        result.exit_code,
        result.success,
        result.duration_ms,
        result.stdout,
        result.stderr
    );
    std::fs::write(path, content)?;
    Ok(())
}

fn persist_baseline_logs(layout: &RunLayout, baseline: &BaselineResult) -> Result<()> {
    if let Some(build) = baseline.build.as_ref() {
        write_command_artifact(&layout.baseline_build_log(), build)?;
    }
    if let Some(test) = baseline.test.as_ref() {
        write_command_artifact(&layout.baseline_test_log(), &test.command_result)?;
    }
    if let Some(lint) = baseline.lint.as_ref() {
        write_command_artifact(&layout.baseline_lint_log(), &lint.command_result)?;
    }
    Ok(())
}

async fn rollback_worktrees(wt_service: &WorktreeService, worktrees: &[WorktreeInfo]) {
    for wt in worktrees {
        if let Err(e) = wt_service.force_cleanup(wt).await {
            tracing::warn!(
                branch = %wt.branch,
                path = %wt.path.display(),
                error = %e,
                "failed to rollback worktree after setup error"
            );
        }
    }
}

fn aggregate_run_cost(
    results: &[(String, Result<AgentRunResult>, Duration)],
) -> (u64, u64, u64, Option<f64>) {
    let mut input_tokens = 0_u64;
    let mut output_tokens = 0_u64;
    let mut total_tokens = 0_u64;
    let mut total_cost_usd = 0.0_f64;
    let mut has_cost = false;

    for (_, result, _) in results {
        if let Ok(outcome) = result {
            input_tokens += outcome.usage.input_tokens;
            output_tokens += outcome.usage.output_tokens;
            total_tokens += outcome.usage.total_tokens;
            if let Some(cost) = outcome.usage.estimated_cost_usd {
                total_cost_usd += cost;
                has_cost = true;
            }
        }
    }

    (
        input_tokens,
        output_tokens,
        total_tokens,
        has_cost.then_some(total_cost_usd),
    )
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

async fn generate_diff_patch(worktree_path: &Path, base_ref: &str) -> Result<String> {
    let base_output = TokioCommand::new("git")
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "diff",
            "--no-color",
            "--patch",
            base_ref,
        ])
        .output()
        .await
        .context("failed to run git diff for patch")?;

    if !base_output.status.success() {
        let stderr = String::from_utf8_lossy(&base_output.stderr)
            .trim()
            .to_string();
        bail!("git diff exited with non-zero status: {stderr}");
    }

    let mut patch = String::from_utf8_lossy(&base_output.stdout).to_string();

    let untracked_output = TokioCommand::new("git")
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "ls-files",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .await
        .context("failed to list untracked files for patch")?;

    if !untracked_output.status.success() {
        let stderr = String::from_utf8_lossy(&untracked_output.stderr)
            .trim()
            .to_string();
        bail!("git ls-files exited with non-zero status: {stderr}");
    }

    for rel_path in String::from_utf8_lossy(&untracked_output.stdout).lines() {
        let rel_path = rel_path.trim();
        if rel_path.is_empty() {
            continue;
        }

        let output = TokioCommand::new("git")
            .args([
                "-C",
                &worktree_path.to_string_lossy(),
                "diff",
                "--no-color",
                "--patch",
                "--no-index",
                "--",
                "/dev/null",
                rel_path,
            ])
            .output()
            .await
            .context("failed to generate patch for untracked file")?;

        // `git diff --no-index` exits with status 1 when differences are present.
        if !output.status.success() && output.status.code() != Some(1) {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            bail!("git diff --no-index exited with non-zero status: {stderr}");
        }

        let text = String::from_utf8_lossy(&output.stdout);
        if !text.trim().is_empty() {
            if !patch.is_empty() && !patch.ends_with('\n') {
                patch.push('\n');
            }
            patch.push_str(&text);
        }
    }

    Ok(patch)
}

fn should_cleanup_worktree(retain: RetentionPolicy, status: &RunStatus) -> bool {
    match retain {
        RetentionPolicy::None => true,
        RetentionPolicy::Failed => matches!(status, RunStatus::Completed),
        RetentionPolicy::All => false,
    }
}

fn normalize_requested_agents(requested: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for key in requested {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn default_tier1_keys(registry: &AdapterRegistry) -> Vec<String> {
    registry
        .tier1()
        .into_iter()
        .map(|a| a.key().to_string())
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum UsageCaptureStatus {
    Captured,
    Missing,
    Unavailable,
}

impl UsageCaptureStatus {
    fn as_str(self) -> &'static str {
        match self {
            UsageCaptureStatus::Captured => "captured",
            UsageCaptureStatus::Missing => "missing",
            UsageCaptureStatus::Unavailable => "unavailable",
        }
    }
}

struct AgentRunResult {
    status: RunStatus,
    error: Option<String>,
    usage: CostEstimate,
    usage_status: UsageCaptureStatus,
}

#[derive(Default)]
struct SharedBudgetState {
    total_tokens: AtomicU64,
    stop_requested: AtomicBool,
    total_cost_usd: Mutex<f64>,
    stop_reason: Mutex<Option<String>>,
}

impl SharedBudgetState {
    fn should_stop(&self) -> bool {
        self.stop_requested.load(Ordering::SeqCst)
    }

    async fn stop_reason(&self) -> Option<String> {
        self.stop_reason.lock().await.clone()
    }

    async fn note_usage(
        &self,
        delta_tokens: u64,
        delta_cost_usd: Option<f64>,
        budget: &BudgetConfig,
    ) -> Option<String> {
        let new_total_tokens =
            self.total_tokens.fetch_add(delta_tokens, Ordering::SeqCst) + delta_tokens;
        if let Some(max_tokens) = budget.max_tokens_total {
            if new_total_tokens >= max_tokens {
                return self
                    .trigger_stop(format!(
                        "token budget exceeded: {} >= {}",
                        new_total_tokens, max_tokens
                    ))
                    .await;
            }
        }

        if let Some(cost_delta) = delta_cost_usd {
            let mut total_cost = self.total_cost_usd.lock().await;
            *total_cost += cost_delta;
            if let Some(max_cost) = budget.max_cost_usd {
                if *total_cost >= max_cost {
                    return self
                        .trigger_stop(format!(
                            "cost budget exceeded: ${:.4} >= ${:.4}",
                            *total_cost, max_cost
                        ))
                        .await;
                }
            }
        }

        None
    }

    async fn trigger_stop(&self, reason: String) -> Option<String> {
        if self
            .stop_requested
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let mut guard = self.stop_reason.lock().await;
            *guard = Some(reason.clone());
            Some(reason)
        } else {
            self.stop_reason().await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::Arc;
    use tempfile::TempDir;

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

    #[test]
    fn normalize_requested_agents_trims_and_deduplicates() {
        let input = vec![
            "claude".to_string(),
            " codex ".to_string(),
            "".to_string(),
            "claude".to_string(),
        ];
        let normalized = normalize_requested_agents(&input);
        assert_eq!(normalized, vec!["claude", "codex"]);
    }

    #[tokio::test]
    async fn shared_budget_stops_on_token_limit() {
        let state = Arc::new(SharedBudgetState::default());
        let budget = BudgetConfig {
            max_tokens_total: Some(100),
            max_cost_usd: None,
        };

        assert!(state.note_usage(50, None, &budget).await.is_none());
        let reason = state.note_usage(60, None, &budget).await;
        assert!(reason.is_some());
        assert!(state.should_stop());
    }

    #[tokio::test]
    async fn generate_diff_patch_includes_uncommitted_new_file_changes() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        fn git(repo: &Path, args: &[&str]) {
            let output = Command::new("git")
                .args(args)
                .current_dir(repo)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        git(repo, &["init"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "Test User"]);

        std::fs::write(repo.join("README.md"), "base\n").unwrap();
        git(repo, &["add", "README.md"]);
        git(repo, &["commit", "-m", "init"]);

        std::fs::write(repo.join("snake.py"), "print('snake')\n").unwrap();

        let patch = generate_diff_patch(repo, "HEAD").await.unwrap();
        assert!(patch.contains("diff --git a/snake.py b/snake.py"));
        assert!(patch.contains("+print('snake')"));
    }
}
