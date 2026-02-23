use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Speed scoring breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedScore {
    pub score: f64,
    pub agent_duration: Duration,
    pub fastest_duration: Duration,
}

/// Score an agent's speed relative to the fastest successful agent.
///
/// Formula:
/// ```text
/// fastest = min(successful_agent_durations)
/// score = clamp((fastest / agent_duration) * 100, 0, 100)
/// ```
pub fn score_speed(agent_duration: Duration, fastest_duration: Duration) -> SpeedScore {
    let agent_secs = agent_duration.as_secs_f64();
    let fastest_secs = fastest_duration.as_secs_f64();

    let score = if agent_secs == 0.0 {
        100.0
    } else {
        (fastest_secs / agent_secs * 100.0).clamp(0.0, 100.0)
    };

    SpeedScore {
        score,
        agent_duration,
        fastest_duration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fastest_agent_gets_100() {
        let s = score_speed(Duration::from_secs(60), Duration::from_secs(60));
        assert_eq!(s.score, 100.0);
    }

    #[test]
    fn twice_as_slow() {
        let s = score_speed(Duration::from_secs(120), Duration::from_secs(60));
        assert!((s.score - 50.0).abs() < 0.01);
    }

    #[test]
    fn three_times_as_slow() {
        let s = score_speed(Duration::from_secs(180), Duration::from_secs(60));
        assert!((s.score - 33.33).abs() < 0.1);
    }

    #[test]
    fn zero_duration_gets_100() {
        let s = score_speed(Duration::from_secs(0), Duration::from_secs(60));
        assert_eq!(s.score, 100.0);
    }

    #[test]
    fn subsecond_precision() {
        let s = score_speed(Duration::from_millis(1500), Duration::from_millis(750));
        assert!((s.score - 50.0).abs() < 0.01);
    }
}
