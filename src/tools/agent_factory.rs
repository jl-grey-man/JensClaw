//! Agent Factory - Create Agent Configurations Safely
//!
//! This module provides the `create_agent_config` tool that creates agent JSON
//! configurations with validated tool whitelisting. Prevents agents from being
//! created with unauthorized or hallucinated tools.
//!
//! The agent factory enforces the Hard Rails architecture by:
//! - Validating agent_id format (alphanumeric + hyphens only)
//! - Filtering tools against whitelist (anti-hallucination)
//! - Writing valid JSON to storage/agents/{agent_id}.json
//! - Rejecting invalid tool names

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;
use std::path::Path;

use super::{schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;

/// Whitelist of valid tools that agents can use
/// This is the anti-hallucination guard - only these tools are allowed
const ALLOWED_TOOLS: &[&str] = &[
    "web_search",
    "web_fetch",
    "read_file",
    "write_file",
    "edit_file",
    "bash",
    "glob",
    "grep",
    "list_files",
    "verify_file_exists",
];

/// Predefined agent templates for common use cases
const PREDEFINED_TEMPLATES: &[(&str, &str, &[&str])] = &[
    (
        "zilla",
        "Journalistic Researcher - performs web research and gathers data",
        &["web_search", "web_fetch", "write_file", "read_file", "bash"],
    ),
    (
        "gonza",
        "Journalistic Writer - transforms research into articles",
        &["read_file", "write_file"],
    ),
    (
        "file-organizer",
        "File Organizer - organizes and manages files",
        &["read_file", "write_file", "bash", "glob", "list_files"],
    ),
    (
        "code-assistant",
        "Code Assistant - helps with coding tasks",
        &["read_file", "write_file", "edit_file", "bash", "grep"],
    ),
];

pub struct AgentFactoryTool {
    storage_dir: String,
}

impl AgentFactoryTool {
    pub fn new(storage_dir: &str) -> Self {
        Self {
            storage_dir: storage_dir.to_string(),
        }
    }

    /// Validate agent_id format (alphanumeric + hyphens only)
    fn validate_agent_id(&self, agent_id: &str) -> Result<(), String> {
        if agent_id.is_empty() {
            return Err("agent_id cannot be empty".to_string());
        }

        if agent_id.len() > 50 {
            return Err("agent_id too long (max 50 characters)".to_string());
        }

        // Check format: lowercase alphanumeric + hyphens only
        if !agent_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(
                "Invalid agent_id format. Use only lowercase letters, numbers, and hyphens. Example: 'research-agent-1'".to_string(),
            );
        }

        // Cannot start or end with hyphen
        if agent_id.starts_with('-') || agent_id.ends_with('-') {
            return Err("agent_id cannot start or end with hyphen".to_string());
        }

        // No consecutive hyphens
        if agent_id.contains("--") {
            return Err("agent_id cannot contain consecutive hyphens".to_string());
        }

        Ok(())
    }

    /// Filter and validate tools against whitelist
    fn validate_tools(&self, tools: &[String]) -> Result<Vec<String>, String> {
        let allowed_set: HashSet<&str> = ALLOWED_TOOLS.iter().copied().collect();
        let mut valid_tools = Vec::new();
        let mut invalid_tools = Vec::new();

        for tool in tools {
            if allowed_set.contains(tool.as_str()) {
                valid_tools.push(tool.clone());
            } else {
                invalid_tools.push(tool.clone());
            }
        }

        if !invalid_tools.is_empty() {
            return Err(format!(
                "Invalid tools requested: {}. Allowed tools: {}",
                invalid_tools.join(", "),
                ALLOWED_TOOLS.join(", ")
            ));
        }

        if valid_tools.is_empty() {
            return Err("At least one tool must be specified".to_string());
        }

        // Remove duplicates while preserving order
        let mut seen = HashSet::new();
        valid_tools.retain(|t| seen.insert(t.clone()));

        Ok(valid_tools)
    }

    /// Get template by name
    fn get_template(&self, template_name: &str) -> Option<(&'static str, &'static str, &'static [&'static str])> {
        PREDEFINED_TEMPLATES
            .iter()
            .find(|(name, _, _)| *name == template_name)
            .copied()
    }

    /// Build agent config JSON
    fn build_agent_config(
        &self,
        agent_id: &str,
        name: &str,
        role: &str,
        tools: Vec<String>,
        constraints: Option<Vec<String>>,
    ) -> serde_json::Value {
        let default_constraints: Vec<String> = vec![
            "CANNOT modify system files or configuration".to_string(),
            "MUST save all output to specified files".to_string(),
            "MUST report errors to output files, not just log them".to_string(),
        ];

        json!({
            "id": agent_id,
            "name": name,
            "role": role,
            "description": format!("Specialized agent for {}", role.to_lowercase()),
            "tools": tools,
            "constraints": constraints.unwrap_or(default_constraints),
            "output_format": "structured_json",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "version": "1.0",
        })
    }

    /// Save agent config to storage
    async fn save_agent_config(
        &self,
        agent_id: &str,
        config: serde_json::Value,
    ) -> Result<std::path::PathBuf, String> {
        let agents_dir = Path::new(&self.storage_dir).join("agents");
        
        // Create agents directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(&agents_dir).await {
            return Err(format!(
                "Failed to create agents directory '{}': {}",
                agents_dir.display(),
                e
            ));
        }

        let config_path = agents_dir.join(format!("{}.json", agent_id));

        // Write config file
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize agent config: {}", e))?;

        if let Err(e) = tokio::fs::write(&config_path, config_json).await {
            return Err(format!(
                "Failed to write agent config to '{}': {}",
                config_path.display(),
                e
            ));
        }

        Ok(config_path)
    }
}

