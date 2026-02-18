use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

use crate::activity::ActivityLogger;
use crate::atomic_io::atomic_write_json;
use crate::claude::ToolDefinition;
use crate::memory_decay;

use super::{schema_object, Tool, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub confidence: i32,
    pub observations_count: i32,
    pub hypothesis: Option<String>,
    pub evidence: Vec<Observation>,
    pub created_at: String,
    pub last_updated: String,
    #[serde(default)]
    pub confidence_locked: bool,
}

fn default_supports() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub date: String,
    pub observation: String,
    pub context: String,
    #[serde(default = "default_supports")]
    pub supports_pattern: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternsData {
    pub version: String,
    pub user_id: String,
    pub patterns: Vec<Pattern>,
    pub meta: PatternsMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternsMeta {
    pub total_patterns: i32,
    pub last_analysis: String,
    pub learning_active: bool,
    pub notes: Option<String>,
}

fn patterns_path(data_dir: &Path) -> PathBuf {
    data_dir.join("runtime/patterns.json")
}

fn read_patterns(data_dir: &Path) -> PatternsData {
    let path = patterns_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to deserialize patterns from {}: {}", path.display(), e);
                // Try legacy migration (object format → vec format)
                if let Some(migrated) = try_migrate_legacy(&content) {
                    info!("Successfully migrated legacy patterns format");
                    // Save migrated data
                    if let Err(write_err) = write_patterns(data_dir, &migrated) {
                        error!("Failed to save migrated patterns: {}", write_err);
                    }
                    return migrated;
                }
                default_patterns()
            }
        },
        Err(_) => default_patterns(),
    }
}

/// Attempt to migrate legacy patterns format where "patterns" is a JSON Object
/// (keyed by pattern ID) into the current Vec<Pattern> format.
fn try_migrate_legacy(content: &str) -> Option<PatternsData> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let obj = value.as_object()?;
    let patterns_val = obj.get("patterns")?;

    // Only migrate if patterns is an object (old format), not an array
    let patterns_obj = patterns_val.as_object()?;

    let mut patterns = Vec::new();
    for (id, entry) in patterns_obj {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or(id).to_string();
        let description = entry.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let category = entry.get("category").and_then(|v| v.as_str()).unwrap_or("general").to_string();
        let confidence = entry.get("confidence").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let observations_count = entry.get("observations_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let hypothesis = entry.get("hypothesis").and_then(|v| v.as_str()).map(String::from);
        let created_at = entry.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let last_updated = entry.get("last_updated").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let evidence: Vec<Observation> = entry
            .get("evidence")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        patterns.push(Pattern {
            id: id.clone(),
            name,
            description,
            category,
            confidence,
            observations_count,
            hypothesis,
            evidence,
            created_at,
            last_updated,
            confidence_locked: false,
        });
    }

    let total = patterns.len() as i32;
    let version = obj.get("version").and_then(|v| v.as_str()).unwrap_or("1.0").to_string();
    let user_id = obj.get("user_id").and_then(|v| v.as_str()).unwrap_or("default").to_string();

    Some(PatternsData {
        version,
        user_id,
        patterns,
        meta: PatternsMeta {
            total_patterns: total,
            last_analysis: chrono::Utc::now().to_rfc3339(),
            learning_active: true,
            notes: Some("Migrated from legacy object format".to_string()),
        },
    })
}

fn write_patterns(data_dir: &Path, data: &PatternsData) -> std::io::Result<()> {
    let path = patterns_path(data_dir);
    atomic_write_json(&path, data)
}

fn default_patterns() -> PatternsData {
    PatternsData {
        version: "1.0".to_string(),
        user_id: "default".to_string(),
        patterns: vec![],
        meta: PatternsMeta {
            total_patterns: 0,
            last_analysis: chrono::Utc::now().to_rfc3339(),
            learning_active: true,
            notes: Some("Default empty patterns. Initialize with categories.".to_string()),
        },
    }
}

