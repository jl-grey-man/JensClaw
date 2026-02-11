use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use crate::activity::ActivityLogger;
use crate::claude::ToolDefinition;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub date: String,
    pub observation: String,
    pub context: String,
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
    data_dir.join("patterns.json")
}

fn read_patterns(data_dir: &Path) -> PatternsData {
    let path = patterns_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| default_patterns()),
        Err(_) => default_patterns(),
    }
}

fn write_patterns(data_dir: &Path, data: &PatternsData) -> std::io::Result<()> {
    let path = patterns_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
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
        };

        pattern.evidence.push(observation);
        pattern.observations_count = pattern.evidence.len() as i32;
        pattern.last_updated = chrono::Utc::now().to_rfc3339();

        // Auto-update confidence based on observation count
        pattern.confidence = match pattern.observations_count {
            0..=2 => (pattern.observations_count * 10).min(25),
            3..=5 => 30 + (pattern.observations_count - 3) * 10,
            6..=10 => 50 + (pattern.observations_count - 6) * 5,
            _ => 70 + ((pattern.observations_count - 10) * 2).min(30),
        }.min(95); // Cap at 95%

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
        
        // Update confidence if provided
        if let Some(confidence) = input.get("confidence").and_then(|v| v.as_i64()) {
            pattern.confidence = confidence as i32;
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
        };

        data.patterns.push(new_pattern);
        data.meta.total_patterns = data.patterns.len() as i32;

        info!("Created new pattern '{}' (total: {} patterns)", id, data.meta.total_patterns);

        // Log activity
        self.activity_logger.log_pattern_created(&id, &pattern_name);

        match write_patterns(&self.data_dir, &data) {
            Ok(()) => ToolResult::success(format!(
                "New pattern '{}' created successfully. Total patterns: {}",
                id, data.meta.total_patterns
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
    fn test_read_patterns_empty() {
        let dir = test_dir();
        let data = read_patterns(&dir);
        assert!(data.patterns.is_empty());
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
}