#[async_trait]
impl Tool for AgentFactoryTool {
    fn name(&self) -> &str {
        "create_agent_config"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_agent_config".into(),
            description: "Create a new agent configuration with validated tool whitelisting. Prevents unauthorized tools and ensures agents follow the Hard Rails architecture.".into(),
            input_schema: schema_object(
                json!({
                    "agent_id": {
                        "type": "string",
                        "description": "Unique agent ID (e.g., 'zilla', 'research-agent-1'). Use lowercase letters, numbers, and hyphens only."
                    },
                    "name": {
                        "type": "string",
                        "description": "Display name for the agent (e.g., 'Zilla', 'Research Agent')"
                    },
                    "role": {
                        "type": "string",
                        "description": "Agent's role/purpose (e.g., 'Journalistic Researcher', 'Code Assistant')"
                    },
                    "tools": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": format!("Tools this agent can use. Allowed: {}", ALLOWED_TOOLS.join(", "))
                    },
                    "template": {
                        "type": "string",
                        "description": "(Optional) Use a predefined template: 'zilla', 'gonza', 'file-organizer', 'code-assistant'. If provided, tools and role are auto-configured.",
                        "default": null
                    },
                    "constraints": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "(Optional) Additional constraints for this agent. Default constraints always applied.",
                        "default": null
                    }
                }),
                &["agent_id", "name", "role", "tools"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let agent_id = match input.get("agent_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: agent_id".into()),
        };

        // Validate agent_id format
        if let Err(e) = self.validate_agent_id(agent_id) {
            return ToolResult::error(format!("Invalid agent_id '{}': {}", agent_id, e));
        }

        // Check for template usage
        let template_name = input.get("template").and_then(|v| v.as_str());

        let (name, role, tools): (String, String, Vec<String>);

