use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Wilson score lower bound for confidence interval.
/// Used to rank solutions by confidence when sample sizes vary.
/// z = 1.96 for 95% confidence interval.
pub fn wilson_score(successes: u32, total: u32) -> f64 {
    if total == 0 {
        return 0.0;
    }

    let n = total as f64;
    let p = successes as f64 / n;
    let z = 1.96; // 95% CI
    let z2 = z * z;

    let numerator = p + z2 / (2.0 * n) - z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt();
    let denominator = 1.0 + z2 / n;

    (numerator / denominator).max(0.0)
}

/// Confidence threshold below which solutions are flagged for re-verification.
pub const LOW_CONFIDENCE_THRESHOLD: f64 = 0.3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionRecord {
    pub key: String,
    pub attempts: u32,
    pub successes: u32,
    pub last_attempt: String,
}

impl SolutionRecord {
    pub fn confidence(&self) -> f64 {
        wilson_score(self.successes, self.attempts)
    }

    pub fn needs_reverification(&self) -> bool {
        self.attempts >= 3 && self.confidence() < LOW_CONFIDENCE_THRESHOLD
    }
}

/// Persistent store for solution confidence data.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfidenceStore {
    pub solutions: HashMap<String, SolutionRecord>,
}

impl ConfidenceStore {
    pub fn new() -> Self {
        Self {
            solutions: HashMap::new(),
        }
    }

    pub fn load(path: &PathBuf) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn record_attempt(&mut self, key: &str, success: bool) {
        let record = self
            .solutions
            .entry(key.to_string())
            .or_insert_with(|| SolutionRecord {
                key: key.to_string(),
                attempts: 0,
                successes: 0,
                last_attempt: String::new(),
            });

        record.attempts += 1;
        if success {
            record.successes += 1;
        }
        record.last_attempt = chrono::Utc::now().to_rfc3339();
    }

    /// Get all solutions flagged for re-verification.
    pub fn flagged_solutions(&self) -> Vec<&SolutionRecord> {
        self.solutions
            .values()
            .filter(|s| s.needs_reverification())
            .collect()
    }

    /// Get confidence score for a specific solution.
    pub fn get_confidence(&self, key: &str) -> Option<f64> {
        self.solutions.get(key).map(|s| s.confidence())
    }
}

impl Default for ConfidenceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wilson_score_zero_total() {
        assert_eq!(wilson_score(0, 0), 0.0);
    }

    #[test]
    fn test_wilson_score_all_success() {
        let score = wilson_score(10, 10);
        assert!(score > 0.7, "All successes should have high confidence: {}", score);
        assert!(score < 1.0, "Should not be exactly 1.0: {}", score);
    }

    #[test]
    fn test_wilson_score_all_failure() {
        let score = wilson_score(0, 10);
        assert!(score < 0.05, "All failures should have very low confidence: {}", score);
    }

    #[test]
    fn test_wilson_score_50_50() {
        let score = wilson_score(5, 10);
        assert!(score > 0.2 && score < 0.5, "50/50 should be moderate: {}", score);
    }

    #[test]
    fn test_wilson_score_small_sample() {
        let score_small = wilson_score(1, 1); // 1/1
        let score_large = wilson_score(100, 100); // 100/100
        assert!(
            score_large > score_small,
            "Larger sample should give higher confidence: small={}, large={}",
            score_small,
            score_large
        );
    }

    #[test]
    fn test_wilson_score_one_success_one_total() {
        let score = wilson_score(1, 1);
        // With z=1.96, 1/1 should be moderate (not very high due to small sample)
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_solution_record_confidence() {
        let record = SolutionRecord {
            key: "fix_scheduler".into(),
            attempts: 10,
            successes: 8,
            last_attempt: "2026-01-01T00:00:00Z".into(),
        };
        let confidence = record.confidence();
        assert!(confidence > 0.4, "80% success rate should have decent confidence: {}", confidence);
    }

    #[test]
    fn test_solution_record_needs_reverification() {
        let bad_solution = SolutionRecord {
            key: "broken_fix".into(),
            attempts: 5,
            successes: 1,
            last_attempt: String::new(),
        };
        assert!(bad_solution.needs_reverification());

        let good_solution = SolutionRecord {
            key: "working_fix".into(),
            attempts: 5,
            successes: 4,
            last_attempt: String::new(),
        };
        assert!(!good_solution.needs_reverification());
    }

    #[test]
    fn test_solution_record_too_few_attempts() {
        // With < 3 attempts, should NOT flag for reverification even if all failed
        let new_solution = SolutionRecord {
            key: "new_fix".into(),
            attempts: 2,
            successes: 0,
            last_attempt: String::new(),
        };
        assert!(!new_solution.needs_reverification());
    }

    #[test]
    fn test_confidence_store_record_and_query() {
        let mut store = ConfidenceStore::new();

        store.record_attempt("fix_a", true);
        store.record_attempt("fix_a", true);
        store.record_attempt("fix_a", false);

        assert_eq!(store.solutions["fix_a"].attempts, 3);
        assert_eq!(store.solutions["fix_a"].successes, 2);

        let confidence = store.get_confidence("fix_a").unwrap();
        assert!(confidence > 0.0);
    }

    #[test]
    fn test_confidence_store_flagged() {
        let mut store = ConfidenceStore::new();

        // Good solution
        for _ in 0..10 {
            store.record_attempt("good", true);
        }

        // Bad solution
        for _ in 0..5 {
            store.record_attempt("bad", false);
        }

        let flagged = store.flagged_solutions();
        assert_eq!(flagged.len(), 1);
        assert_eq!(flagged[0].key, "bad");
    }

    #[test]
    fn test_confidence_store_persistence() {
        let path = std::env::temp_dir().join(format!("sandy_conf_test_{}.json", uuid::Uuid::new_v4()));

        let mut store = ConfidenceStore::new();
        store.record_attempt("test_solution", true);
        store.save(&path).unwrap();

        let loaded = ConfidenceStore::load(&path);
        assert_eq!(loaded.solutions["test_solution"].attempts, 1);
        assert_eq!(loaded.solutions["test_solution"].successes, 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_confidence_store_unknown_key() {
        let store = ConfidenceStore::new();
        assert!(store.get_confidence("nonexistent").is_none());
    }
}
