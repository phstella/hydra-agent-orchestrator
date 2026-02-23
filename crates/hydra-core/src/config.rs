use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::warn;

use crate::{HydraError, Result};

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HydraConfig {
    pub general: GeneralConfig,
    pub scoring: ScoringConfig,
    pub adapters: AdaptersConfig,
    pub retention: RetentionConfig,
    pub budget: BudgetConfig,
}

impl HydraConfig {
    /// Load config from a specific `hydra.toml` file path.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| HydraError::Config(format!("failed to read {}: {e}", path.display())))?;
        let config: Self = toml::from_str(&contents)
            .map_err(|e| HydraError::Config(format!("failed to parse {}: {e}", path.display())))?;
        config.validate()?;
        Ok(config)
    }

    /// Load from `./hydra.toml` if it exists, otherwise return defaults.
    pub fn load_or_default() -> Self {
        let path = PathBuf::from("hydra.toml");
        if path.exists() {
            match Self::load(&path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    warn!(%e, "failed to load hydra.toml, falling back to defaults");
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    /// Validate the config and return actionable error messages.
    pub fn validate(&self) -> Result<()> {
        // Timeout sanity
        if self.general.default_timeout_seconds == 0 {
            return Err(HydraError::Config(
                "general.default_timeout_seconds must be > 0".into(),
            ));
        }
        if self.general.hard_timeout_seconds == 0 {
            return Err(HydraError::Config(
                "general.hard_timeout_seconds must be > 0".into(),
            ));
        }
        if self.general.idle_timeout_seconds == 0 {
            return Err(HydraError::Config(
                "general.idle_timeout_seconds must be > 0".into(),
            ));
        }
        if self.general.hard_timeout_seconds <= self.general.default_timeout_seconds {
            return Err(HydraError::Config(
                "general.hard_timeout_seconds must be greater than general.default_timeout_seconds"
                    .into(),
            ));
        }

        // Scoring weights
        let w = &self.scoring.weights;
        let total = w.build + w.tests + w.lint + w.diff_scope + w.speed;
        if total != 100 {
            warn!(
                total,
                "scoring weights sum to {total}, not 100 -- scores may not behave as expected"
            );
        }

        // Profile name
        let valid_profiles = ["js-node", "rust", "python", "custom"];
        if !valid_profiles.contains(&self.scoring.profile.as_str()) {
            return Err(HydraError::Config(format!(
                "scoring.profile '{}' is not recognised; valid profiles: {}",
                self.scoring.profile,
                valid_profiles.join(", ")
            )));
        }

        // Scoring timeout
        if self.scoring.timeout_per_check_seconds == 0 {
            return Err(HydraError::Config(
                "scoring.timeout_per_check_seconds must be > 0".into(),
            ));
        }

        // Protected paths: reject empty strings
        for (i, p) in self.scoring.diff_scope.protected_paths.iter().enumerate() {
            if p.trim().is_empty() {
                return Err(HydraError::Config(format!(
                    "scoring.diff_scope.protected_paths[{i}] is empty"
                )));
            }
        }

        // Retention policy
        let valid_policies = ["none", "failed", "all"];
        if !valid_policies.contains(&self.retention.policy.as_str()) {
            return Err(HydraError::Config(format!(
                "retention.policy '{}' is not recognised; valid policies: {}",
                self.retention.policy,
                valid_policies.join(", ")
            )));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GeneralConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Default timeout for agent execution in seconds.
    pub default_timeout_seconds: u64,
    /// Hard timeout that kills the process.
    pub hard_timeout_seconds: u64,
    /// Idle timeout (no output) before killing agent.
    pub idle_timeout_seconds: u64,
    /// Allow experimental adapters in runs.
    pub allow_experimental_adapters: bool,
    /// Enable unsafe mode (allow writes outside worktree).
    pub unsafe_mode: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_timeout_seconds: 600,
            hard_timeout_seconds: 900,
            idle_timeout_seconds: 120,
            allow_experimental_adapters: false,
            unsafe_mode: false,
        }
    }
}

// ---------------------------------------------------------------------------
// ScoringConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringConfig {
    /// Language/repo profile preset: "js-node", "rust", "python", or "custom".
    pub profile: String,
    /// Timeout per scoring check in seconds.
    pub timeout_per_check_seconds: u64,
    pub weights: ScoringWeights,
    pub gates: ScoringGates,
    pub diff_scope: DiffScopeConfig,
    pub commands: ScoringCommands,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            profile: "custom".into(),
            timeout_per_check_seconds: 300,
            weights: ScoringWeights::default(),
            gates: ScoringGates::default(),
            diff_scope: DiffScopeConfig::default(),
            commands: ScoringCommands::default(),
        }
    }
}