/// Compute confidence using temporal decay and contradiction awareness.
///
/// 1. Parse each observation's date, compute age in days
/// 2. Weight by exponential decay (60-day half-life)
/// 3. Split into supporting/contradicting by `supports_pattern`
/// 4. agreement_ratio = weighted_support / total_weight
/// 5. count_factor = (1.0 - exp(-0.3 * count)) — asymptotic to 1.0
/// 6. confidence = (agreement_ratio * count_factor * 100.0).clamp(0, 95)
fn compute_confidence(evidence: &[Observation]) -> i32 {
    if evidence.is_empty() {
        return 0;
    }

    let now = chrono::Utc::now();
    let mut weighted_support = 0.0f64;
    let mut total_weight = 0.0f64;

    for obs in evidence {
        let age_days = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&obs.date) {
            let duration = now.signed_duration_since(dt);
            (duration.num_seconds() as f64 / 86400.0).max(0.0)
        } else {
            30.0 // default to 30 days if parse fails
        };

        let weight = memory_decay::decay_score(age_days, memory_decay::HALF_LIFE_PATTERNS);
        total_weight += weight;
        if obs.supports_pattern {
            weighted_support += weight;
        }
    }

    if total_weight == 0.0 {
        return 0;
    }

    let agreement_ratio = weighted_support / total_weight;
    // Use effective count (sum of decay weights) instead of raw count
    // so older observations contribute less to confidence buildup
    let effective_count = total_weight;
    let count_factor = 1.0 - (-0.3 * effective_count).exp();
    let confidence = (agreement_ratio * count_factor * 100.0).clamp(0.0, 95.0);
    confidence as i32
}

fn format_patterns(data: &PatternsData, detailed: bool) -> String {
    if data.patterns.is_empty() {
        return "No patterns recorded yet.".into();
    }

    let mut out = String::new();
    out.push_str(&format!("Total patterns: {}\n\n", data.patterns.len()));

    for pattern in &data.patterns {
        let status = if pattern.confidence >= 70 {
            "✓"
        } else if pattern.confidence >= 40 {
            "~"
        } else {
            "?"
        };

        out.push_str(&format!(
            "{} {} ({}%) - {} observations\n",
            status, pattern.name, pattern.confidence, pattern.observations_count
        ));

        if detailed {
            out.push_str(&format!("   ID: {}\n", pattern.id));
            out.push_str(&format!("   Description: {}\n", pattern.description));
            if let Some(ref hypothesis) = pattern.hypothesis {
                out.push_str(&format!("   Hypothesis: {}\n", hypothesis));
            }
            out.push_str("\n");
        }
    }

    out
}

// --- ReadPatternsTool ---

pub struct ReadPatternsTool {
    data_dir: PathBuf,
}

impl ReadPatternsTool {
    pub fn new(data_dir: &str) -> Self {
        ReadPatternsTool {
            data_dir: PathBuf::from(data_dir),
        }
    }
}

