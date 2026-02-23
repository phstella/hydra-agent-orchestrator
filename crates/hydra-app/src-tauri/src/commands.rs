use serde::{Deserialize, Serialize};
use tauri::State;

use hydra_core::artifact::events::RunEvent;
use hydra_core::artifact::manifest::RunManifest;
use hydra_core::artifact::run_dir::RunDir;
use hydra_core::config::HydraConfig;
use hydra_core::doctor::DoctorReport;
use hydra_core::merge::{MergeReport, MergeService};
use hydra_core::orchestrator::Orchestrator;

use crate::state::AppState;

/// Result of a race, serializable for IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResult {
    pub run_id: String,
    pub agent_key: String,
    pub status: String,
    pub artifact_dir: String,
}

/// Start a single-agent race.
#[tauri::command]
pub async fn start_race(
    state: State<'_, AppState>,
    agent: String,
    prompt: String,
) -> Result<RaceResult, String> {
    let repo_root = state.repo_root().await;
    let orchestrator = Orchestrator::new(state.config.clone(), repo_root);

    let result = orchestrator
        .race_single(&agent, &prompt)
        .await
        .map_err(|e| e.to_string())?;

    let first_agent = result.agents.first().ok_or("no agent results")?;

    Ok(RaceResult {
        run_id: result.run_id.to_string(),
        agent_key: first_agent.agent_key.clone(),
        status: format!("{:?}", first_agent.status),
        artifact_dir: result.artifact_dir.display().to_string(),
    })
}

/// Run the doctor check and return readiness report.
#[tauri::command]
pub async fn get_doctor_report(state: State<'_, AppState>) -> Result<DoctorReport, String> {
    let repo_root = state.repo_root().await;
    Ok(DoctorReport::run(Some(&repo_root)))
}

/// Get the manifest for a specific run.
#[tauri::command]
pub async fn get_run_manifest(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<RunManifest, String> {
    let repo_root = state.repo_root().await;
    let run_path = repo_root.join(".hydra").join("runs").join(&run_id);
    let run_dir = RunDir::open(&run_path);
    run_dir.read_manifest().map_err(|e| e.to_string())
}

/// Get all events for a specific run.
#[tauri::command]
pub async fn get_run_events(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<RunEvent>, String> {
    let repo_root = state.repo_root().await;
    let run_path = repo_root.join(".hydra").join("runs").join(&run_id);
    let run_dir = RunDir::open(&run_path);
    run_dir.read_events().map_err(|e| e.to_string())
}

/// Perform a merge dry-run to check for conflicts.
#[tauri::command]
pub async fn merge_dry_run(
    state: State<'_, AppState>,
    source_branch: String,
    target_branch: String,
) -> Result<MergeReport, String> {
    let repo_root = state.repo_root().await;
    let svc = MergeService::new(repo_root);
    svc.dry_run(&source_branch, &target_branch)
        .await
        .map_err(|e| e.to_string())
}

/// Execute a real merge.
#[tauri::command]
pub async fn merge_confirm(
    state: State<'_, AppState>,
    source_branch: String,
    target_branch: String,
) -> Result<MergeReport, String> {
    let repo_root = state.repo_root().await;
    let svc = MergeService::new(repo_root);
    svc.merge(&source_branch, &target_branch)
        .await
        .map_err(|e| e.to_string())
}

/// Get the current configuration.
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<HydraConfig, String> {
    Ok(state.config.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_result_serialization() {
        let result = RaceResult {
            run_id: "test-id".to_string(),
            agent_key: "claude".to_string(),
            status: "Completed".to_string(),
            artifact_dir: "/tmp/test".to_string(),
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let deser: RaceResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.run_id, "test-id");
        assert_eq!(deser.agent_key, "claude");
    }

    #[test]
    fn race_result_fields() {
        let result = RaceResult {
            run_id: "abc-123".to_string(),
            agent_key: "codex".to_string(),
            status: "Failed".to_string(),
            artifact_dir: "/home/user/.hydra/runs/abc-123".to_string(),
        };
        assert_eq!(result.run_id, "abc-123");
        assert_eq!(result.agent_key, "codex");
        assert_eq!(result.status, "Failed");
    }
}
