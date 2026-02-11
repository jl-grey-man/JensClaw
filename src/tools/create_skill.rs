use async_trait::async_trait;
use serde_json::json;
use std::path::Path;

use super::{schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;

pub struct CreateSkillTool {
    skills_dir: String,
}

impl CreateSkillTool {
    pub fn new(skills_data_dir: &str) -> Self {
        Self {
            skills_dir: format!("{}/custom", skills_data_dir),
        }
    }
}

#[async_trait]
impl Tool for CreateSkillTool {
    fn name(&self) -> &str {
        "create_skill"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_skill".into(),
            description: "Create a new custom skill for Sandy. Skills are reusable workflows and instructions stored in soul/data/skills/custom/.".into(),
            input_schema: schema_object(
                json!({
                    "skill_name": {
                        "type": "string",
                        "description": "Skill name in lowercase-with-hyphens format (e.g., 'morning-routine', 'research-assistant')"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of what this skill does and when to use it"
                    },
                    "content": {
                        "type": "string",
                        "description": "The SKILL.md content (markdown format with instructions, examples, etc.)"
                    }
                }),
                &["skill_name", "description", "content"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let skill_name = match input.get("skill_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return ToolResult::error("Missing required parameter: skill_name".into()),
        };

        let description = match input.get("description").and_then(|v| v.as_str()) {
            Some(desc) => desc,
            None => return ToolResult::error("Missing required parameter: description".into()),
        };

        let content = match input.get("content").and_then(|v| v.as_str()) {
            Some(content) => content,
            None => return ToolResult::error("Missing required parameter: content".into()),
        };

        // Validate skill name format
        if !skill_name.chars().all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()) {
            return ToolResult::error(
                "Invalid skill_name format. Use only lowercase letters, numbers, and hyphens. Examples: 'morning-routine', 'research-assistant', 'file-organizer'".into()
            );
        }

        // Create skill directory
        let skill_dir = Path::new(&self.skills_dir).join(skill_name);
        if let Err(e) = std::fs::create_dir_all(&skill_dir) {
            return ToolResult::error(format!(
                "Failed to create skill directory '{}': {}",
                skill_dir.display(),
                e
            ));
        }

        // Build SKILL.md content
        let skill_md = format!(
            "---\nname: {}\ndescription: {}\nlicense: MIT\n---\n\n{}",
            skill_name, description, content
        );

        // Write SKILL.md
        let skill_md_path = skill_dir.join("SKILL.md");
        if let Err(e) = std::fs::write(&skill_md_path, skill_md) {
            return ToolResult::error(format!(
                "Failed to write SKILL.md to '{}': {}",
                skill_md_path.display(),
                e
            ));
        }

        ToolResult::success(format!(
            "✅ Skill '{}' created successfully!\n\nLocation: {}\n\nTo use this skill, say:\n• 'Use my {} skill'\n• 'Activate {}'\n• 'Help me with {}'",
            skill_name,
            skill_md_path.display(),
            skill_name, skill_name, skill_name
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_skill_definition() {
        let tool = CreateSkillTool::new("/tmp/skills");
        assert_eq!(tool.name(), "create_skill");
        let def = tool.definition();
        assert_eq!(def.name, "create_skill");
        assert!(def.description.contains("custom skill"));
    }

    #[tokio::test]
    async fn test_create_skill_missing_params() {
        let tool = CreateSkillTool::new("/tmp/skills");
        let result = tool.execute(json!({})).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn test_create_skill_invalid_name() {
        let tool = CreateSkillTool::new("/tmp/skills");
        let result = tool.execute(json!({
            "skill_name": "Invalid Name With Spaces",
            "description": "Test skill",
            "content": "# Test"
        })).await;
        assert!(result.is_error);
        assert!(result.content.contains("Invalid skill_name format"));
    }
}
