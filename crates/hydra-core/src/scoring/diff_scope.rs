use serde::{Deserialize, Serialize};

use crate::config::DiffScopeConfig;

/// Statistics about a diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub lines_added: u32,
    pub lines_removed: u32,
    pub files_touched: u32,
    pub touched_paths: Vec<String>,
}

impl DiffStats {
    /// Total churn (lines added + removed).
    pub fn churn(&self) -> u32 {
        self.lines_added + self.lines_removed
    }
}

/// Detailed diff scope scoring breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffScopeScore {
    pub score: f64,
    pub churn_score: f64,
    pub files_score: f64,
    pub protected_path_violation: bool,
    pub violating_paths: Vec<String>,
}

/// Score diff scope. Modest churn scores highest.
///
/// Heuristics:
/// - Churn within soft limit: 100. Above: penalized linearly.
/// - Files within soft limit: 100. Above: penalized linearly.
/// - Protected path violation: hard cap at 30.
///
/// Final = weighted average of churn + files scores (equal weight), capped if protected violation.
pub fn score_diff_scope(stats: &DiffStats, config: &DiffScopeConfig) -> DiffScopeScore {
    let churn = stats.churn() as f64;
    let max_churn = config.max_churn_soft as f64;
    let churn_score = if max_churn == 0.0 || churn <= max_churn {
        100.0
    } else {
        // Linear decay: score drops from 100 toward 0 as churn increases beyond limit.
        // At 2x the limit, score = 50. At 3x, score = 33. etc.
        (max_churn / churn * 100.0).clamp(0.0, 100.0)
    };

    let files = stats.files_touched as f64;
    let max_files = config.max_files_soft as f64;
    let files_score = if max_files == 0.0 || files <= max_files {
        100.0
    } else {
        (max_files / files * 100.0).clamp(0.0, 100.0)
    };

    // Check protected path violations
    let violating_paths: Vec<String> = stats
        .touched_paths
        .iter()
        .filter(|path| {
            config
                .protected_paths
                .iter()
                .any(|protected| path.starts_with(protected.as_str()))
        })
        .cloned()
        .collect();
    let protected_path_violation = !violating_paths.is_empty();

    // Composite: equal weight for churn and files
    let base_score = (churn_score + files_score) / 2.0;

    // Hard cap for protected path violations
    let score = if protected_path_violation {
        base_score.min(30.0)
    } else {
        base_score
    };

    DiffScopeScore {
        score: score.clamp(0.0, 100.0),
        churn_score,
        files_score,
        protected_path_violation,
        violating_paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> DiffScopeConfig {
        DiffScopeConfig {
            max_files_soft: 20,
            max_churn_soft: 800,
            protected_paths: vec!["infra/".to_string(), "scripts/release/".to_string()],
        }
    }

    #[test]
    fn modest_churn_scores_100() {
        let stats = DiffStats {
            lines_added: 50,
            lines_removed: 20,
            files_touched: 3,
            touched_paths: vec!["src/main.rs".to_string()],
        };
        let s = score_diff_scope(&stats, &default_config());
        assert_eq!(s.score, 100.0);
        assert!(!s.protected_path_violation);
    }

    #[test]
    fn excessive_churn_penalized() {
        // 1600 churn = 2x limit -> churn_score = 50
        let stats = DiffStats {
            lines_added: 1200,
            lines_removed: 400,
            files_touched: 5,
            touched_paths: vec![],
        };
        let s = score_diff_scope(&stats, &default_config());
        // churn_score = 50, files_score = 100, avg = 75
        assert!((s.score - 75.0).abs() < 0.01);
        assert!((s.churn_score - 50.0).abs() < 0.01);
    }

    #[test]
    fn excessive_files_penalized() {
        // 40 files = 2x limit -> files_score = 50
        let stats = DiffStats {
            lines_added: 100,
            lines_removed: 50,
            files_touched: 40,
            touched_paths: vec![],
        };
        let s = score_diff_scope(&stats, &default_config());
        // churn_score = 100, files_score = 50, avg = 75
        assert!((s.score - 75.0).abs() < 0.01);
        assert!((s.files_score - 50.0).abs() < 0.01);
    }

    #[test]
    fn protected_path_caps_at_30() {
        let stats = DiffStats {
            lines_added: 10,
            lines_removed: 5,
            files_touched: 2,
            touched_paths: vec!["infra/deploy.yml".to_string(), "src/lib.rs".to_string()],
        };
        let s = score_diff_scope(&stats, &default_config());
        assert!(s.protected_path_violation);
        assert!(s.score <= 30.0);
        assert_eq!(s.violating_paths, vec!["infra/deploy.yml"]);
    }

    #[test]
    fn multiple_protected_violations() {
        let stats = DiffStats {
            lines_added: 10,
            lines_removed: 5,
            files_touched: 3,
            touched_paths: vec![
                "infra/main.tf".to_string(),
                "scripts/release/deploy.sh".to_string(),
                "src/lib.rs".to_string(),
            ],
        };
        let s = score_diff_scope(&stats, &default_config());
        assert!(s.protected_path_violation);
        assert!(s.score <= 30.0);
        assert_eq!(s.violating_paths.len(), 2);
    }

    #[test]
    fn zero_churn() {
        let stats = DiffStats {
            lines_added: 0,
            lines_removed: 0,
            files_touched: 0,
            touched_paths: vec![],
        };
        let s = score_diff_scope(&stats, &default_config());
        assert_eq!(s.score, 100.0);
    }

    #[test]
    fn no_protected_paths_in_config() {
        let config = DiffScopeConfig {
            max_files_soft: 20,
            max_churn_soft: 800,
            protected_paths: vec![],
        };
        let stats = DiffStats {
            lines_added: 10,
            lines_removed: 5,
            files_touched: 2,
            touched_paths: vec!["infra/deploy.yml".to_string()],
        };
        let s = score_diff_scope(&stats, &config);
        assert!(!s.protected_path_violation);
        assert_eq!(s.score, 100.0);
    }

    #[test]
    fn very_large_churn() {
        // 8000 churn = 10x limit -> churn_score = 10
        let stats = DiffStats {
            lines_added: 6000,
            lines_removed: 2000,
            files_touched: 5,
            touched_paths: vec![],
        };
        let s = score_diff_scope(&stats, &default_config());
        assert!((s.churn_score - 10.0).abs() < 0.01);
    }
}
