use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::sync::mpsc;
use uuid::Uuid;

use hydra_core::adapter::claude::ClaudeAdapter;
use hydra_core::adapter::codex::CodexAdapter;
use hydra_core::adapter::{AgentAdapter, BuiltCommand, SpawnRequest};
use hydra_core::artifact::{
    AgentEntry, EventKind, EventWriter, RunEvent, RunLayout, RunManifest, RunStatus,
};
use hydra_core::config::{HydraConfig, RetentionPolicy};
use hydra_core::security::{SandboxPolicy, SandboxResult};
use hydra_core::supervisor::{supervise, SupervisorEvent, SupervisorPolicy};
use hydra_core::worktree::{WorktreeInfo, WorktreeService};

pub struct RaceOpts {
    pub agent: String,
    pub prompt: String,
    pub base_ref: String,
    pub json: bool,
    pub unsafe_mode: bool,
}

pub async fn run_race(opts: RaceOpts) -> Result<()> {
    let config = load_race_config()?;
    let repo_root = discover_repo_root()?;
    let run_id = Uuid::new_v4();

    let adapter = resolve_adapter(&opts.agent, &config)?;
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
    let supported_flags = detect.supported_flags;

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
            "unsafe_mode": opts.unsafe_mode,
        }),
    ))?;

    let run_result = run_agent(
        Arc::clone(&adapter),
        &opts,
        &config,
        &wt_info,
        &mut event_writer,
        supported_flags,
    )
    .await;

    let mut final_status = match &run_result {
        Ok(outcome) => outcome.status.clone(),
        Err(_) => RunStatus::Failed,
    };
    let mut run_error = match run_result {
        Ok(outcome) => outcome.error,
        Err(e) => Some(format!("{e:#}")),
    };

    let cleanup_requested = should_cleanup_worktree(config.worktree.retain, &final_status);
    let cleanup_label = if cleanup_requested {
        "requested"
    } else {
        "retained"
    };
    let mut cleanup_performed = false;
    if cleanup_requested {
        match wt_service.force_cleanup(&wt_info).await {
            Ok(()) => cleanup_performed = true,
            Err(e) => {
                final_status = RunStatus::Failed;
                run_error = Some(match run_error {
                    Some(existing) => format!("{existing}; cleanup failed: {e}"),
                    None => format!("cleanup failed: {e}"),
                });
            }
        }
    }

    event_writer.write_event(&RunEvent::new(
        match final_status {
            RunStatus::Completed => EventKind::RunCompleted,
            _ => EventKind::RunFailed,
        },
        None,
        serde_json::json!({
            "status": format!("{final_status:?}"),
            "worktree_cleanup": cleanup_label,
        }),
    ))?;

    manifest.mark_completed(final_status.clone());
    manifest.write_to(&layout.manifest_path())?;

    let branch = wt_info.branch.clone();
    let worktree = wt_info.path.display().to_string();
    if opts.json {
        let summary = serde_json::json!({
            "run_id": run_id.to_string(),
            "status": format!("{final_status:?}"),
            "agent": adapter.key(),
            "branch": branch,
            "worktree": worktree,
            "artifacts": layout.base_dir().display().to_string(),
            "unsafe_mode": opts.unsafe_mode,
            "worktree_cleanup": cleanup_performed,
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!();
        println!("Run Summary");
        println!("===========");
        println!("  Run ID:    {run_id}");
        println!("  Agent:     {}", adapter.key());
        println!("  Status:    {final_status:?}");
        println!("  Branch:    {}", branch);
        println!("  Worktree:  {}", worktree);
        println!(
            "  Cleanup:   {}",
            if cleanup_performed {
                "performed"
            } else {
                "retained"
            }
        );
        println!("  Artifacts: {}", layout.base_dir().display());
    }

    if final_status != RunStatus::Completed {
        if !opts.json {
            if let Some(e) = &run_error {
                eprintln!("\nError: {e}");
            } else {
                eprintln!("\nError: run status is {final_status:?}");
            }
        }
        std::process::exit(1);
    }

    Ok(())
}

async fn run_agent(
    adapter: Arc<dyn AgentAdapter>,
    opts: &RaceOpts,
    config: &HydraConfig,
    wt_info: &WorktreeInfo,
    event_writer: &mut EventWriter,
    supported_flags: Vec<String>,
) -> Result<AgentRunResult> {
    let sandbox = if opts.unsafe_mode {
        SandboxPolicy::unsafe_mode(wt_info.path.clone())
    } else {
        SandboxPolicy::strict(wt_info.path.clone())
    };

    let req = SpawnRequest {
        task_prompt: opts.prompt.clone(),
        worktree_path: wt_info.path.clone(),
        timeout_seconds: config.supervisor.hard_timeout_seconds,
        allow_network: opts.unsafe_mode,
        force_edit: true,
        output_json_stream: true,
        unsafe_mode: opts.unsafe_mode,
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

fn resolve_adapter(agent_key: &str, config: &HydraConfig) -> Result<Arc<dyn AgentAdapter>> {
    match agent_key {
        "claude" => Ok(Arc::new(ClaudeAdapter::new(config.adapters.claude.clone()))),
        "codex" => Ok(Arc::new(CodexAdapter::new(config.adapters.codex.clone()))),
        other => bail!("unknown agent '{}'. Supported agents: claude, codex", other),
    }
}

fn sha256_short(input: &str) -> String {
    let digest = sha256_digest(input.as_bytes());
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

fn sha256_digest(data: &[u8]) -> [u8; 32] {
    const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = H0;
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, part) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([part[0], part[1], part[2], part[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
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
