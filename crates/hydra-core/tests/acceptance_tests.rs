//! M5.5: Release candidate acceptance tests.
//!
//! These tests exercise high-level library functionality to verify that
//! hydra-core is ready for release.

use std::path::PathBuf;
use std::time::Duration;

use hydra_core::artifact::events::{EventType, RunEvent};
use hydra_core::artifact::manifest::{RunManifest, RunStatus};
use hydra_core::artifact::run_dir::RunDir;
use hydra_core::config::{DiffScopeConfig, HydraConfig, ScoringConfig};
use hydra_core::doctor::DoctorReport;
use hydra_core::migration::{MigrationTool, SchemaVersion, CURRENT_SCHEMA_VERSION};
use hydra_core::platform;
use hydra_core::recovery::RecoveryService;
use hydra_core::scoring::composite::AgentInput;
use hydra_core::scoring::{
    rank_agents, score_build, score_diff_scope, score_lint, score_speed, score_tests,
};
use hydra_core::workflow::{
    builder_reviewer_refiner, iterative_refinement, specialization, WorkflowEngine, WorkflowStatus,
};

use chrono::Utc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Platform detection
// ---------------------------------------------------------------------------

#[test]
fn platform_detection_works() {
    // At least one must be true on any supported platform.
    assert!(platform::is_unix() || platform::is_windows());
}

// ---------------------------------------------------------------------------
// Doctor report generation
// ---------------------------------------------------------------------------

#[test]
fn doctor_report_can_be_generated() {
    let report = DoctorReport::run(None);

    // We can't assert overall_ready in CI (adapters may not be installed),
    // but the report should be well-formed and serialisable.
    let json = serde_json::to_string(&report).expect("report serialises to JSON");
    assert!(!json.is_empty());
}

// ---------------------------------------------------------------------------
// Artifact integrity: write + read back
// ---------------------------------------------------------------------------

#[test]
fn artifact_write_and_read_back() {
    let tmp = tempfile::tempdir().unwrap();
    let run_id = Uuid::new_v4();

    let run_dir = RunDir::create(tmp.path(), run_id).unwrap();

    // Write manifest.
    let manifest = RunManifest {
        schema_version: CURRENT_SCHEMA_VERSION.to_string(),
        run_id,
        repo_root: PathBuf::from("/tmp/test"),
        base_ref: "HEAD".into(),
        task_prompt_hash: "abc123".into(),
        started_at: Utc::now(),
        completed_at: None,
        status: RunStatus::Running,
        agents: vec![],
    };
    run_dir.write_manifest(&manifest).unwrap();

    // Write events.
    run_dir
        .append_event(&RunEvent {
            timestamp: Utc::now(),
            run_id,
            event_type: EventType::RunStarted,
            agent_key: None,
            data: serde_json::json!({}),
        })
        .unwrap();

    run_dir
        .append_event(&RunEvent {
            timestamp: Utc::now(),
            run_id,
            event_type: EventType::RunCompleted,
            agent_key: None,
            data: serde_json::json!({"status": "ok"}),
        })
        .unwrap();

    // Read back and verify.
    let read_manifest = run_dir.read_manifest().unwrap();
    assert_eq!(read_manifest.run_id, run_id);
    assert_eq!(read_manifest.schema_version, CURRENT_SCHEMA_VERSION);

    let events = run_dir.read_events().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, EventType::RunStarted);
    assert_eq!(events[1].event_type, EventType::RunCompleted);
}

// ---------------------------------------------------------------------------
// Scoring reproducibility from artifacts
// ---------------------------------------------------------------------------