impl ScoringConfig {
    /// Build a `ScoringConfig` from a named profile preset.
    ///
    /// Recognised profiles: `"js-node"`, `"rust"`, `"python"`, `"custom"`.
    pub fn from_profile(name: &str) -> Result<Self> {
        let mut cfg = Self {
            profile: name.into(),
            ..Self::default()
        };

        match name {
            "js-node" => {
                cfg.commands.build = Some("npm run build".into());
                cfg.commands.test = Some("npm test -- --runInBand".into());
                cfg.commands.lint = Some("npm run lint".into());
            }
            "rust" => {
                cfg.commands.build = Some("cargo build --all-targets".into());
                cfg.commands.test = Some("cargo test".into());
                cfg.commands.lint = Some("cargo clippy --all-targets -- -D warnings".into());
            }
            "python" => {
                cfg.commands.build = None;
                cfg.commands.test = Some("pytest -q".into());
                cfg.commands.lint = Some("ruff check .".into());
            }
            "custom" => { /* no preset commands */ }
            other => {
                return Err(HydraError::Config(format!(
                    "unknown scoring profile '{other}'; valid profiles: js-node, rust, python, custom"
                )));
            }
        }

        Ok(cfg)
    }
}

// ---------------------------------------------------------------------------
// Scoring sub-structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringWeights {
    pub build: u32,
    pub tests: u32,
    pub lint: u32,
    pub diff_scope: u32,
    pub speed: u32,
}

impl Default for ScoringWeights {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringGates {
    pub require_build_pass: bool,
    pub max_test_regression_percent: f64,
}

impl Default for ScoringGates {
    fn default() -> Self {
        Self {
            require_build_pass: true,
            max_test_regression_percent: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScoringCommands {
    pub build: Option<String>,
    pub test: Option<String>,
    pub lint: Option<String>,
}

// ---------------------------------------------------------------------------
// AdaptersConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AdaptersConfig {
    pub claude: AdapterConfig,
    pub codex: AdapterConfig,
    pub cursor: AdapterConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AdapterConfig {
    pub enabled: Option<bool>,
    pub binary_path: Option<PathBuf>,
    pub extra_args: Vec<String>,
}

// ---------------------------------------------------------------------------
// RetentionConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetentionConfig {
    /// Retention policy: "none", "failed", or "all".
    pub policy: String,
    pub max_age_days: Option<u64>,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            policy: "failed".into(),
            max_age_days: None,
        }
    }
}

// ---------------------------------------------------------------------------
// BudgetConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetConfig {
    pub max_tokens_total: Option<u64>,
    pub max_cost_usd: Option<f64>,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Full, valid hydra.toml.
    const FULL_TOML: &str = r#"
[general]
default_timeout_seconds = 300
hard_timeout_seconds = 600
idle_timeout_seconds = 60
allow_experimental_adapters = true
unsafe_mode = false

[scoring]
profile = "rust"
timeout_per_check_seconds = 120

[scoring.weights]
build = 30
tests = 30
lint = 15
diff_scope = 15
speed = 10

[scoring.gates]
require_build_pass = true
max_test_regression_percent = 5.0

[scoring.diff_scope]
max_files_soft = 15
max_churn_soft = 500
protected_paths = ["Cargo.lock"]

[scoring.commands]
build = "cargo build"
test = "cargo test"
lint = "cargo clippy"

[adapters.claude]
enabled = true
binary_path = "/usr/local/bin/claude"
extra_args = ["--verbose"]

[adapters.codex]
enabled = false

[adapters.cursor]
enabled = false

[retention]
policy = "all"
max_age_days = 30

[budget]
max_tokens_total = 100000
max_cost_usd = 10.0
"#;

