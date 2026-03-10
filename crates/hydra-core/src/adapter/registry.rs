use std::sync::Arc;

use thiserror::Error;

use super::claude::ClaudeAdapter;
use super::codex::CodexAdapter;
use super::cursor::CursorAdapter;
use super::types::AdapterTier;
use super::AgentAdapter;
use crate::config::AdaptersConfig;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("unknown adapter '{key}'. Known adapters: {known}")]
    UnknownAdapter { key: String, known: String },

    #[error("adapter '{key}' is experimental and requires --allow-experimental-adapters to use")]
    ExperimentalBlocked { key: String },
}

/// Central registry of all known agent adapters with tier-policy enforcement.
pub struct AdapterRegistry {
    adapters: Vec<Arc<dyn AgentAdapter>>,
}

impl AdapterRegistry {
    pub fn from_config(config: &AdaptersConfig) -> Self {
        let adapters: Vec<Arc<dyn AgentAdapter>> = vec![
            Arc::new(ClaudeAdapter::new(config.claude.clone())),
            Arc::new(CodexAdapter::new(config.codex.clone())),
            Arc::new(CursorAdapter::new(config.cursor.clone())),
        ];
        Self { adapters }
    }

    pub fn new(adapters: Vec<Arc<dyn AgentAdapter>>) -> Self {
        Self { adapters }
    }

    /// Resolve a single adapter by key, enforcing tier policy.
    pub fn resolve(
        &self,
        key: &str,
        allow_experimental: bool,
    ) -> Result<Arc<dyn AgentAdapter>, RegistryError> {
        let adapter = self
            .adapters
            .iter()
            .find(|a| a.key() == key)
            .ok_or_else(|| RegistryError::UnknownAdapter {
                key: key.to_string(),
                known: self.known_keys().join(", "),
            })?;

        if adapter.tier() == AdapterTier::Experimental && !allow_experimental {
            return Err(RegistryError::ExperimentalBlocked {
                key: key.to_string(),
            });
        }

        Ok(Arc::clone(adapter))
    }

    /// Resolve multiple adapters by key, enforcing tier policy on each.
    pub fn resolve_many(
        &self,
        keys: &[String],
        allow_experimental: bool,
    ) -> Result<Vec<Arc<dyn AgentAdapter>>, RegistryError> {
        keys.iter()
            .map(|k| self.resolve(k, allow_experimental))
            .collect()
    }

    /// Return all Tier-1 adapters (regardless of probe status).
    pub fn tier1(&self) -> Vec<Arc<dyn AgentAdapter>> {
        self.adapters
            .iter()
            .filter(|a| a.tier() == AdapterTier::Tier1)
            .cloned()
            .collect()
    }

    /// Return all adapters whose probe reports them as available,
    /// filtering experimental unless allowed.
    pub fn available(&self, allow_experimental: bool) -> Vec<Arc<dyn AgentAdapter>> {
        self.adapters
            .iter()
            .filter(|a| {
                if a.tier() == AdapterTier::Experimental && !allow_experimental {
                    return false;
                }
                a.detect().status.is_available()
            })
            .cloned()
            .collect()
    }

    /// All known adapter keys.
    pub fn known_keys(&self) -> Vec<&str> {
        self.adapters.iter().map(|a| a.key()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::types::*;

    struct MockAdapter {
        name: &'static str,
        adapter_tier: AdapterTier,
    }

    impl AgentAdapter for MockAdapter {
        fn key(&self) -> &'static str {
            self.name
        }

        fn tier(&self) -> AdapterTier {
            self.adapter_tier
        }

        fn detect(&self) -> DetectResult {
            DetectResult {
                status: match self.adapter_tier {
                    AdapterTier::Tier1 => DetectStatus::Ready,
                    AdapterTier::Experimental => DetectStatus::ExperimentalReady,
                },
                binary_path: None,
                version: None,
                supported_flags: vec![],
                confidence: CapabilityConfidence::Verified,
                error: None,
            }
        }

        fn capabilities(&self) -> CapabilitySet {
            CapabilitySet {
                json_stream: CapabilityEntry::unknown(),
                plain_text: CapabilityEntry::unknown(),
                force_edit_mode: CapabilityEntry::unknown(),
                sandbox_controls: CapabilityEntry::unknown(),
                approval_controls: CapabilityEntry::unknown(),
                session_resume: CapabilityEntry::unknown(),
                emits_usage: CapabilityEntry::unknown(),
            }
        }
    }

    fn test_registry() -> AdapterRegistry {
        AdapterRegistry::new(vec![
            Arc::new(MockAdapter {
                name: "claude",
                adapter_tier: AdapterTier::Tier1,
            }),
            Arc::new(MockAdapter {
                name: "codex",
                adapter_tier: AdapterTier::Tier1,
            }),
            Arc::new(MockAdapter {
                name: "cursor-agent",
                adapter_tier: AdapterTier::Experimental,
            }),
        ])
    }

    #[test]
    fn tier1_returns_only_tier1_adapters() {
        let reg = test_registry();
        let tier1 = reg.tier1();
        assert_eq!(tier1.len(), 2);
        assert!(tier1.iter().all(|a| a.tier() == AdapterTier::Tier1));
    }

    #[test]
    fn resolve_tier1_adapter_succeeds() {
        let reg = test_registry();
        let adapter = reg.resolve("claude", false).unwrap();
        assert_eq!(adapter.key(), "claude");
    }

    #[test]
    fn resolve_unknown_adapter_returns_error() {
        let reg = test_registry();
        let result = reg.resolve("unknown", false);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, RegistryError::UnknownAdapter { .. }));
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn resolve_experimental_without_opt_in_returns_error() {
        let reg = test_registry();
        let result = reg.resolve("cursor-agent", false);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, RegistryError::ExperimentalBlocked { .. }));
        assert!(err.to_string().contains("--allow-experimental-adapters"));
    }

    #[test]
    fn resolve_experimental_with_opt_in_succeeds() {
        let reg = test_registry();
        let adapter = reg.resolve("cursor-agent", true).unwrap();
        assert_eq!(adapter.key(), "cursor-agent");
        assert_eq!(adapter.tier(), AdapterTier::Experimental);
    }

    #[test]
    fn resolve_many_succeeds_for_tier1() {
        let reg = test_registry();
        let adapters = reg
            .resolve_many(&["claude".to_string(), "codex".to_string()], false)
            .unwrap();
        assert_eq!(adapters.len(), 2);
    }

    #[test]
    fn resolve_many_fails_if_any_experimental_blocked() {
        let reg = test_registry();
        let result = reg.resolve_many(&["claude".to_string(), "cursor-agent".to_string()], false);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, RegistryError::ExperimentalBlocked { .. }));
    }

    #[test]
    fn available_without_experimental_excludes_cursor() {
        let reg = test_registry();
        let avail = reg.available(false);
        assert_eq!(avail.len(), 2);
        assert!(avail.iter().all(|a| a.tier() == AdapterTier::Tier1));
    }

    #[test]
    fn available_with_experimental_includes_cursor() {
        let reg = test_registry();
        let avail = reg.available(true);
        assert_eq!(avail.len(), 3);
    }

    #[test]
    fn known_keys_lists_all_adapters() {
        let reg = test_registry();
        let keys = reg.known_keys();
        assert_eq!(keys, vec!["claude", "codex", "cursor-agent"]);
    }
}
