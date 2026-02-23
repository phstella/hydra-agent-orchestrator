use serde::{Deserialize, Serialize};

/// Top-level configuration loaded from `hydra.toml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HydraConfig {
    pub scoring: ScoringConfig,
    pub adapters: AdaptersConfig,
    pub worktree: WorktreeConfig,
    pub supervisor: SupervisorConfig,
}

/// Scoring configuration: profile, weights, gates, timeouts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ScoringConfig {
    pub profile: Option<ScoringProfile>,
    pub timeout_per_check_seconds: u64,
    pub weights: WeightsConfig,
    pub gates: GatesConfig,
    pub diff_scope: DiffScopeConfig,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            profile: None,
            timeout_per_check_seconds: 300,
            weights: WeightsConfig::default(),
            gates: GatesConfig::default(),
            diff_scope: DiffScopeConfig::default(),
        }
    }
}

/// Language/repo profile preset that sets build/test/lint commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScoringProfile {
    JsNode,
    Rust,
    Python,
}

/// Scoring dimension weights (should sum to a positive value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WeightsConfig {
    pub build: u32,
    pub tests: u32,
    pub lint: u32,
    pub diff_scope: u32,
    pub speed: u32,
}

impl Default for WeightsConfig {
    fn default() -> Self {
        Self {
            build: 30,
            tests: 30,
            lint: 15,
            diff_scope: 15,
            speed: 10,
        }
    }
}

/// Mergeability gates applied before ranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GatesConfig {
    pub require_build_pass: bool,
    pub max_test_regression_percent: f64,
}

impl Default for GatesConfig {
    fn default() -> Self {
        Self {
            require_build_pass: true,
            max_test_regression_percent: 0.0,
        }
    }
}

/// Diff scope scoring configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DiffScopeConfig {
    pub max_files_soft: u32,
    pub max_churn_soft: u32,
    pub protected_paths: Vec<String>,
}

impl Default for DiffScopeConfig {
    fn default() -> Self {
        Self {
            max_files_soft: 20,
            max_churn_soft: 800,
            protected_paths: Vec::new(),
        }
    }
}

/// Adapter binary path overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AdaptersConfig {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub cursor: Option<String>,
}

/// Worktree management configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WorktreeConfig {
    pub base_dir: String,
    pub retain: RetentionPolicy,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            base_dir: ".hydra/worktrees".to_string(),
            retain: RetentionPolicy::Failed,
        }
    }
}

/// Worktree retention policy after run completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    None,
    Failed,
    All,
}

/// Process supervisor configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SupervisorConfig {
    pub hard_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub output_buffer_bytes: usize,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            hard_timeout_seconds: 1800,
            idle_timeout_seconds: 300,
            output_buffer_bytes: 10 * 1024 * 1024, // 10 MiB
        }
    }
}