    #[test]
    fn parse_full_toml() {
        let cfg: HydraConfig = toml::from_str(FULL_TOML).expect("parse full TOML");
        assert_eq!(cfg.general.default_timeout_seconds, 300);
        assert_eq!(cfg.general.hard_timeout_seconds, 600);
        assert!(cfg.general.allow_experimental_adapters);
        assert_eq!(cfg.scoring.profile, "rust");
        assert_eq!(cfg.scoring.weights.build, 30);
        assert_eq!(cfg.scoring.gates.max_test_regression_percent, 5.0);
        assert_eq!(
            cfg.scoring.diff_scope.protected_paths,
            vec!["Cargo.lock".to_string()]
        );
        assert_eq!(cfg.adapters.claude.enabled, Some(true));
        assert_eq!(
            cfg.adapters.claude.binary_path.as_deref(),
            Some(Path::new("/usr/local/bin/claude"))
        );
        assert_eq!(cfg.retention.policy, "all");
        assert_eq!(cfg.retention.max_age_days, Some(30));
        assert_eq!(cfg.budget.max_tokens_total, Some(100_000));
        cfg.validate().expect("full config is valid");
    }

    #[test]
    fn parse_minimal_toml_gets_defaults() {
        let cfg: HydraConfig = toml::from_str("").expect("parse empty TOML");
        assert_eq!(cfg.general.default_timeout_seconds, 600);
        assert_eq!(cfg.general.hard_timeout_seconds, 900);
        assert_eq!(cfg.general.idle_timeout_seconds, 120);
        assert!(!cfg.general.allow_experimental_adapters);
        assert!(!cfg.general.unsafe_mode);
        assert_eq!(cfg.scoring.profile, "custom");
        assert_eq!(cfg.scoring.weights.build, 30);
        assert_eq!(cfg.scoring.weights.tests, 30);
        assert_eq!(cfg.scoring.weights.lint, 15);
        assert_eq!(cfg.scoring.weights.diff_scope, 15);
        assert_eq!(cfg.scoring.weights.speed, 10);
        assert!(cfg.scoring.gates.require_build_pass);
        assert_eq!(cfg.scoring.gates.max_test_regression_percent, 0.0);
        assert_eq!(cfg.scoring.timeout_per_check_seconds, 300);
        assert_eq!(cfg.scoring.diff_scope.max_files_soft, 20);
        assert_eq!(cfg.scoring.diff_scope.max_churn_soft, 800);
        assert_eq!(cfg.retention.policy, "failed");
        assert!(cfg.budget.max_tokens_total.is_none());
        cfg.validate().expect("default config is valid");
    }

