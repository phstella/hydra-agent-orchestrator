use serde::{Deserialize, Serialize};

/// Build score breakdown with evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildScore {
    pub score: f64,
    pub build_passed: bool,
}

/// Score a build result. Binary: pass = 100, fail = 0.
pub fn score_build(build_passed: bool) -> BuildScore {
    BuildScore {
        score: if build_passed { 100.0 } else { 0.0 },
        build_passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pass() {
        let s = score_build(true);
        assert_eq!(s.score, 100.0);
        assert!(s.build_passed);
    }

    #[test]
    fn build_fail() {
        let s = score_build(false);
        assert_eq!(s.score, 0.0);
        assert!(!s.build_passed);
    }
}