#[test]
fn scoring_is_deterministic_from_same_inputs() {
    use hydra_core::scoring::diff_scope::DiffStats;
    use hydra_core::scoring::lint::LintInput;
    use hydra_core::scoring::tests::TestInput;

    // Build scoring: deterministic.
    let build_1 = score_build(true);
    let build_2 = score_build(true);
    assert_eq!(
        build_1.score, build_2.score,
        "build score must be deterministic"
    );

    // Test scoring: deterministic.
    let test_input = TestInput {
        agent_passed: 48,
        agent_total: 50,
        baseline_passed: 50,
        baseline_total: 50,
    };
    let test_1 = score_tests(&test_input);
    let test_2 = score_tests(&test_input);
    assert_eq!(
        test_1.score, test_2.score,
        "test score must be deterministic"
    );

    // Lint scoring: deterministic.
    let lint_input = LintInput {
        agent_errors: 0,
        agent_warnings: 5,
        baseline_errors: 0,
        baseline_warnings: 2,
    };
    let lint_1 = score_lint(&lint_input);
    let lint_2 = score_lint(&lint_input);
    assert_eq!(
        lint_1.score, lint_2.score,
        "lint score must be deterministic"
    );

    // Speed scoring: deterministic.
    let speed_1 = score_speed(Duration::from_secs(5), Duration::from_secs(3));
    let speed_2 = score_speed(Duration::from_secs(5), Duration::from_secs(3));
    assert_eq!(
        speed_1.score, speed_2.score,
        "speed score must be deterministic"
    );

    // Diff scope scoring: deterministic.
    let diff_stats = DiffStats {
        lines_added: 50,
        lines_removed: 20,
        files_touched: 3,
        touched_paths: vec!["src/main.rs".into()],
    };
    let diff_config = DiffScopeConfig::default();
    let diff_1 = score_diff_scope(&diff_stats, &diff_config);
    let diff_2 = score_diff_scope(&diff_stats, &diff_config);
    assert_eq!(
        diff_1.score, diff_2.score,
        "diff score must be deterministic"
    );

    // Composite ranking: deterministic.
    let config = HydraConfig::default();
    let agent_input = AgentInput {
        agent_key: "claude".into(),
        build_score: Some(build_1.score),
        build_passed: true,
        test_score: Some(test_1.score),
        test_regression_percent: 0.0,
        lint_score: Some(lint_1.score),
        diff_scope_score: Some(diff_1.score),
        speed_score: Some(speed_1.score),
    };
    let agent_input_2 = AgentInput {
        agent_key: "claude".into(),
        build_score: Some(build_2.score),
        build_passed: true,
        test_score: Some(test_2.score),
        test_regression_percent: 0.0,
        lint_score: Some(lint_2.score),
        diff_scope_score: Some(diff_2.score),
        speed_score: Some(speed_2.score),
    };

    let result_1 = rank_agents(
        Uuid::nil(),
        vec![agent_input],
        &config.scoring.weights,
        &config.scoring.gates,
    );
    let result_2 = rank_agents(
        Uuid::nil(),
        vec![agent_input_2],
        &config.scoring.weights,
        &config.scoring.gates,
    );

    assert_eq!(
        result_1.rankings[0].total, result_2.rankings[0].total,
        "composite score must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// Config loading with all profiles
// ---------------------------------------------------------------------------

#[test]
fn all_scoring_profiles_load_successfully() {
    for profile in &["js-node", "rust", "python", "custom"] {
        let cfg = ScoringConfig::from_profile(profile);
        assert!(cfg.is_ok(), "profile '{profile}' should load successfully");
        cfg.unwrap();
    }
}

#[test]
fn default_config_validates() {
    let cfg = HydraConfig::default();
    cfg.validate().expect("default config should validate");
}

// ---------------------------------------------------------------------------
// Experimental adapter gating
// ---------------------------------------------------------------------------

#[test]
fn experimental_adapter_gating_default_off() {
    let cfg = HydraConfig::default();
    assert!(
        !cfg.general.allow_experimental_adapters,
        "experimental adapters should be off by default"
    );
}

// ---------------------------------------------------------------------------
// Workflow preset execution
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workflow_builder_reviewer_executes() {
    let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
    let wf = builder_reviewer_refiner("claude", "codex", "claude", "test task");
    let result = engine.execute(&wf).await.unwrap();
    assert_eq!(result.status, WorkflowStatus::Completed);
}

#[tokio::test]
async fn workflow_specialization_executes() {
    let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
    let wf = specialization(
        vec![
            ("frontend".into(), "claude".into(), "build UI".into()),
            ("backend".into(), "codex".into(), "build API".into()),
        ],
        Some("claude".into()),
    );
    let result = engine.execute(&wf).await.unwrap();
    assert_eq!(result.status, WorkflowStatus::Completed);
}

#[tokio::test]
async fn workflow_iterative_executes() {
    let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
    let wf = iterative_refinement("claude", "improve code", 2, 95.0);
    let result = engine.execute(&wf).await.unwrap();
    assert_eq!(result.status, WorkflowStatus::Completed);
}

// ---------------------------------------------------------------------------
// Merge dry-run and confirm flow (via report serialization)
// ---------------------------------------------------------------------------

#[test]
fn merge_report_round_trips() {
    use hydra_core::merge::{ConflictFile, MergeReport};

    let report = MergeReport {
        source_branch: "hydra/abc/agent/claude".into(),
        target_branch: "main".into(),
        dry_run: true,
        can_merge: true,
        conflicts: vec![],
        files_changed: 5,
        insertions: 100,
        deletions: 20,
    };

    let json = serde_json::to_string_pretty(&report).unwrap();
    let deser: MergeReport = serde_json::from_str(&json).unwrap();
    assert!(deser.can_merge);
    assert_eq!(deser.files_changed, 5);

    // With conflicts.
    let report_with_conflicts = MergeReport {
        source_branch: "feature".into(),
        target_branch: "main".into(),
        dry_run: true,
        can_merge: false,
        conflicts: vec![ConflictFile {
            path: "src/main.rs".into(),
            conflict_type: "content".into(),
        }],
        files_changed: 3,
        insertions: 10,
        deletions: 5,
    };
    let json = serde_json::to_string(&report_with_conflicts).unwrap();
    let deser: MergeReport = serde_json::from_str(&json).unwrap();
    assert!(!deser.can_merge);
    assert_eq!(deser.conflicts.len(), 1);
}

// ---------------------------------------------------------------------------
// Schema migration
// ---------------------------------------------------------------------------

#[test]
fn schema_version_parsing_and_compatibility() {
    let v1 = SchemaVersion::parse(CURRENT_SCHEMA_VERSION).unwrap();
    assert_eq!(v1.to_string(), CURRENT_SCHEMA_VERSION);

    // Current is compatible with itself.
    assert!(v1.is_compatible_with(&v1));
}

#[test]
fn migration_tool_no_op_for_current_version() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = serde_json::json!({
        "schema_version": CURRENT_SCHEMA_VERSION,
        "run_id": Uuid::new_v4().to_string(),
    });
    std::fs::write(
        tmp.path().join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let report = MigrationTool::migrate(tmp.path()).unwrap();
    assert_eq!(report.files_migrated, 0);
}

// ---------------------------------------------------------------------------
// Recovery service
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recovery_scan_and_cleanup_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let svc = RecoveryService::new(tmp.path());

    let stale = svc.scan_stale_runs().await.unwrap();
    assert!(stale.is_empty());

    let report = svc.cleanup_all().await.unwrap();
    assert_eq!(report.runs_cleaned, 0);
}

// ---------------------------------------------------------------------------
// Platform paths
// ---------------------------------------------------------------------------

#[test]
fn path_validation_accepts_normal_paths() {
    use hydra_core::platform::paths::{check_unicode_safety, validate_artifact_path};
    use std::path::Path;

    assert!(validate_artifact_path(Path::new("/tmp/hydra/runs/abc/manifest.json")).is_ok());
    assert!(check_unicode_safety(Path::new("/tmp/hydra/test")));
}

#[test]
fn path_validation_rejects_null_bytes() {
    use hydra_core::platform::paths::validate_artifact_path;
    use std::path::Path;

    let result = validate_artifact_path(Path::new("foo\0bar"));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Compilation smoke test
// ---------------------------------------------------------------------------

#[test]
fn hydra_core_compiles_on_current_platform() {
    // If this test runs, hydra-core compiled successfully.
    // Reference a function to ensure the crate is linked.
    let _f: fn() = hydra_core::init_tracing;
}