    #[test]
    fn validate_rejects_zero_timeout() {
        let mut cfg = HydraConfig::default();
        cfg.general.default_timeout_seconds = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("default_timeout_seconds"));
    }

    #[test]
    fn validate_rejects_hard_lte_default() {
        let mut cfg = HydraConfig::default();
        cfg.general.hard_timeout_seconds = cfg.general.default_timeout_seconds;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("hard_timeout_seconds"));
    }

    #[test]
    fn validate_rejects_unknown_profile() {
        let mut cfg = HydraConfig::default();
        cfg.scoring.profile = "unknown".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn validate_rejects_empty_protected_path() {
        let mut cfg = HydraConfig::default();
        cfg.scoring.diff_scope.protected_paths = vec!["  ".into()];
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("protected_paths"));
    }

    #[test]
    fn validate_rejects_unknown_retention_policy() {
        let mut cfg = HydraConfig::default();
        cfg.retention.policy = "forever".into();
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("forever"));
    }

    #[test]
    fn profile_js_node() {
        let cfg = ScoringConfig::from_profile("js-node").unwrap();
        assert_eq!(cfg.profile, "js-node");
        assert_eq!(cfg.commands.build.as_deref(), Some("npm run build"));
        assert_eq!(
            cfg.commands.test.as_deref(),
            Some("npm test -- --runInBand")
        );
        assert_eq!(cfg.commands.lint.as_deref(), Some("npm run lint"));
    }

    #[test]
    fn profile_rust() {
        let cfg = ScoringConfig::from_profile("rust").unwrap();
        assert_eq!(
            cfg.commands.build.as_deref(),
            Some("cargo build --all-targets")
        );
        assert_eq!(cfg.commands.test.as_deref(), Some("cargo test"));
        assert_eq!(
            cfg.commands.lint.as_deref(),
            Some("cargo clippy --all-targets -- -D warnings")
        );
    }

    #[test]
    fn profile_python() {
        let cfg = ScoringConfig::from_profile("python").unwrap();
        assert!(cfg.commands.build.is_none());
        assert_eq!(cfg.commands.test.as_deref(), Some("pytest -q"));
        assert_eq!(cfg.commands.lint.as_deref(), Some("ruff check ."));
    }

    #[test]
    fn profile_custom_has_no_commands() {
        let cfg = ScoringConfig::from_profile("custom").unwrap();
        assert!(cfg.commands.build.is_none());
        assert!(cfg.commands.test.is_none());
        assert!(cfg.commands.lint.is_none());
    }

    #[test]
    fn profile_unknown_errors() {
        let err = ScoringConfig::from_profile("go").unwrap_err();
        assert!(err.to_string().contains("go"));
    }

    #[test]
    fn default_config_is_valid() {
        HydraConfig::default().validate().expect("default is valid");
    }

    #[test]
    fn serialization_round_trip() {
        let original = HydraConfig::default();
        let toml_str = toml::to_string_pretty(&original).expect("serialize");
        let restored: HydraConfig = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(
            original.general.default_timeout_seconds,
            restored.general.default_timeout_seconds
        );
        assert_eq!(original.scoring.profile, restored.scoring.profile);
        assert_eq!(
            original.scoring.weights.build,
            restored.scoring.weights.build
        );
        assert_eq!(original.retention.policy, restored.retention.policy);

        restored.validate().expect("round-tripped config is valid");
    }

    #[test]
    fn parse_partial_toml_fills_defaults() {
        let partial = r#"
[general]
default_timeout_seconds = 400

[scoring]
profile = "rust"
"#;
        let cfg: HydraConfig = toml::from_str(partial).expect("parse partial");
        assert_eq!(cfg.general.default_timeout_seconds, 400);
        // Rest should be defaults
        assert_eq!(cfg.general.hard_timeout_seconds, 900);
        assert_eq!(cfg.general.idle_timeout_seconds, 120);
        assert_eq!(cfg.scoring.profile, "rust");
        assert_eq!(cfg.scoring.weights.build, 30);
        assert_eq!(cfg.retention.policy, "failed");
        cfg.validate().expect("partial config is valid");
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let bad = "this is not [valid toml";
        let result = toml::from_str::<HydraConfig>(bad);
        assert!(result.is_err());
    }

    #[test]
    fn validate_rejects_zero_scoring_timeout() {
        let mut cfg = HydraConfig::default();
        cfg.scoring.timeout_per_check_seconds = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("timeout_per_check_seconds"));
    }
}
