//! Cost tracking and budget enforcement for agent runs.
//!
//! Tracks token usage per agent and checks against configured budget limits.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::BudgetConfig;

/// Approximate pricing per million tokens (USD).
/// These are rough estimates for cost tracking purposes.
const INPUT_PRICE_PER_M: f64 = 3.0;
const OUTPUT_PRICE_PER_M: f64 = 15.0;

/// Tracks cost across all agents in a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    pub per_agent: HashMap<String, AgentCost>,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

/// Cost breakdown for a single agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_cost_usd: f64,
}

/// Indicates a budget limit has been exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetExceeded {
    pub reason: String,
    pub agent_key: Option<String>,
}

/// Summary of costs for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub per_agent: Vec<(String, AgentCost)>,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub budget_remaining: Option<f64>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record token usage from an agent event.
    pub fn record_usage(&mut self, agent_key: &str, input_tokens: u64, output_tokens: u64) {
        let cost = estimate_cost(input_tokens, output_tokens);

        let entry = self
            .per_agent
            .entry(agent_key.to_string())
            .or_default();
        entry.input_tokens += input_tokens;
        entry.output_tokens += output_tokens;
        entry.estimated_cost_usd += cost;

        self.total_tokens += input_tokens + output_tokens;
        self.total_cost_usd += cost;
    }

    /// Check if budget limits are exceeded.
    ///
    /// Returns `Some(BudgetExceeded)` if any limit is breached, `None` otherwise.
    pub fn check_budget(&self, config: &BudgetConfig) -> Option<BudgetExceeded> {
        if let Some(max_tokens) = config.max_tokens_total {
            if self.total_tokens > max_tokens {
                return Some(BudgetExceeded {
                    reason: format!(
                        "total tokens {} exceeds limit {}",
                        self.total_tokens, max_tokens
                    ),
                    agent_key: None,
                });
            }
        }

        if let Some(max_cost) = config.max_cost_usd {
            if self.total_cost_usd > max_cost {
                return Some(BudgetExceeded {
                    reason: format!(
                        "total cost ${:.4} exceeds limit ${:.2}",
                        self.total_cost_usd, max_cost
                    ),
                    agent_key: None,
                });
            }
        }

        None
    }

    /// Get cost summary for display.
    pub fn summary(&self, config: &BudgetConfig) -> CostSummary {
        let mut per_agent: Vec<(String, AgentCost)> = self
            .per_agent
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        per_agent.sort_by(|a, b| a.0.cmp(&b.0));

        let budget_remaining = config
            .max_cost_usd
            .map(|max| (max - self.total_cost_usd).max(0.0));

        CostSummary {
            per_agent,
            total_tokens: self.total_tokens,
            total_cost_usd: self.total_cost_usd,
            budget_remaining,
        }
    }
}

/// Estimate cost in USD from token counts.
fn estimate_cost(input_tokens: u64, output_tokens: u64) -> f64 {
    let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_PRICE_PER_M;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_PRICE_PER_M;
    input_cost + output_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_is_empty() {
        let tracker = CostTracker::new();
        assert_eq!(tracker.total_tokens, 0);
        assert_eq!(tracker.total_cost_usd, 0.0);
        assert!(tracker.per_agent.is_empty());
    }

    #[test]
    fn record_usage_accumulates() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("claude", 1000, 500);
        tracker.record_usage("claude", 2000, 1000);

        assert_eq!(tracker.total_tokens, 4500);
        let agent = &tracker.per_agent["claude"];
        assert_eq!(agent.input_tokens, 3000);
        assert_eq!(agent.output_tokens, 1500);
    }

    #[test]
    fn record_usage_multiple_agents() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("claude", 1000, 500);
        tracker.record_usage("codex", 2000, 1000);

        assert_eq!(tracker.per_agent.len(), 2);
        assert_eq!(tracker.per_agent["claude"].input_tokens, 1000);
        assert_eq!(tracker.per_agent["codex"].input_tokens, 2000);
        assert_eq!(tracker.total_tokens, 4500);
    }

    #[test]
    fn budget_not_exceeded_no_limits() {
        let tracker = CostTracker::new();
        let config = BudgetConfig {
            max_tokens_total: None,
            max_cost_usd: None,
        };
        assert!(tracker.check_budget(&config).is_none());
    }

    #[test]
    fn budget_not_exceeded_under_limit() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("claude", 1000, 500);
        let config = BudgetConfig {
            max_tokens_total: Some(100_000),
            max_cost_usd: Some(10.0),
        };
        assert!(tracker.check_budget(&config).is_none());
    }

    #[test]
    fn budget_exceeded_tokens() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("claude", 50_000, 60_000);
        let config = BudgetConfig {
            max_tokens_total: Some(100_000),
            max_cost_usd: None,
        };
        let exceeded = tracker.check_budget(&config);
        assert!(exceeded.is_some());
        let exceeded = exceeded.unwrap();
        assert!(exceeded.reason.contains("tokens"));
        assert!(exceeded.reason.contains("110000"));
    }

    #[test]
    fn budget_exceeded_cost() {
        let mut tracker = CostTracker::new();
        // Record enough tokens to exceed $1.00 budget
        // 1M input tokens at $3/M = $3.00
        tracker.record_usage("claude", 1_000_000, 0);
        let config = BudgetConfig {
            max_tokens_total: None,
            max_cost_usd: Some(1.0),
        };
        let exceeded = tracker.check_budget(&config);
        assert!(exceeded.is_some());
        assert!(exceeded.unwrap().reason.contains("cost"));
    }

    #[test]
    fn cost_estimation() {
        // 1M input tokens at $3/M = $3.00
        let cost = estimate_cost(1_000_000, 0);
        assert!((cost - 3.0).abs() < 0.001);

        // 1M output tokens at $15/M = $15.00
        let cost = estimate_cost(0, 1_000_000);
        assert!((cost - 15.0).abs() < 0.001);

        // Combined
        let cost = estimate_cost(1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.001);
    }

    #[test]
    fn summary_with_budget() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("claude", 1000, 500);
        tracker.record_usage("codex", 2000, 1000);

        let config = BudgetConfig {
            max_tokens_total: None,
            max_cost_usd: Some(10.0),
        };
        let summary = tracker.summary(&config);

        assert_eq!(summary.per_agent.len(), 2);
        assert_eq!(summary.total_tokens, 4500);
        assert!(summary.budget_remaining.is_some());
        assert!(summary.budget_remaining.unwrap() > 0.0);
    }

    #[test]
    fn summary_without_budget() {
        let tracker = CostTracker::new();
        let config = BudgetConfig {
            max_tokens_total: None,
            max_cost_usd: None,
        };
        let summary = tracker.summary(&config);
        assert!(summary.budget_remaining.is_none());
    }

    #[test]
    fn summary_sorted_by_agent_key() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("codex", 1000, 500);
        tracker.record_usage("claude", 2000, 1000);
        tracker.record_usage("cursor", 500, 250);

        let config = BudgetConfig::default();
        let summary = tracker.summary(&config);

        assert_eq!(summary.per_agent[0].0, "claude");
        assert_eq!(summary.per_agent[1].0, "codex");
        assert_eq!(summary.per_agent[2].0, "cursor");
    }

    #[test]
    fn serialization_round_trip() {
        let mut tracker = CostTracker::new();
        tracker.record_usage("claude", 5000, 2000);

        let json = serde_json::to_string(&tracker).expect("serialize");
        let deser: CostTracker = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deser.total_tokens, tracker.total_tokens);
        assert_eq!(deser.per_agent["claude"].input_tokens, 5000);
    }
}
