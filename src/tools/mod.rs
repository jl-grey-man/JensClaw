pub mod activate_skill;
pub mod agent_factory;
pub mod agent_management;
pub mod bash;
pub mod execute_workflow;
pub mod browser;
pub mod create_skill;
pub mod doctor;
pub mod edit_file;
pub mod file_ops;
pub mod export_chat;
pub mod glob;
pub mod grep;
pub mod mcp;
pub mod memory;
pub mod memory_log;
pub mod memory_search;
pub mod parse_datetime;
pub mod path_guard;
pub mod patterns;
pub mod read_file;
pub mod schedule;
pub mod send_file;
pub mod send_message;
pub mod sub_agent;
pub mod tool_filter;
pub mod todo;
pub mod tracking;
pub mod transcribe;
pub mod web_fetch;
pub mod web_search;
pub mod write_file;

use std::sync::Arc;
use std::{path::Path, path::PathBuf};

use async_trait::async_trait;
use serde_json::json;
use teloxide::prelude::*;

use crate::claude::ToolDefinition;
use crate::config::Config;
use crate::db::Database;
use crate::hooks::HookRegistry;

#[derive(Debug)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(content: String) -> Self {
        ToolResult {
            content,
            is_error: false,
        }
    }

    pub fn error(content: String) -> Self {
        ToolResult {
            content,
            is_error: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolAuthContext {
    pub caller_chat_id: i64,
    pub control_chat_ids: Vec<i64>,
}

impl ToolAuthContext {
    pub fn is_control_chat(&self) -> bool {
        self.control_chat_ids.contains(&self.caller_chat_id)
    }

    pub fn can_access_chat(&self, target_chat_id: i64) -> bool {
        self.is_control_chat() || self.caller_chat_id == target_chat_id
    }
}

const AUTH_CONTEXT_KEY: &str = "__sandy_auth";

pub fn auth_context_from_input(input: &serde_json::Value) -> Option<ToolAuthContext> {
    let ctx = input.get(AUTH_CONTEXT_KEY)?;
    let caller_chat_id = ctx.get("caller_chat_id")?.as_i64()?;
    let control_chat_ids = ctx
        .get("control_chat_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_i64()).collect())
        .unwrap_or_default();
    Some(ToolAuthContext {
        caller_chat_id,
        control_chat_ids,
    })
}

pub fn authorize_chat_access(input: &serde_json::Value, target_chat_id: i64) -> Result<(), String> {
    if let Some(auth) = auth_context_from_input(input) {
        if !auth.can_access_chat(target_chat_id) {
            return Err(format!(
                "Permission denied: chat {} cannot operate on chat {}",
                auth.caller_chat_id, target_chat_id
            ));
        }
    }
    Ok(())
}

pub fn inject_auth_context(input: serde_json::Value, auth: &ToolAuthContext) -> serde_json::Value {
    let mut obj = match input {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    obj.insert(
        AUTH_CONTEXT_KEY.to_string(),
        json!({
            "caller_chat_id": auth.caller_chat_id,
            "control_chat_ids": auth.control_chat_ids,
        }),
    );
    serde_json::Value::Object(obj)
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, input: serde_json::Value) -> ToolResult;
}

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    is_main_bot: bool,  // true for Sandy's main registry, false for sub-agents
    hooks: Arc<HookRegistry>,
}

pub fn resolve_tool_path(working_dir: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        working_dir.join(candidate)
    }
}

impl ToolRegistry {
    /// Create an empty tool registry (for building filtered registries)
    pub fn empty() -> Self {
        ToolRegistry {
            tools: Vec::new(),
            is_main_bot: false,  // Filtered registries are for sub-agents
            hooks: Arc::new(HookRegistry::new()),
        }
    }

    pub fn new(config: &Config, bot: Bot, db: Arc<Database>) -> Self {
        let working_dir = PathBuf::from(&config.working_dir);
        if let Err(e) = std::fs::create_dir_all(&working_dir) {
            tracing::warn!(
                "Failed to create working_dir '{}': {}",
                working_dir.display(),
                e
            );
        }
        let skills_data_dir = config.skills_data_dir();
        let mut tools: Vec<Box<dyn Tool>> = vec![
            Box::new(agent_factory::AgentFactoryTool::new(&config.working_dir)),
            Box::new(agent_management::SpawnAgentTool::new(config)),
            Box::new(agent_management::ListAgentsTool::new()),
            Box::new(agent_management::AgentStatusTool::new()),
            Box::new(execute_workflow::ExecuteWorkflowTool::new(config)),
            Box::new(send_message::SendMessageTool::new(bot.clone())),
            Box::new(send_file::SendFileTool::new(bot.clone())),
            Box::new(schedule::ScheduleTaskTool::new(
                db.clone(),
                config.timezone.clone(),
                config.data_dir.clone(),
            )),
            Box::new(schedule::ListTasksTool::new(db.clone())),
            Box::new(schedule::PauseTaskTool::new(db.clone())),
            Box::new(schedule::ResumeTaskTool::new(db.clone())),
            Box::new(schedule::CancelTaskTool::new(db.clone())),
            Box::new(schedule::GetTaskHistoryTool::new(db.clone())),
            Box::new(bash::BashTool::new(&config.working_dir)),
            Box::new(browser::BrowserTool::new(&config.data_dir)),
            Box::new(transcribe::TranscribeTool::new(config.openai_api_key.clone().unwrap_or_default())),
            Box::new(read_file::ReadFileTool::new(&config.working_dir).with_data_dir(&config.data_dir)),
            Box::new(write_file::WriteFileTool::new(&config.working_dir).with_data_dir(&config.data_dir)),
            Box::new(edit_file::EditFileTool::new(&config.working_dir)),
            Box::new(glob::GlobTool::new(&config.working_dir)),
            Box::new(grep::GrepTool::new(&config.working_dir)),
            Box::new(memory::ReadMemoryTool::new(&config.data_dir)),
            Box::new(memory_search::MemorySearchTool::new(
                PathBuf::from(&config.data_dir).join("memory")
            )),
            Box::new(memory_log::MemoryLogTool::new(
                PathBuf::from(&config.data_dir).join("memory")
            )),
            // Pattern learning tools
            Box::new(patterns::ReadPatternsTool::new(&config.data_dir)),
            Box::new(patterns::CreatePatternTool::new(&config.data_dir)),
            Box::new(patterns::AddObservationTool::new(&config.data_dir)),
            Box::new(patterns::UpdateHypothesisTool::new(&config.data_dir)),
            Box::new(web_fetch::WebFetchTool),
            Box::new(web_search::WebSearchTool::new(config)),
            Box::new(activate_skill::ActivateSkillTool::new(&skills_data_dir)),
            // Task/Goal/Project tracking tools
            Box::new(tracking::ReadTrackingTool::new(&config.data_dir)),
            Box::new(tracking::CreateTaskTool::new(&config.data_dir)),
            Box::new(tracking::CreateGoalTool::new(&config.data_dir)),
            Box::new(tracking::CreateProjectTool::new(&config.data_dir)),
            Box::new(tracking::UpdateStatusTool::new(&config.data_dir)),
            Box::new(tracking::AddNoteTool::new(&config.data_dir)),
            Box::new(tracking::RemoveNoteTool::new(&config.data_dir)),
        ];

        // Add doctor tool with actual registered tool names (must come after tools vec is built)
        let tool_names: Vec<String> = tools.iter().map(|t| t.name().to_string()).collect();
        tools.push(Box::new(doctor::DoctorTool::new(config, tool_names)));

        // Debug: Log all registered tools
        tracing::info!("Registering {} tools:", tools.len());
        for tool in &tools {
            tracing::info!("  - {}", tool.name());
        }
        ToolRegistry {
            tools,
            is_main_bot: true,  // This is Sandy's main registry
            hooks: Arc::new(HookRegistry::new()),
        }
    }

    /// Create a restricted tool registry for sub-agents (no side-effect or recursive tools).
    pub fn new_sub_agent(config: &Config) -> Self {
        let working_dir = PathBuf::from(&config.working_dir);
        if let Err(e) = std::fs::create_dir_all(&working_dir) {
            tracing::warn!(
                "Failed to create working_dir '{}': {}",
                working_dir.display(),
                e
            );
        }
        let skills_data_dir = config.skills_data_dir();
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(bash::BashTool::new(&config.working_dir)),
            Box::new(browser::BrowserTool::new(&config.data_dir)),
            Box::new(read_file::ReadFileTool::new(&config.working_dir).with_data_dir(&config.data_dir)),
            Box::new(write_file::WriteFileTool::new(&config.working_dir).with_data_dir(&config.data_dir)),
            Box::new(edit_file::EditFileTool::new(&config.working_dir)),
            Box::new(glob::GlobTool::new(&config.working_dir)),
            Box::new(grep::GrepTool::new(&config.working_dir)),
            Box::new(memory::ReadMemoryTool::new(&config.data_dir)),
            Box::new(memory_search::MemorySearchTool::new(
                PathBuf::from(&config.data_dir).join("memory")
            )),
            Box::new(memory_log::MemoryLogTool::new(
                PathBuf::from(&config.data_dir).join("memory")
            )),
            // Pattern learning tools
            Box::new(patterns::ReadPatternsTool::new(&config.data_dir)),
            Box::new(patterns::CreatePatternTool::new(&config.data_dir)),
            Box::new(patterns::AddObservationTool::new(&config.data_dir)),
            Box::new(patterns::UpdateHypothesisTool::new(&config.data_dir)),
            Box::new(web_fetch::WebFetchTool),
            Box::new(web_search::WebSearchTool::new(config)),
            Box::new(activate_skill::ActivateSkillTool::new(&skills_data_dir)),
        ];
        ToolRegistry {
            tools,
            is_main_bot: false,  // This is for sub-agents, not main Sandy
            hooks: Arc::new(HookRegistry::new()),
        }
    }

    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Set the hook registry for this tool registry
    pub fn set_hooks(&mut self, hooks: HookRegistry) {
        self.hooks = Arc::new(hooks);
    }

    /// Get a reference to the hook registry
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    /// Get the number of tools in this registry
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Check if a tool with the given name exists in this registry
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name() == name)
    }

    /// Get a list of all tool names in this registry
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .iter()
            .filter(|t| {
                // For main bot, exclude tools that are forbidden by the tool filter
                // No point paying tokens for tool definitions Sandy can't use
                if self.is_main_bot {
                    tool_filter::can_sandy_use(t.name()).is_ok()
                } else {
                    true
                }
            })
            .map(|t| t.definition())
            .collect()
    }

    pub async fn execute(&self, name: &str, input: serde_json::Value) -> ToolResult {
        // GUARDRAIL: Check if Sandy (main bot) is allowed to use this tool
        // Only apply to main bot, not sub-agents (Zilla, Gonza need web access!)
        if self.is_main_bot {
            if let Err(violation) = tool_filter::can_sandy_use(name) {
                tracing::warn!("Tool filter blocked main bot from using: {}", name);
                return ToolResult::error(format!("{}\n\nSuggestion: {}", violation, tool_filter::get_alternative(name)));
            }
        }

        // Run pre-hooks (may block or modify input)
        let input = match self.hooks.run_pre_hooks(name, input).await {
            Ok(input) => input,
            Err(blocked) => return blocked,
        };

        let start = std::time::Instant::now();

        let result = {
            let mut found = false;
            let mut result = ToolResult::error(format!("Unknown tool: {name}"));
            for tool in &self.tools {
                if tool.name() == name {
                    result = tool.execute(input.clone()).await;
                    found = true;
                    break;
                }
            }
            if !found {
                return result;
            }
            result
        };

        let duration = start.elapsed();

        // Run post-hooks
        self.hooks
            .run_post_hooks(name, &input, &result, duration)
            .await;

        result
    }

    pub async fn execute_with_auth(
        &self,
        name: &str,
        input: serde_json::Value,
        auth: &ToolAuthContext,
    ) -> ToolResult {
        let input = inject_auth_context(input, auth);
        self.execute(name, input).await
    }
}