        if let Some(template_name) = template_name {
            // Use template
            match self.get_template(template_name) {
                Some((tpl_name, tpl_role, tpl_tools)) => {
                    name = input
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| tpl_name.to_string());
                    role = input
                        .get("role")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| tpl_role.to_string());
                    tools = tpl_tools.iter().map(|&t| t.to_string()).collect();
                }
                None => {
                    let available = PREDEFINED_TEMPLATES.iter().map(|(n, _, _)| *n).collect::<Vec<_>>().join(", ");
                    return ToolResult::error(format!(
                        "Unknown template '{}'. Available templates: {}",
                        template_name, available
                    ));
                }
            }
        } else {
            // Manual configuration
            name = match input.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_string(),
                None => return ToolResult::error("Missing required parameter: name".into()),
            };

            role = match input.get("role").and_then(|v| v.as_str()) {
                Some(r) => r.to_string(),
                None => return ToolResult::error("Missing required parameter: role".into()),
            };

            tools = match input.get("tools").and_then(|v| v.as_array()) {
                Some(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                None => return ToolResult::error("Missing required parameter: tools".into()),
            };
        }

        // Validate tools against whitelist
        let validated_tools = match self.validate_tools(&tools) {
            Ok(tools) => tools,
            Err(e) => return ToolResult::error(e),
        };

        // Get optional constraints
        let constraints = input.get("constraints").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
        });

        // Build agent config
        let config = self.build_agent_config(agent_id, &name, &role, validated_tools, constraints);

        // Save to storage
        let config_path = match self.save_agent_config(agent_id, config).await {
            Ok(path) => path,
            Err(e) => return ToolResult::error(e),
        };

        // Build success message
        let tools_list = if let Some(tools_array) = input.get("tools").and_then(|v| v.as_array()) {
            tools_array
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        } else if template_name.is_some() {
            tools.join(", ")
        } else {
            String::new()
        };

        let template_msg = if let Some(tpl) = template_name {
            format!("\nTemplate used: {}", tpl)
        } else {
            String::new()
        };

        ToolResult::success(format!(
            "✅ Agent '{}' created successfully!\n\nName: {}\nRole: {}\nTools: {}{}\n\nConfig saved to: {}",
            agent_id,
            name,
            role,
            tools_list,
            template_msg,
            config_path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_factory_basic() {
        let temp_dir = std::env::temp_dir().join(format!("agent_factory_test_{}", uuid::Uuid::new_v4()));
        let factory = AgentFactoryTool::new(temp_dir.to_str().unwrap());

        // Test basic agent creation
        let input = json!({
            "agent_id": "test-agent",
            "name": "Test Agent",
            "role": "Tester",
            "tools": ["read_file", "write_file"]
        });

        let result = factory.execute(input).await;
        assert!(!result.is_error, "Should succeed with valid input");
        assert!(result.content.contains("created successfully"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_agent_factory_invalid_id() {
        let factory = AgentFactoryTool::new("/tmp");

        // Test invalid agent_id (spaces)
        let input = json!({
            "agent_id": "invalid id",
            "name": "Test",
            "role": "Tester",
            "tools": ["read_file"]
        });

        let result = factory.execute(input).await;
        assert!(result.is_error, "Should fail with invalid agent_id");
        assert!(result.content.contains("Invalid agent_id"));
    }

    #[tokio::test]
    async fn test_agent_factory_invalid_tools() {
        let factory = AgentFactoryTool::new("/tmp");

        // Test invalid tools (hallucinated tool)
        let input = json!({
            "agent_id": "test-agent",
            "name": "Test",
            "role": "Tester",
            "tools": ["read_file", "hallucinated_tool"]
        });

        let result = factory.execute(input).await;
        assert!(result.is_error, "Should fail with invalid tools");
        assert!(result.content.contains("Invalid tools"));
    }

    #[tokio::test]
    async fn test_agent_factory_template() {
        let temp_dir = std::env::temp_dir().join(format!("agent_factory_test_{}", uuid::Uuid::new_v4()));
        let factory = AgentFactoryTool::new(temp_dir.to_str().unwrap());

        // Test using template
        let input = json!({
            "agent_id": "my-researcher",
            "name": "My Researcher",
            "template": "zilla"
        });

        let result = factory.execute(input).await;
        assert!(!result.is_error, "Should succeed with template");
        assert!(result.content.contains("zilla"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
