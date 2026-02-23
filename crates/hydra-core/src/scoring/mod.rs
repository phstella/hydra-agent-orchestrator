pub mod baseline;
pub mod build;
pub mod cost;
pub mod diff_scope;
pub mod lint;
pub mod ranking;
pub mod tests;

use serde::{Deserialize, Serialize};

/// Score for a single scoring dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub name: String,
    pub score: f64,
    pub evidence: serde_json::Value,
}
