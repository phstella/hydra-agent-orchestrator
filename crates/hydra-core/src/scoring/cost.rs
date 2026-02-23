use serde::{Deserialize, Serialize};

use crate::adapter::AgentEvent;

/// Accumulates token usage from agent event streams.
#[derive(Debug, Clone, Default)]
pub struct UsageAccumulator {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub usage_events: u64,
    pub reported_cost_usd: f64,
    has_reported_cost: bool,
}

impl UsageAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an agent event, accumulating usage if present.
    pub fn process_event(&mut self, event: &AgentEvent) {
        if let AgentEvent::Usage {
            input_tokens,
            output_tokens,
            extra,
        } = event
        {
            self.input_tokens += input_tokens;
            self.output_tokens += output_tokens;
            self.usage_events += 1;
            if let Some(cost) = extra.get("cost_usd").and_then(|v| v.as_f64()) {
                self.reported_cost_usd += cost;
                self.has_reported_cost = true;
            }
        }
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    pub fn has_usage_data(&self) -> bool {
        self.usage_events > 0
    }

    pub fn to_estimate(&self) -> CostEstimate {
        CostEstimate {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            total_tokens: self.total_tokens(),
            estimated_cost_usd: self.has_reported_cost.then_some(self.reported_cost_usd),
        }
    }
}

/// Per-agent cost estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: Option<f64>,
}

/// Budget enforcement action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetAction {
    Continue,
    Stop { reason: String },
}

/// Check budget limits against accumulated usage.
pub fn check_budget(
    accumulator: &UsageAccumulator,
    max_tokens_total: Option<u64>,
    max_cost_usd: Option<f64>,
) -> BudgetAction {
    if let Some(max_tokens) = max_tokens_total {
        if accumulator.total_tokens() >= max_tokens {
            return BudgetAction::Stop {
                reason: format!(
                    "token budget exceeded: {} >= {}",
                    accumulator.total_tokens(),
                    max_tokens
                ),
            };
        }
    }

    if let Some(max_cost) = max_cost_usd {
        if let Some(estimate_cost) = accumulator.to_estimate().estimated_cost_usd {
            if estimate_cost >= max_cost {
                return BudgetAction::Stop {
                    reason: format!("cost budget exceeded: ${estimate_cost:.4} >= ${max_cost:.4}"),
                };
            }
        }
    }

    BudgetAction::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn accumulate_usage_events() {
        let mut acc = UsageAccumulator::new();
        acc.process_event(&AgentEvent::Usage {
            input_tokens: 100,
            output_tokens: 50,
            extra: HashMap::new(),
        });
        acc.process_event(&AgentEvent::Usage {
            input_tokens: 200,
            output_tokens: 100,
            extra: HashMap::new(),
        });
        assert_eq!(acc.input_tokens, 300);
        assert_eq!(acc.output_tokens, 150);
        assert_eq!(acc.total_tokens(), 450);
        assert_eq!(acc.usage_events, 2);
        assert!(acc.has_usage_data());
    }

    #[test]
    fn non_usage_events_ignored() {
        let mut acc = UsageAccumulator::new();
        acc.process_event(&AgentEvent::Message {
            content: "hello".to_string(),
        });
        assert_eq!(acc.total_tokens(), 0);
        assert!(!acc.has_usage_data());
    }

    #[test]
    fn budget_continue_when_under_limit() {
        let acc = UsageAccumulator {
            input_tokens: 100,
            output_tokens: 50,
            usage_events: 1,
            reported_cost_usd: 0.0,
            has_reported_cost: false,
        };
        assert_eq!(check_budget(&acc, Some(1000), None), BudgetAction::Continue);
    }

    #[test]
    fn budget_stop_when_over_limit() {
        let acc = UsageAccumulator {
            input_tokens: 800,
            output_tokens: 300,
            usage_events: 1,
            reported_cost_usd: 0.0,
            has_reported_cost: false,
        };
        let action = check_budget(&acc, Some(1000), None);
        assert!(matches!(action, BudgetAction::Stop { .. }));
    }

    #[test]
    fn budget_no_limit_always_continues() {
        let acc = UsageAccumulator {
            input_tokens: 999999,
            output_tokens: 999999,
            usage_events: 2,
            reported_cost_usd: 0.0,
            has_reported_cost: false,
        };
        assert_eq!(check_budget(&acc, None, None), BudgetAction::Continue);
    }

    #[test]
    fn to_estimate_conversion() {
        let acc = UsageAccumulator {
            input_tokens: 100,
            output_tokens: 50,
            usage_events: 1,
            reported_cost_usd: 0.0,
            has_reported_cost: false,
        };
        let est = acc.to_estimate();
        assert_eq!(est.total_tokens, 150);
        assert!(est.estimated_cost_usd.is_none());
    }

    #[test]
    fn cost_budget_stops_when_reported_cost_exceeds_limit() {
        let mut extra = HashMap::new();
        extra.insert("cost_usd".to_string(), serde_json::json!(0.75));
        let mut acc = UsageAccumulator::new();
        acc.process_event(&AgentEvent::Usage {
            input_tokens: 100,
            output_tokens: 10,
            extra,
        });

        let action = check_budget(&acc, None, Some(0.5));
        assert!(matches!(action, BudgetAction::Stop { .. }));
    }
}
