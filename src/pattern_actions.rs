use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::tools::patterns::{PatternsData, Pattern};

/// Minimum number of observations before a pattern triggers action suggestions.
const HIGH_FREQUENCY_THRESHOLD: i32 = 5;

/// Types of actions that can be suggested for high-frequency patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestedAction {
    /// Create a reusable skill based on this pattern
    CreateSkill {
        pattern_id: String,
        pattern_name: String,
        reason: String,
    },
    /// Schedule a recurring task related to this pattern
    ScheduleTask {
        pattern_id: String,
        pattern_name: String,
        reason: String,
    },
    /// Send an insight to the control chat
    SendInsight {
        pattern_id: String,
        pattern_name: String,
        insight: String,
    },
}

impl SuggestedAction {
    pub fn description(&self) -> String {
        match self {
            SuggestedAction::CreateSkill { pattern_name, reason, .. } => {
                format!("Create a skill for '{}': {}", pattern_name, reason)
            }
            SuggestedAction::ScheduleTask { pattern_name, reason, .. } => {
                format!("Schedule task for '{}': {}", pattern_name, reason)
            }
            SuggestedAction::SendInsight { pattern_name, insight, .. } => {
                format!("Insight about '{}': {}", pattern_name, insight)
            }
        }
    }
}

/// Analyze patterns and generate action suggestions for high-frequency ones.
pub fn analyze_patterns(data: &PatternsData) -> Vec<SuggestedAction> {
    let mut suggestions = Vec::new();

    for pattern in &data.patterns {
        if pattern.observations_count < HIGH_FREQUENCY_THRESHOLD {
            continue;
        }

        // Generate suggestions based on pattern characteristics
        suggestions.extend(suggest_actions_for_pattern(pattern));
    }

    suggestions
}

fn suggest_actions_for_pattern(pattern: &Pattern) -> Vec<SuggestedAction> {
    let mut actions = Vec::new();

    // High confidence patterns (>= 60%) → suggest creating a skill
    if pattern.confidence >= 60 && pattern.observations_count >= HIGH_FREQUENCY_THRESHOLD {
        actions.push(SuggestedAction::CreateSkill {
            pattern_id: pattern.id.clone(),
            pattern_name: pattern.name.clone(),
            reason: format!(
                "Pattern '{}' has {} observations at {}% confidence — a reusable skill could help.",
                pattern.name, pattern.observations_count, pattern.confidence
            ),
        });
    }

    // Patterns with time-based observations → suggest scheduling
    let has_temporal_aspect = pattern.category == "productivity"
        || pattern.description.to_lowercase().contains("time")
        || pattern.description.to_lowercase().contains("morning")
        || pattern.description.to_lowercase().contains("evening")
        || pattern.description.to_lowercase().contains("schedule");

    if has_temporal_aspect && pattern.observations_count >= HIGH_FREQUENCY_THRESHOLD {
        actions.push(SuggestedAction::ScheduleTask {
            pattern_id: pattern.id.clone(),
            pattern_name: pattern.name.clone(),
            reason: format!(
                "Pattern '{}' has a temporal aspect — consider scheduling a related check-in.",
                pattern.name
            ),
        });
    }

    // Very high observation count → send insight
    if pattern.observations_count >= 10 {
        let insight = if let Some(ref hypothesis) = pattern.hypothesis {
            format!(
                "Strong pattern detected ({}+ observations): {}",
                pattern.observations_count, hypothesis
            )
        } else {
            format!(
                "Pattern '{}' has accumulated {} observations but no hypothesis yet. Consider formulating one.",
                pattern.name, pattern.observations_count
            )
        };

        actions.push(SuggestedAction::SendInsight {
            pattern_id: pattern.id.clone(),
            pattern_name: pattern.name.clone(),
            insight,
        });
    }

    actions
}

/// Format suggestions as a human-readable report.
pub fn format_suggestions(suggestions: &[SuggestedAction]) -> String {
    if suggestions.is_empty() {
        return "No pattern-based suggestions at this time.".into();
    }

    let mut report = format!("📊 **Pattern Analysis** ({} suggestions)\n\n", suggestions.len());

    for (i, action) in suggestions.iter().enumerate() {
        let emoji = match action {
            SuggestedAction::CreateSkill { .. } => "🛠️",
            SuggestedAction::ScheduleTask { .. } => "⏰",
            SuggestedAction::SendInsight { .. } => "💡",
        };
        report.push_str(&format!("{}. {} {}\n", i + 1, emoji, action.description()));
    }

    report
}

