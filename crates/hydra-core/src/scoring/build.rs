use serde::{Deserialize, Serialize};

use super::DimensionScore;
use crate::scoring::baseline::CommandResult;

/// Score the build dimension: binary pass=100, fail=0.
pub fn score_build(
    _baseline: Option<&CommandResult>,
    agent_result: &CommandResult,
) -> DimensionScore {
    let score = if agent_result.success { 100.0 } else { 0.0 };

    DimensionScore {
        name: "build".to_string(),
        score,
        evidence: serde_json::json!({
            "command": agent_result.command,
            "exit_code": agent_result.exit_code,
            "success": agent_result.success,
            "duration_ms": agent_result.duration_ms,
        }),
    }
}

/// Detailed build score with additional context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildScoreDetail {
    pub passed: bool,
    pub baseline_passed: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(success: bool, exit_code: i32) -> CommandResult {
        CommandResult {
            command: "cargo build".to_string(),
            success,
            exit_code,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 100,
        }
    }

    #[test]
    fn build_pass_scores_100() {
        let result = make_result(true, 0);
        let score = score_build(None, &result);
        assert_eq!(score.score, 100.0);
        assert_eq!(score.name, "build");
    }

    #[test]
    fn build_fail_scores_0() {
        let result = make_result(false, 1);
        let score = score_build(None, &result);
        assert_eq!(score.score, 0.0);
    }

    #[test]
    fn build_score_includes_evidence() {
        let result = make_result(true, 0);
        let score = score_build(None, &result);
        assert_eq!(score.evidence["exit_code"], 0);
        assert_eq!(score.evidence["success"], true);
    }

    #[test]
    fn baseline_broken_agent_pass_still_scores_100() {
        let baseline = make_result(false, 1);
        let agent = make_result(true, 0);
        let score = score_build(Some(&baseline), &agent);
        assert_eq!(score.score, 100.0);
    }

    #[test]
    fn baseline_broken_agent_fail_scores_0() {
        let baseline = make_result(false, 1);
        let agent = make_result(false, 1);
        let score = score_build(Some(&baseline), &agent);
        assert_eq!(score.score, 0.0);
    }
}