/// Helper to build a JSON Schema object with required properties.
pub fn schema_object(properties: serde_json::Value, required: &[&str]) -> serde_json::Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::tests::test_config;

    #[test]
    fn test_tool_result_success() {
        let r = ToolResult::success("ok".into());
        assert_eq!(r.content, "ok");
        assert!(!r.is_error);
    }

    #[test]
    fn test_tool_result_error() {
        let r = ToolResult::error("fail".into());
        assert_eq!(r.content, "fail");
        assert!(r.is_error);
    }

    #[test]
    fn test_schema_object() {
        let schema = schema_object(
            json!({
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }),
            &["name"],
        );
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
        assert!(schema["properties"]["age"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "name");
    }

    #[test]
    fn test_schema_object_empty_required() {
        let schema = schema_object(json!({}), &[]);
        let required = schema["required"].as_array().unwrap();
        assert!(required.is_empty());
    }

    #[test]
    fn test_auth_context_from_input() {
        let input = json!({
            "__sandy_auth": {
                "caller_chat_id": 123,
                "control_chat_ids": [123, 999]
            }
        });
        let auth = auth_context_from_input(&input).unwrap();
        assert_eq!(auth.caller_chat_id, 123);
        assert!(auth.is_control_chat());
        assert!(auth.can_access_chat(456));
    }

    #[test]
    fn test_authorize_chat_access_denied() {
        let input = json!({
            "__sandy_auth": {
                "caller_chat_id": 100,
                "control_chat_ids": []
            }
        });
        let err = authorize_chat_access(&input, 200).unwrap_err();
        assert!(err.contains("Permission denied"));
    }

    #[test]
    fn test_tool_registry_empty() {
        let registry = ToolRegistry::empty();
        assert_eq!(registry.tool_count(), 0);
        assert!(!registry.has_tool("any_tool"));
        assert_eq!(registry.tool_names().len(), 0);
    }

    #[test]
    fn test_tool_registry_tool_count() {
        let config = test_config();
        let registry = ToolRegistry::new_sub_agent(&config);
        assert_eq!(registry.tool_count(), 17); // Sub-agent registry has 17 tools
    }

    #[test]
    fn test_tool_registry_has_tool() {
        let config = test_config();
        let registry = ToolRegistry::new_sub_agent(&config);
        assert!(registry.has_tool("bash"));
        assert!(registry.has_tool("read_file"));
        assert!(registry.has_tool("write_file"));
        assert!(!registry.has_tool("spawn_agent")); // Not in sub-agent registry
        assert!(!registry.has_tool("nonexistent_tool"));
    }

    #[test]
    fn test_tool_registry_tool_names() {
        let config = test_config();
        let registry = ToolRegistry::new_sub_agent(&config);
        let names = registry.tool_names();
        assert_eq!(names.len(), 17);
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"read_file".to_string()));
        assert!(!names.contains(&"spawn_agent".to_string()));
    }
}
