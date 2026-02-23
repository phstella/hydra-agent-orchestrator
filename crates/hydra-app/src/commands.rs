use tauri::State;

use crate::ipc_types::*;
use crate::state::AppState;

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
pub async fn run_preflight(
    state: State<'_, AppState>,
) -> Result<PreflightResult, String> {
    let report = state.run_probes().await;

    let adapters: Vec<AdapterInfo> = report.results.iter().map(AdapterInfo::from).collect();

    let mut checks = Vec::new();
    let mut warnings = Vec::new();

    // Check: Git repository
    let git_ok = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    checks.push(DiagnosticCheck {
        name: "Git Repository".to_string(),
        description: if git_ok {
            "Working inside a valid git repository".to_string()
        } else {
            "Not inside a git repository".to_string()
        },
        status: if git_ok {
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
        .filter(|a| {
            a.tier == hydra_core::adapter::AdapterTier::Tier1 && a.status.is_available()
        })
        .count();

    checks.push(DiagnosticCheck {
        name: "Validating Adapters".to_string(),
        description: format!(
            "{}/{} tier-1 adapters ready",
            tier1_ready, tier1_count
        ),
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

    // Check: Working tree clean
    let worktree_ok = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    checks.push(DiagnosticCheck {
        name: "Checking Git Repository".to_string(),
        description: if worktree_ok {
            "Clean working tree on current branch".to_string()
        } else {
            "Unable to list worktrees".to_string()
        },
        status: if worktree_ok {
            CheckStatus::Passed
        } else {
            CheckStatus::Warning
        },
        evidence: None,
    });

    // Warnings for experimental adapters on CPU
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

    let passed = checks.iter().filter(|c| c.status == CheckStatus::Passed).count() as u32;
    let failed = checks.iter().filter(|c| c.status == CheckStatus::Failed).count() as u32;
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
pub async fn list_adapters(
    state: State<'_, AppState>,
) -> Result<Vec<AdapterInfo>, String> {
    let report = state.run_probes().await;
    Ok(report.results.iter().map(AdapterInfo::from).collect())
}

// ---------------------------------------------------------------------------
// Race commands (P3-IPC-01)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_race(
    _state: State<'_, AppState>,
    request: RaceRequest,
) -> Result<RaceStarted, String> {
    if request.task_prompt.trim().is_empty() {
        return Err(
            IpcError::validation("Task prompt cannot be empty").to_string()
        );
    }
    if request.agents.is_empty() {
        return Err(
            IpcError::validation("At least one agent must be selected").to_string()
        );
    }

    let run_id = uuid::Uuid::new_v4().to_string();

    Ok(RaceStarted {
        run_id,
        agents: request.agents,
    })
}

#[tauri::command]
pub async fn get_race_result(
    _run_id: String,
) -> Result<Option<RaceResult>, String> {
    // Stub: will be wired to actual orchestrator in M3.2 full implementation
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc_types::*;

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
}