/// Load patterns from disk and analyze them.
pub fn analyze_from_data_dir(data_dir: &Path) -> Vec<SuggestedAction> {
    let path = data_dir.join("runtime/patterns.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let data: PatternsData = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    analyze_patterns(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::patterns::{Observation, PatternsMeta};

    fn make_pattern(id: &str, name: &str, obs_count: i32, confidence: i32, category: &str) -> Pattern {
        Pattern {
            id: id.into(),
            name: name.into(),
            description: format!("Test pattern: {}", name),
            category: category.into(),
            confidence,
            observations_count: obs_count,
            hypothesis: if confidence > 50 {
                Some(format!("Hypothesis for {}", name))
            } else {
                None
            },
            evidence: (0..obs_count as usize)
                .map(|i| Observation {
                    date: "2026-01-01T00:00:00Z".into(),
                    observation: format!("Obs {}", i),
                    context: "test".into(),
                    supports_pattern: true,
                })
                .collect(),
            created_at: "2026-01-01T00:00:00Z".into(),
            last_updated: "2026-02-01T00:00:00Z".into(),
            confidence_locked: false,
        }
    }

    fn make_data(patterns: Vec<Pattern>) -> PatternsData {
        PatternsData {
            version: "1.0".into(),
            user_id: "test".into(),
            patterns,
            meta: PatternsMeta {
                total_patterns: 0,
                last_analysis: "2026-01-01T00:00:00Z".into(),
                learning_active: true,
                notes: None,
            },
        }
    }

    #[test]
    fn test_no_suggestions_for_low_frequency() {
        let data = make_data(vec![
            make_pattern("p1", "Low Freq", 3, 50, "cognitive"),
        ]);
        let suggestions = analyze_patterns(&data);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_create_skill_suggestion() {
        let data = make_data(vec![
            make_pattern("focus", "Focus Patterns", 7, 70, "cognitive"),
        ]);
        let suggestions = analyze_patterns(&data);
        assert!(
            suggestions.iter().any(|s| matches!(s, SuggestedAction::CreateSkill { .. })),
            "Should suggest creating a skill"
        );
    }

    #[test]
    fn test_schedule_task_suggestion_for_temporal() {
        let mut pattern = make_pattern("morning", "Morning Routine", 6, 40, "productivity");
        pattern.description = "Tracks morning energy and focus time patterns".into();
        let data = make_data(vec![pattern]);
        let suggestions = analyze_patterns(&data);
        assert!(
            suggestions.iter().any(|s| matches!(s, SuggestedAction::ScheduleTask { .. })),
            "Should suggest scheduling for temporal pattern"
        );
    }

    #[test]
    fn test_send_insight_for_high_observations() {
        let data = make_data(vec![
            make_pattern("energy", "Energy Levels", 12, 75, "physical"),
        ]);
        let suggestions = analyze_patterns(&data);
        assert!(
            suggestions.iter().any(|s| matches!(s, SuggestedAction::SendInsight { .. })),
            "Should suggest insight for 12 observations"
        );
    }

    #[test]
    fn test_insight_with_hypothesis() {
        let data = make_data(vec![
            make_pattern("focus", "Focus Patterns", 15, 80, "cognitive"),
        ]);
        let suggestions = analyze_patterns(&data);
        let insight = suggestions
            .iter()
            .find(|s| matches!(s, SuggestedAction::SendInsight { .. }));
        assert!(insight.is_some());
        if let SuggestedAction::SendInsight { insight, .. } = insight.unwrap() {
            assert!(insight.contains("Hypothesis"));
        }
    }

    #[test]
    fn test_insight_without_hypothesis() {
        let mut pattern = make_pattern("new", "New Pattern", 10, 40, "cognitive");
        pattern.hypothesis = None;
        let data = make_data(vec![pattern]);
        let suggestions = analyze_patterns(&data);
        let insight = suggestions
            .iter()
            .find(|s| matches!(s, SuggestedAction::SendInsight { .. }));
        assert!(insight.is_some());
        if let SuggestedAction::SendInsight { insight, .. } = insight.unwrap() {
            assert!(insight.contains("no hypothesis"));
        }
    }

    #[test]
    fn test_format_suggestions() {
        let suggestions = vec![
            SuggestedAction::CreateSkill {
                pattern_id: "p1".into(),
                pattern_name: "Focus".into(),
                reason: "High confidence pattern".into(),
            },
            SuggestedAction::SendInsight {
                pattern_id: "p2".into(),
                pattern_name: "Energy".into(),
                insight: "Strong pattern detected".into(),
            },
        ];
        let report = format_suggestions(&suggestions);
        assert!(report.contains("Pattern Analysis"));
        assert!(report.contains("Focus"));
        assert!(report.contains("Energy"));
        assert!(report.contains("2 suggestions"));
    }

    #[test]
    fn test_format_empty_suggestions() {
        let report = format_suggestions(&[]);
        assert!(report.contains("No pattern-based suggestions"));
    }

    #[test]
    fn test_multiple_patterns() {
        let data = make_data(vec![
            make_pattern("focus", "Focus", 7, 70, "cognitive"),
            make_pattern("energy", "Energy", 3, 30, "physical"),
            make_pattern("time", "Time Blindness", 12, 80, "productivity"),
        ]);
        let suggestions = analyze_patterns(&data);
        // Focus: skill + insight (>10? no, 7). Just skill.
        // Energy: below threshold
        // Time: skill + schedule + insight
        assert!(suggestions.len() >= 2, "Should have multiple suggestions: got {}", suggestions.len());
    }
}
