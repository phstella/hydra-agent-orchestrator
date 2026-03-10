use std::path::Path;

use thiserror::Error;

mod schema;

pub use schema::{
    AdaptersConfig, BudgetConfig, CommandsConfig, DiffScopeConfig, GatesConfig, HydraConfig,
    RetentionPolicy, ScoringConfig, ScoringProfile, SupervisorConfig, WeightsConfig,
    WorktreeConfig,
};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file '{path}': {source}")]
    ReadFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to parse config: {0}")]
    ParseFailed(#[from] toml::de::Error),

    #[error("config validation error: {message}")]
    Validation { message: String },
}

/// Load and validate a `HydraConfig` from a TOML file path.
///
/// Returns the default config if the file does not exist.
pub fn load_config(path: &Path) -> Result<HydraConfig, ConfigError> {
    if !path.exists() {
        tracing::debug!(path = %path.display(), "config file not found, using defaults");
        return Ok(HydraConfig::default());
    }

    let data = std::fs::read_to_string(path).map_err(|e| ConfigError::ReadFailed {
        path: path.display().to_string(),
        source: e,
    })?;

    parse_config(&data)
}

/// Parse and validate a `HydraConfig` from a TOML string.
pub fn parse_config(data: &str) -> Result<HydraConfig, ConfigError> {
    let config: HydraConfig = toml::from_str(data)?;
    validate(&config)?;
    Ok(config)
}