#[async_trait]
impl Tool for ReadPatternsTool {
    fn name(&self) -> &str {
        "read_patterns"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_patterns".into(),
            description: "Read the ADHD pattern learning data. Shows all 18 pattern categories with confidence scores and observation counts. Use 'detailed: true' to see full descriptions and hypotheses.".into(),
            input_schema: schema_object(
                json!({
                    "detailed": {
                        "type": "boolean",
                        "description": "Show full details including descriptions and hypotheses"
                    }
                }),
                &[],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let detailed = input.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

        info!("Reading patterns data");
        let data = read_patterns(&self.data_dir);
        ToolResult::success(format_patterns(&data, detailed))
    }
}

// --- AddObservationTool ---

pub struct AddObservationTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl AddObservationTool {
    pub fn new(data_dir: &str) -> Self {
        AddObservationTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for AddObservationTool {
    fn name(&self) -> &str {
        "add_observation"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "add_observation".into(),
            description: "Add an observation to a pattern category. Record what you learned about the user's behavior. The observation will be stored with a timestamp and context. If the pattern doesn't exist, suggest creating it first.".into(),
            input_schema: schema_object(
                json!({
                    "pattern_id": {
                        "type": "string",
                        "description": "The pattern ID (e.g., 'procrastination', 'energy', 'focus')"
                    },
                    "observation": {
                        "type": "string",
                        "description": "What you observed about the user's behavior"
                    },
                    "context": {
                        "type": "string",
                        "description": "The situation or context where this was observed"
                    },
                    "supports_pattern": {
                        "type": "boolean",
                        "description": "Whether this observation supports (true) or contradicts (false) the pattern. Default: true."
                    }
                }),
                &["pattern_id", "observation"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let pattern_id = match input.get("pattern_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => return ToolResult::error("Missing 'pattern_id' parameter".into()),
        };

        let observation_text = match input.get("observation").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'observation' parameter".into()),
        };

        let context = input
            .get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("General context")
            .to_string();

        let supports_pattern = input
            .get("supports_pattern")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut data = read_patterns(&self.data_dir);

        let pattern = match data.patterns.iter_mut().find(|p| p.id == pattern_id) {
            Some(p) => p,
            None => {
                return ToolResult::error(format!(
                    "Pattern '{}' not found. Use read_patterns to see available patterns, or create_pattern to add a new one.",
                    pattern_id
                ));
            }
        };

        let observation = Observation {
            date: chrono::Utc::now().to_rfc3339(),
            observation: observation_text,
            context,
            supports_pattern,
        };

        pattern.evidence.push(observation);
        pattern.observations_count = pattern.evidence.len() as i32;
        pattern.last_updated = chrono::Utc::now().to_rfc3339();

        // Update confidence: use decay-weighted model unless manually locked
        if !pattern.confidence_locked {
            pattern.confidence = compute_confidence(&pattern.evidence);
        }

        // Save values before borrow ends
        let pattern_name = pattern.name.clone();
        let obs_count = pattern.observations_count;
        let confidence = pattern.confidence;

        info!("Added observation to pattern '{}' (now {} observations, {}% confidence)",
              pattern_id, obs_count, confidence);

        // Log activity
        self.activity_logger.log_observation_added(&pattern_id, &pattern_name);

        match write_patterns(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "Observation added to '{}' ({}). Total observations: {}, confidence: {}%",
                pattern_name, pattern_id, obs_count, confidence
            )),
            Err(e) => ToolResult::error(format!("Failed to save observation: {e}")),
        }
    }
}

// --- UpdateHypothesisTool ---

pub struct UpdateHypothesisTool {
    data_dir: PathBuf,
}

impl UpdateHypothesisTool {
    pub fn new(data_dir: &str) -> Self {
        UpdateHypothesisTool {
            data_dir: PathBuf::from(data_dir),
        }
    }
}

#[async_trait]
impl Tool for UpdateHypothesisTool {
    fn name(&self) -> &str {
        "update_hypothesis"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "update_hypothesis".into(),
            description: "Update the hypothesis for a pattern based on accumulated observations. Formulate what you believe is true about this pattern based on the evidence collected.".into(),
            input_schema: schema_object(
                json!({
                    "pattern_id": {
                        "type": "string",
                        "description": "The pattern ID"
                    },
                    "hypothesis": {
                        "type": "string",
                        "description": "Your hypothesis about this pattern based on observations"
                    },
                    "confidence": {
                        "type": "integer",
                        "description": "Confidence level 0-100 (optional, auto-calculated if not provided)"
                    }
                }),
                &["pattern_id", "hypothesis"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let pattern_id = match input.get("pattern_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => return ToolResult::error("Missing 'pattern_id' parameter".into()),
        };

        let hypothesis = match input.get("hypothesis").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'hypothesis' parameter".into()),
        };

        let mut data = read_patterns(&self.data_dir);

        let pattern = match data.patterns.iter_mut().find(|p| p.id == pattern_id) {
            Some(p) => p,
            None => {
                return ToolResult::error(format!(
                    "Pattern '{}' not found",
                    pattern_id
                ));
            }
        };

        pattern.hypothesis = Some(hypothesis);

        // Update confidence if provided — also lock it so auto-calculation doesn't override
        if let Some(confidence) = input.get("confidence").and_then(|v| v.as_i64()) {
            pattern.confidence = confidence as i32;
            pattern.confidence_locked = true;
        }

        pattern.last_updated = chrono::Utc::now().to_rfc3339();

        // Save values before borrow ends
        let pattern_name = pattern.name.clone();
        let confidence = pattern.confidence;

        info!("Updated hypothesis for pattern '{}'", pattern_id);

        match write_patterns(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "Hypothesis updated for '{}' ({}% confidence)",
                pattern_name, confidence
            )),
            Err(e) => ToolResult::error(format!("Failed to update hypothesis: {e}")),
        }
    }
}

// --- CreatePatternTool ---

pub struct CreatePatternTool {
    data_dir: PathBuf,
    activity_logger: Arc<ActivityLogger>,
}

impl CreatePatternTool {
    pub fn new(data_dir: &str) -> Self {
        CreatePatternTool {
            data_dir: PathBuf::from(data_dir),
            activity_logger: Arc::new(ActivityLogger::new(data_dir)),
        }
    }
}

#[async_trait]
impl Tool for CreatePatternTool {
    fn name(&self) -> &str {
        "create_pattern"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_pattern".into(),
            description: "Create a new pattern category. Use this when you observe a behavior that doesn't fit existing patterns. The new pattern will start with 0 confidence and can accumulate observations over time.".into(),
            input_schema: schema_object(
                json!({
                    "id": {
                        "type": "string",
                        "description": "Unique identifier (lowercase, no spaces, use underscores)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Display name of the pattern"
                    },
                    "description": {
                        "type": "string",
                        "description": "What this pattern tracks and why it matters"
                    },
                    "category": {
                        "type": "string",
                        "description": "Broad category (e.g., 'cognitive', 'emotional', 'productivity', 'physical')"
                    }
                }),
                &["id", "name", "description"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let id = match input.get("id").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'id' parameter".into()),
        };

        let name = match input.get("name").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'name' parameter".into()),
        };

        let description = match input.get("description").and_then(|v| v.as_str()) {
            Some(text) => text.to_string(),
            None => return ToolResult::error("Missing 'description' parameter".into()),
        };

        let category = input
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();

        let mut data = read_patterns(&self.data_dir);

        // Check if pattern already exists
        if data.patterns.iter().any(|p| p.id == id) {
            return ToolResult::error(format!(
                "Pattern '{}' already exists. Use read_patterns to see it.",
                id
            ));
        }

        let now = chrono::Utc::now().to_rfc3339();
        let pattern_name = name.clone();
        let new_pattern = Pattern {
            id: id.clone(),
            name,
            description,
            category,
            confidence: 0,
            observations_count: 0,
            hypothesis: None,
            evidence: vec![],
            created_at: now.clone(),
            last_updated: now,
            confidence_locked: false,
        };

        data.patterns.push(new_pattern);
        data.meta.total_patterns = data.patterns.len() as i32;

        info!("Created new pattern '{}' (total: {} patterns)", id, data.meta.total_patterns);

        // Log activity
        self.activity_logger.log_pattern_created(&id, &pattern_name);

        match write_patterns(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "New pattern '{}' ({}) created successfully. Total patterns: {}",
                id, pattern_name, data.meta.total_patterns
            )),
            Err(e) => ToolResult::error(format!("Failed to create pattern: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("sandy_patterns_test_{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_patterns_path_points_to_runtime() {
        let path = patterns_path(Path::new("/data"));
        assert!(path.ends_with("runtime/patterns.json"));
    }

    #[test]
    fn test_read_patterns_empty() {
        let dir = test_dir();
        let data = read_patterns(&dir);
        assert!(data.patterns.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn test_corrupt_patterns_file_logs_error() {
        let dir = test_dir();
        let path = patterns_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{"bad":"#).unwrap();
        let data = read_patterns(&dir);
        // Should return defaults without panicking
        assert!(data.patterns.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn test_migrate_legacy_object_format() {
        let legacy_json = r#"{
            "version": "1.0",
            "user_id": "jens",
            "patterns": {
                "focus": {
                    "name": "Focus Patterns",
                    "description": "Tracks focus",
                    "category": "cognitive",
                    "confidence": 40,
                    "observations_count": 3,
                    "hypothesis": null,
                    "evidence": [],
                    "created_at": "2026-01-01T00:00:00Z",
                    "last_updated": "2026-02-01T00:00:00Z"
                }
            }
        }"#;

        let migrated = try_migrate_legacy(legacy_json).unwrap();
        assert_eq!(migrated.patterns.len(), 1);
        assert_eq!(migrated.patterns[0].id, "focus");
        assert_eq!(migrated.patterns[0].name, "Focus Patterns");
        assert_eq!(migrated.patterns[0].confidence, 40);
        assert!(!migrated.patterns[0].confidence_locked);
    }

    #[test]
    fn test_migrate_legacy_returns_none_for_array_format() {
        let array_json = r#"{"version":"1.0","user_id":"test","patterns":[],"meta":{"total_patterns":0,"last_analysis":"","learning_active":true,"notes":null}}"#;
        assert!(try_migrate_legacy(array_json).is_none());
    }

    #[test]
    fn test_write_patterns_atomic_roundtrip() {
        let dir = test_dir();
        let data = PatternsData {
            version: "1.0".into(),
            user_id: "test".into(),
            patterns: vec![Pattern {
                id: "test".into(),
                name: "Test".into(),
                description: "desc".into(),
                category: "general".into(),
                confidence: 50,
                observations_count: 1,
                hypothesis: None,
                evidence: vec![],
                created_at: "2026-01-01T00:00:00Z".into(),
                last_updated: "2026-01-01T00:00:00Z".into(),
                confidence_locked: false,
            }],
            meta: PatternsMeta {
                total_patterns: 1,
                last_analysis: "".into(),
                learning_active: true,
                notes: None,
            },
        };
        write_patterns(&dir, &data).unwrap();
        let loaded = read_patterns(&dir);
        assert_eq!(loaded.patterns.len(), 1);
        assert_eq!(loaded.patterns[0].id, "test");
        cleanup(&dir);
    }

    #[test]
    fn test_observation_deserialize_without_supports_field() {
        let json = r#"{"date":"2026-01-01","observation":"test","context":"ctx"}"#;
        let obs: Observation = serde_json::from_str(json).unwrap();
        assert!(obs.supports_pattern); // defaults true
    }

    #[test]
    fn test_observation_with_supports_false_roundtrips() {
        let obs = Observation {
            date: "2026-01-01".into(),
            observation: "test".into(),
            context: "ctx".into(),
            supports_pattern: false,
        };
        let json = serde_json::to_string(&obs).unwrap();
        let loaded: Observation = serde_json::from_str(&json).unwrap();
        assert!(!loaded.supports_pattern);
    }

    #[test]
    fn test_confidence_empty() {
        assert_eq!(compute_confidence(&[]), 0);
    }

    #[test]
    fn test_confidence_one_recent_supporting() {
        let obs = vec![Observation {
            date: chrono::Utc::now().to_rfc3339(),
            observation: "test".into(),
            context: "ctx".into(),
            supports_pattern: true,
        }];
        let c = compute_confidence(&obs);
        assert!(c > 20 && c < 35, "Expected ~26, got {}", c);
    }

    #[test]
    fn test_confidence_ten_supporting() {
        let obs: Vec<Observation> = (0..10)
            .map(|_| Observation {
                date: chrono::Utc::now().to_rfc3339(),
                observation: "test".into(),
                context: "ctx".into(),
                supports_pattern: true,
            })
            .collect();
        let c = compute_confidence(&obs);
        assert!(c >= 90, "Expected >=90 for 10 supporting, got {}", c);
    }

    #[test]
    fn test_confidence_contradictions_lower_score() {
        let supporting: Vec<Observation> = (0..5)
            .map(|_| Observation {
                date: chrono::Utc::now().to_rfc3339(),
                observation: "supports".into(),
                context: "ctx".into(),
                supports_pattern: true,
            })
            .collect();

        let mut mixed = supporting.clone();
        mixed.push(Observation {
            date: chrono::Utc::now().to_rfc3339(),
            observation: "contradicts".into(),
            context: "ctx".into(),
            supports_pattern: false,
        });

        let c_pure = compute_confidence(&supporting);
        let c_mixed = compute_confidence(&mixed);
        assert!(c_mixed < c_pure, "Mixed {} should be less than pure {}", c_mixed, c_pure);
    }

    #[test]
    fn test_confidence_old_observations_decay() {
        let recent = vec![Observation {
            date: chrono::Utc::now().to_rfc3339(),
            observation: "recent".into(),
            context: "ctx".into(),
            supports_pattern: true,
        }];

        let old_date = (chrono::Utc::now() - chrono::Duration::days(120)).to_rfc3339();
        let old = vec![Observation {
            date: old_date,
            observation: "old".into(),
            context: "ctx".into(),
            supports_pattern: true,
        }];

        let c_recent = compute_confidence(&recent);
        let c_old = compute_confidence(&old);
        assert!(c_recent > c_old, "Recent {} should exceed old {}", c_recent, c_old);
    }

    #[test]
    fn test_manual_confidence_preserved_after_observation() {
        let dir = test_dir();
        // Create initial data with a locked pattern
        let data = PatternsData {
            version: "1.0".into(),
            user_id: "test".into(),
            patterns: vec![Pattern {
                id: "locked".into(),
                name: "Locked".into(),
                description: "test".into(),
                category: "general".into(),
                confidence: 80,
                observations_count: 0,
                hypothesis: Some("manual".into()),
                evidence: vec![],
                created_at: "2026-01-01T00:00:00Z".into(),
                last_updated: "2026-01-01T00:00:00Z".into(),
                confidence_locked: true,
            }],
            meta: PatternsMeta {
                total_patterns: 1,
                last_analysis: "".into(),
                learning_active: true,
                notes: None,
            },
        };
        write_patterns(&dir, &data).unwrap();

        // Simulate adding an observation to the locked pattern
        let mut data = read_patterns(&dir);
        let pattern = data.patterns.iter_mut().find(|p| p.id == "locked").unwrap();
        pattern.evidence.push(Observation {
            date: chrono::Utc::now().to_rfc3339(),
            observation: "new obs".into(),
            context: "test".into(),
            supports_pattern: true,
        });
        pattern.observations_count = pattern.evidence.len() as i32;
        // Mimic AddObservationTool logic: skip compute if locked
        if !pattern.confidence_locked {
            pattern.confidence = compute_confidence(&pattern.evidence);
        }
        assert_eq!(pattern.confidence, 80, "Locked confidence should not change");
        cleanup(&dir);
    }

    #[test]
    fn test_add_observation_schema_includes_supports_pattern() {
        let dir = test_dir();
        let tool = AddObservationTool::new(dir.to_str().unwrap());
        let def = tool.definition();
        let props = &def.input_schema["properties"];
        assert!(props.get("supports_pattern").is_some());
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_create_pattern() {
        let dir = test_dir();
        let tool = CreatePatternTool::new(dir.to_str().unwrap());

        let result = tool.execute(json!({
            "id": "test_pattern",
            "name": "Test Pattern",
            "description": "A test pattern",
            "category": "test"
        })).await;

        assert!(!result.is_error);
        assert!(result.content.contains("Test Pattern"));

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_add_observation() {
        let dir = test_dir();

        // First create a pattern
        let create_tool = CreatePatternTool::new(dir.to_str().unwrap());
        create_tool.execute(json!({
            "id": "focus",
            "name": "Focus Patterns",
            "description": "Testing focus"
        })).await;

        // Then add observation
        let add_tool = AddObservationTool::new(dir.to_str().unwrap());
        let result = add_tool.execute(json!({
            "pattern_id": "focus",
            "observation": "User focuses better in mornings",
            "context": "Morning conversation"
        })).await;

        assert!(!result.is_error);
        assert!(result.content.contains("focus"));

        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_add_contradicting_observation_lowers_confidence() {
        let dir = test_dir();
        let create_tool = CreatePatternTool::new(dir.to_str().unwrap());
        create_tool.execute(json!({
            "id": "test_contra",
            "name": "Contradiction Test",
            "description": "Testing contradictions"
        })).await;

        let add_tool = AddObservationTool::new(dir.to_str().unwrap());

        // Add 3 supporting observations
        for _ in 0..3 {
            add_tool.execute(json!({
                "pattern_id": "test_contra",
                "observation": "supports pattern",
                "supports_pattern": true
            })).await;
        }

        let data = read_patterns(&dir);
        let conf_before = data.patterns.iter().find(|p| p.id == "test_contra").unwrap().confidence;

        // Add 1 contradicting
        add_tool.execute(json!({
            "pattern_id": "test_contra",
            "observation": "contradicts pattern",
            "supports_pattern": false
        })).await;

        let data = read_patterns(&dir);
        let conf_after = data.patterns.iter().find(|p| p.id == "test_contra").unwrap().confidence;

        assert!(conf_after < conf_before,
            "Confidence should drop after contradiction: before={}, after={}", conf_before, conf_after);

        cleanup(&dir);
    }
}
