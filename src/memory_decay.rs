/// Temporal memory decay: score memories based on age using exponential decay.
/// Different half-lives for different categories of memory.

/// Half-lives in days for different memory categories
pub const HALF_LIFE_SOLUTIONS: f64 = 30.0;
pub const HALF_LIFE_ERRORS: f64 = 14.0;
pub const HALF_LIFE_PATTERNS: f64 = 60.0;
pub const HALF_LIFE_INSIGHTS: f64 = 90.0;

/// Calculate decay score for a memory based on its age.
/// Returns a value between 0.0 and 1.0, where 1.0 is brand new.
/// Formula: exp(-ln(2) * age_days / half_life)
pub fn decay_score(age_days: f64, half_life: f64) -> f64 {
    if half_life <= 0.0 {
        return 0.0;
    }
    if age_days <= 0.0 {
        return 1.0;
    }
    (-f64::ln(2.0) * age_days / half_life).exp()
}

/// Get the half-life for a given category name.
pub fn half_life_for_category(category: &str) -> f64 {
    match category {
        "solutions" => HALF_LIFE_SOLUTIONS,
        "errors" => HALF_LIFE_ERRORS,
        "patterns" => HALF_LIFE_PATTERNS,
        "insights" => HALF_LIFE_INSIGHTS,
        _ => HALF_LIFE_PATTERNS, // default
    }
}

/// Parse a timestamp from a memory entry header like "## 2024-02-17 10:30:45 UTC"
/// Returns age in days from now, or None if parsing fails.
pub fn age_from_timestamp(timestamp_str: &str) -> Option<f64> {
    // Try parsing "YYYY-MM-DD HH:MM:SS UTC" format
    let cleaned = timestamp_str.trim().trim_start_matches("## ").trim();
    let dt = chrono::NaiveDateTime::parse_from_str(
        cleaned.trim_end_matches(" UTC"),
        "%Y-%m-%d %H:%M:%S",
    )
    .ok()?;

    let then = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc);
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(then);
    Some(duration.num_seconds() as f64 / 86400.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_score_zero_age() {
        let score = decay_score(0.0, 30.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decay_score_at_half_life() {
        let score = decay_score(30.0, 30.0);
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_decay_score_at_double_half_life() {
        let score = decay_score(60.0, 30.0);
        assert!((score - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_decay_score_very_old() {
        let score = decay_score(365.0, 14.0);
        assert!(score < 0.001);
    }

    #[test]
    fn test_decay_score_negative_age() {
        let score = decay_score(-5.0, 30.0);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_decay_score_zero_half_life() {
        let score = decay_score(10.0, 0.0);
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_half_life_for_category() {
        assert_eq!(half_life_for_category("solutions"), 30.0);
        assert_eq!(half_life_for_category("errors"), 14.0);
        assert_eq!(half_life_for_category("patterns"), 60.0);
        assert_eq!(half_life_for_category("insights"), 90.0);
        assert_eq!(half_life_for_category("unknown"), 60.0); // default
    }

    #[test]
    fn test_decay_different_categories() {
        let age = 14.0;
        let error_score = decay_score(age, HALF_LIFE_ERRORS);
        let solution_score = decay_score(age, HALF_LIFE_SOLUTIONS);
        let pattern_score = decay_score(age, HALF_LIFE_PATTERNS);

        // Errors decay fastest (shortest half-life)
        assert!((error_score - 0.5).abs() < 0.001); // 14 days = half-life for errors
        assert!(solution_score > error_score); // Solutions decay slower
        assert!(pattern_score > solution_score); // Patterns decay slowest
    }

    #[test]
    fn test_age_from_timestamp() {
        // This test uses a known past date
        let age = age_from_timestamp("## 2025-01-01 00:00:00 UTC");
        assert!(age.is_some());
        assert!(age.unwrap() > 0.0);
    }

    #[test]
    fn test_age_from_timestamp_invalid() {
        assert!(age_from_timestamp("not a date").is_none());
        assert!(age_from_timestamp("").is_none());
    }
}