fn validate(config: &HydraConfig) -> Result<(), ConfigError> {
    let w = &config.scoring.weights;
    let total = w.build + w.tests + w.lint + w.diff_scope + w.speed;
    if total == 0 {
        return Err(ConfigError::Validation {
            message: "scoring weights must not all be zero".to_string(),
        });
    }

    if config.scoring.gates.max_test_regression_percent > 100.0 {
        return Err(ConfigError::Validation {
            message: format!(
                "max_test_regression_percent must be 0..=100, got {}",
                config.scoring.gates.max_test_regression_percent
            ),
        });
    }

    if config.supervisor.hard_timeout_seconds == 0 {
        return Err(ConfigError::Validation {
            message: "supervisor.hard_timeout_seconds must be > 0".to_string(),
        });
    }

    if config.supervisor.idle_timeout_seconds == 0 {
        return Err(ConfigError::Validation {
            message: "supervisor.idle_timeout_seconds must be > 0".to_string(),
        });
    }

    if let Some(max_tokens_total) = config.scoring.budget.max_tokens_total {
        if max_tokens_total == 0 {
            return Err(ConfigError::Validation {
                message: "scoring.budget.max_tokens_total must be > 0".to_string(),
            });
        }
    }

    if let Some(max_cost_usd) = config.scoring.budget.max_cost_usd {
        if !max_cost_usd.is_finite() || max_cost_usd < 0.0 {
            return Err(ConfigError::Validation {
                message: "scoring.budget.max_cost_usd must be a finite number >= 0".to_string(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_passes_validation() {
        let config = HydraConfig::default();
        validate(&config).unwrap();
    }

    #[test]
    fn minimal_toml_parses_to_defaults() {
        let config = parse_config("").unwrap();
        assert_eq!(config.scoring.weights.build, 30);
        assert_eq!(config.scoring.weights.tests, 30);
        assert_eq!(config.scoring.weights.lint, 15);
        assert_eq!(config.scoring.weights.diff_scope, 15);
        assert_eq!(config.scoring.weights.speed, 10);
        assert!(config.scoring.gates.require_build_pass);
    }

    #[test]
    fn full_example_config_parses() {
        let data = r#"
[scoring]
profile = "js-node"
timeout_per_check_seconds = 300

[scoring.weights]
build = 30
tests = 30
lint = 15
diff_scope = 15
speed = 10

[scoring.gates]
require_build_pass = true
max_test_regression_percent = 0.0

[scoring.diff_scope]
max_files_soft = 20
max_churn_soft = 800
protected_paths = ["infra/", "scripts/release/"]

[adapters]
claude = "/opt/claude"
codex = "/opt/codex"
cursor = "/opt/cursor-agent"

[worktree]
base_dir = ".hydra/worktrees"
retain = "failed"

[supervisor]
hard_timeout_seconds = 1800
idle_timeout_seconds = 300
output_buffer_bytes = 10485760
"#;

        let config = parse_config(data).unwrap();
        assert_eq!(config.scoring.profile, Some(ScoringProfile::JsNode));
        assert_eq!(config.scoring.timeout_per_check_seconds, 300);
        assert_eq!(config.scoring.diff_scope.max_files_soft, 20);
        assert_eq!(config.scoring.diff_scope.max_churn_soft, 800);
        assert_eq!(
            config.scoring.diff_scope.protected_paths,
            vec!["infra/", "scripts/release/"]
        );
        assert_eq!(config.adapters.claude.as_deref(), Some("/opt/claude"));
        assert_eq!(config.adapters.codex.as_deref(), Some("/opt/codex"));
        assert_eq!(config.adapters.cursor.as_deref(), Some("/opt/cursor-agent"));
        assert_eq!(config.worktree.retain, RetentionPolicy::Failed);
        assert_eq!(config.supervisor.hard_timeout_seconds, 1800);
        assert_eq!(config.supervisor.idle_timeout_seconds, 300);
        assert_eq!(config.supervisor.output_buffer_bytes, 10_485_760);
    }

    #[test]
    fn partial_config_fills_defaults() {
        let data = r#"
[scoring.weights]
build = 50
"#;
        let config = parse_config(data).unwrap();
        assert_eq!(config.scoring.weights.build, 50);
        assert_eq!(config.scoring.weights.tests, 30);
        assert_eq!(config.scoring.weights.lint, 15);
    }

    #[test]
    fn zero_weights_rejected() {
        let data = r#"
[scoring.weights]
build = 0
tests = 0
lint = 0
diff_scope = 0
speed = 0
"#;
        let err = parse_config(data).unwrap_err();
        assert!(err.to_string().contains("must not all be zero"));
    }

    #[test]
    fn invalid_regression_percent_rejected() {
        let data = r#"
[scoring.gates]
max_test_regression_percent = 150.0
"#;
        let err = parse_config(data).unwrap_err();
        assert!(err.to_string().contains("max_test_regression_percent"));
    }

    #[test]
    fn unknown_field_in_toml_returns_parse_error() {
        let data = r#"
[scoring]
nonexistent_field = "bad"
"#;
        let err = parse_config(data).unwrap_err();
        assert!(matches!(err, ConfigError::ParseFailed(_)));
    }

    #[test]
    fn missing_config_file_returns_defaults() {
        let config = load_config(Path::new("/tmp/nonexistent-hydra-test.toml")).unwrap();
        assert_eq!(config, HydraConfig::default());
    }

    #[test]
    fn zero_hard_timeout_rejected() {
        let data = r#"
[supervisor]
hard_timeout_seconds = 0
"#;
        let err = parse_config(data).unwrap_err();
        assert!(err.to_string().contains("hard_timeout_seconds"));
    }

    #[test]
    fn retention_policy_variants_parse() {
        for (input, expected) in [
            (r#"[worktree]\nretain = "none""#, RetentionPolicy::None),
            (r#"[worktree]\nretain = "failed""#, RetentionPolicy::Failed),
            (r#"[worktree]\nretain = "all""#, RetentionPolicy::All),
        ] {
            let data = input.replace("\\n", "\n");
            let config = parse_config(&data).unwrap();
            assert_eq!(config.worktree.retain, expected);
        }
    }

    #[test]
    fn scoring_profiles_parse() {
        for (input, expected) in [
            ("js-node", ScoringProfile::JsNode),
            ("rust", ScoringProfile::Rust),
            ("python", ScoringProfile::Python),
        ] {
            let data = format!("[scoring]\nprofile = \"{input}\"");
            let config = parse_config(&data).unwrap();
            assert_eq!(config.scoring.profile, Some(expected));
        }
    }

    #[test]
    fn budget_fields_parse() {
        let data = r#"
[scoring.budget]
max_tokens_total = 12345
max_cost_usd = 4.5
"#;
        let config = parse_config(data).unwrap();
        assert_eq!(config.scoring.budget.max_tokens_total, Some(12_345));
        assert_eq!(config.scoring.budget.max_cost_usd, Some(4.5));
    }

    #[test]
    fn zero_max_tokens_budget_rejected() {
        let data = r#"
[scoring.budget]
max_tokens_total = 0
"#;
        let err = parse_config(data).unwrap_err();
        assert!(err.to_string().contains("max_tokens_total"));
    }

    #[test]
    fn negative_max_cost_budget_rejected() {
        let data = r#"
[scoring.budget]
max_cost_usd = -0.1
"#;
        let err = parse_config(data).unwrap_err();
        assert!(err.to_string().contains("max_cost_usd"));
    }
}
