//! Experimental adapter gating.
//!
//! Validates that experimental adapter usage is properly gated behind
//! explicit opt-in flags.

use serde::{Deserialize, Serialize};

use super::probe::AdapterTier;
use crate::{HydraError, Result};

/// Warning emitted when an experimental adapter is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalWarning {
    pub adapter_key: String,
    pub message: String,
}

/// Known experimental adapter keys.
const EXPERIMENTAL_ADAPTERS: &[&str] = &["cursor-agent"];

/// Check if a given adapter key is experimental.
pub fn is_experimental(adapter_key: &str) -> bool {
    EXPERIMENTAL_ADAPTERS.contains(&adapter_key)
}

/// Format a label for experimental adapters in CLI output.
pub fn experimental_label(adapter_key: &str) -> String {
    if is_experimental(adapter_key) {
        format!("[EXPERIMENTAL] {adapter_key}")
    } else {
        adapter_key.to_string()
    }
}

/// Validate that experimental adapter usage is properly gated.
///
/// If any of the `requested_adapters` is experimental and
/// `allow_experimental` is `false`, returns an error.
///
/// If experimental adapters are allowed, returns warnings for each
/// experimental adapter that was requested.
pub fn validate_experimental_usage(
    requested_adapters: &[String],
    allow_experimental: bool,
) -> Result<Vec<ExperimentalWarning>> {
    let experimental_requested: Vec<&String> = requested_adapters
        .iter()
        .filter(|key| is_experimental(key))
        .collect();

    if experimental_requested.is_empty() {
        return Ok(Vec::new());
    }

    if !allow_experimental {
        let keys: Vec<&str> = experimental_requested.iter().map(|s| s.as_str()).collect();
        return Err(HydraError::Adapter(format!(
            "experimental adapter(s) requested [{}] but --allow-experimental-adapters flag is not set",
            keys.join(", ")
        )));
    }

    let warnings: Vec<ExperimentalWarning> = experimental_requested
        .into_iter()
        .map(|key| ExperimentalWarning {
            adapter_key: key.clone(),
            message: format!(
                "{key} is an experimental adapter. Output quality and reliability are not guaranteed."
            ),
        })
        .collect();

    Ok(warnings)
}

/// Classify an adapter key by tier.
pub fn adapter_tier(adapter_key: &str) -> AdapterTier {
    if is_experimental(adapter_key) {
        AdapterTier::Experimental
    } else {
        AdapterTier::Tier1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_is_experimental() {
        assert!(is_experimental("cursor-agent"));
    }

    #[test]
    fn claude_is_not_experimental() {
        assert!(!is_experimental("claude"));
    }

    #[test]
    fn codex_is_not_experimental() {
        assert!(!is_experimental("codex"));
    }

    #[test]
    fn validate_no_experimental_requested() {
        let adapters = vec!["claude".to_string(), "codex".to_string()];
        let warnings = validate_experimental_usage(&adapters, false).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_experimental_blocked() {
        let adapters = vec!["claude".to_string(), "cursor-agent".to_string()];
        let result = validate_experimental_usage(&adapters, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("experimental"));
        assert!(err.contains("cursor-agent"));
    }

    #[test]
    fn validate_experimental_allowed() {
        let adapters = vec!["claude".to_string(), "cursor-agent".to_string()];
        let warnings = validate_experimental_usage(&adapters, true).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].adapter_key, "cursor-agent");
        assert!(warnings[0].message.contains("experimental"));
    }

    #[test]
    fn experimental_label_for_cursor() {
        let label = experimental_label("cursor-agent");
        assert!(label.starts_with("[EXPERIMENTAL]"));
        assert!(label.contains("cursor-agent"));
    }

    #[test]
    fn experimental_label_for_tier1() {
        let label = experimental_label("claude");
        assert_eq!(label, "claude");
    }

    #[test]
    fn adapter_tier_classification() {
        assert_eq!(adapter_tier("cursor-agent"), AdapterTier::Experimental);
        assert_eq!(adapter_tier("claude"), AdapterTier::Tier1);
        assert_eq!(adapter_tier("codex"), AdapterTier::Tier1);
    }

    #[test]
    fn empty_adapter_list() {
        let warnings = validate_experimental_usage(&[], false).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn warning_serialization() {
        let warning = ExperimentalWarning {
            adapter_key: "cursor-agent".to_string(),
            message: "test warning".to_string(),
        };
        let json = serde_json::to_string(&warning).expect("serialize");
        let deser: ExperimentalWarning = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.adapter_key, "cursor-agent");
    }
}
