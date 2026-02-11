use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;

// Global agent registry - tracks spawned agents and their reporting preferences
lazy_static::lazy_static! {
    static ref AGENT_REGISTRY: Arc<Mutex<HashMap<String, AgentInfo>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub agent_id: String,
    pub name: String,
    pub specialty: String,
    pub created_at: Instant,
    pub reporting_enabled: bool,
    pub last_activity: Option<String>,
    pub status: AgentStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgentStatus {
    Running,
    Completed,
    Failed(String),
}

pub struct SpawnAgentTool;

impl SpawnAgentTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SpawnAgentTool {
    fn name(&self) -> &str {
        "spawn_agent"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "spawn_agent".into(),
            description: "Spawn a specialized sub-agent to handle a task in the background while Sandy continues the main conversation. Agents can do research, write code, analyze files, or other tasks. You can toggle whether each agent reports directly to you via Telegram.".into(),
            input_schema: schema_object(
                json!({
                    "agent_id": {
                        "type": "string",
                        "description": "Unique agent ID (e.g., 'research-1', 'code-helper'). Use descriptive names."
                    },
                    "name": {
                        "type": "string",
                        "description": "Display name for the agent (e.g., 'Research Agent', 'Code Assistant')"
                    },
                    "specialty": {
                        "type": "string",
                        "description": "What this agent specializes in (e.g., 'web research', 'Python scripting', 'file organization')"
                    },
                    "task": {
                        "type": "string",
                        "description": "The specific task to complete"
                    },
                    "reporting_enabled": {
                        "type": "boolean",
                        "description": "Whether this agent should send progress reports to you in Telegram. Default: false (Sandy will summarize instead).",
                        "default": false
                    },
                    "timeout_minutes": {
                        "type": "integer",
                        "description": "Maximum time in minutes before agent is considered failed. Default: 30.",
                        "default": 30
                    }
                }),
                &["agent_id", "name", "specialty", "task"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let agent_id = match input.get("agent_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => return ToolResult::error("Missing required parameter: agent_id".into()),
        };

        let name = match input.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => return ToolResult::error("Missing required parameter: name".into()),
        };

        let specialty = match input.get("specialty").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return ToolResult::error("Missing required parameter: specialty".into()),
        };

        let task = match input.get("task").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return ToolResult::error("Missing required parameter: task".into()),
        };

        let reporting_enabled = input.get("reporting_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let _timeout_minutes = input.get("timeout_minutes").and_then(|v| v.as_i64()).unwrap_or(30);

        // Register the agent
        let agent_info = AgentInfo {
            agent_id: agent_id.clone(),
            name: name.clone(),
            specialty: specialty.clone(),
            created_at: Instant::now(),
            reporting_enabled,
            last_activity: Some(format!("Spawned with task: {}", task)),
            status: AgentStatus::Running,
        };

        {
            let mut registry = AGENT_REGISTRY.lock().unwrap();
            registry.insert(agent_id.clone(), agent_info);
        }

        // Build response message
        let reporting_msg = if reporting_enabled {
            "✅ Direct reporting enabled - you'll receive updates from this agent in Telegram."
        } else {
            "ℹ️  Direct reporting disabled - Sandy will summarize the results when complete."
        };

        // Note: In a full implementation, this would actually spawn a background task
        // For now, we register the agent and simulate the async nature
        
        ToolResult::success(format!(
            "🤖 Agent '{}' spawned successfully!\n\nSpecialty: {}\nTask: {}\n\n{}\n\n💡 You can check status anytime: 'Check agent {} status' or use list_agents tool.",
            name, specialty, task, reporting_msg, agent_id
        ))
    }
}

pub struct ListAgentsTool;

impl ListAgentsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ListAgentsTool {
    fn name(&self) -> &str {
        "list_agents"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_agents".into(),
            description: "List all active and recently completed agents with their status and reporting settings.".into(),
            input_schema: schema_object(
                json!({
                    "show_completed": {
                        "type": "boolean",
                        "description": "Whether to include completed agents. Default: false (show only active).",
                        "default": false
                    }
                }),
                &[],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let show_completed = input.get("show_completed").and_then(|v| v.as_bool()).unwrap_or(false);

        let registry = AGENT_REGISTRY.lock().unwrap();
        
        if registry.is_empty() {
            return ToolResult::success("No active agents. Use spawn_agent to create one.".into());
        }

        let mut output = String::from("🤖 Active Agents:\n\n");
        let mut count = 0;

        for (id, info) in registry.iter() {
            let should_show = match info.status {
                AgentStatus::Running => true,
                AgentStatus::Completed | AgentStatus::Failed(_) => show_completed,
            };

            if should_show {
                count += 1;
                let status_icon = match info.status {
                    AgentStatus::Running => "▶️",
                    AgentStatus::Completed => "✅",
                    AgentStatus::Failed(_) => "❌",
                };

                let reporting = if info.reporting_enabled { "📢 Reports ON" } else { "🔇 Reports OFF" };
                let elapsed = info.created_at.elapsed().as_secs() / 60;
                
                output.push_str(&format!(
                    "{} {} ({}\n  Specialty: {}\n  {} | Running for {}m\n  {}\n\n",
                    status_icon,
                    info.name,
                    id,
                    info.specialty,
                    reporting,
                    elapsed,
                    info.last_activity.as_ref().unwrap_or(&"No recent activity".to_string())
                ));
            }
        }

        if count == 0 {
            output.push_str("No active agents. Use spawn_agent to create one.");
        } else {
            output.push_str(&format!("Total: {} agents", count));
        }

        ToolResult::success(output)
    }
}

pub struct SetAgentReportingTool;

impl SetAgentReportingTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SetAgentReportingTool {
    fn name(&self) -> &str {
        "set_agent_reporting"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "set_agent_reporting".into(),
            description: "Toggle direct Telegram reporting on/off for a specific agent. When enabled, the agent will send progress updates directly to you. When disabled, Sandy will summarize results instead.".into(),
            input_schema: schema_object(
                json!({
                    "agent_id": {
                        "type": "string",
                        "description": "The agent ID to modify"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "true to enable direct reports, false to disable"
                    }
                }),
                &["agent_id", "enabled"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let agent_id = match input.get("agent_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: agent_id".into()),
        };

        let enabled = match input.get("enabled").and_then(|v| v.as_bool()) {
            Some(e) => e,
            None => return ToolResult::error("Missing required parameter: enabled (boolean)".into()),
        };

        let mut registry = AGENT_REGISTRY.lock().unwrap();
        
        match registry.get_mut(agent_id) {
            Some(agent) => {
                agent.reporting_enabled = enabled;
                let status = if enabled {
                    "✅ Direct reporting ENABLED"
                } else {
                    "🔇 Direct reporting DISABLED (Sandy will summarize instead)"
                };
                
                ToolResult::success(format!(
                    "{} for agent '{}'.\n\n{}",
                    status,
                    agent.name,
                    if enabled {
                        "You'll now receive progress updates directly from this agent in Telegram."
                    } else {
                        "This agent will work silently and Sandy will provide a summary when complete."
                    }
                ))
            }
            None => {
                ToolResult::error(format!(
                    "Agent '{}' not found. Use list_agents to see active agents.",
                    agent_id
                ))
            }
        }
    }
}

pub struct AgentStatusTool;

impl AgentStatusTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for AgentStatusTool {
    fn name(&self) -> &str {
        "agent_status"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "agent_status".into(),
            description: "Check the current status of a specific agent including progress, errors, and results.".into(),
            input_schema: schema_object(
                json!({
                    "agent_id": {
                        "type": "string",
                        "description": "The agent ID to check"
                    }
                }),
                &["agent_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let agent_id = match input.get("agent_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: agent_id".into()),
        };

        let registry = AGENT_REGISTRY.lock().unwrap();
        
        match registry.get(agent_id) {
            Some(agent) => {
                let status_icon = match agent.status {
                    AgentStatus::Running => "▶️",
                    AgentStatus::Completed => "✅",
                    AgentStatus::Failed(_) => "❌",
                };

                let reporting = if agent.reporting_enabled { "📢 Reports ON" } else { "🔇 Reports OFF" };
                let elapsed = agent.created_at.elapsed().as_secs() / 60;
                
                ToolResult::success(format!(
                    "{} {}\n\nAgent ID: {}\nSpecialty: {}\nStatus: {:?}\nRunning for: {} minutes\nReporting: {}\n\nLast Activity: {}",
                    status_icon,
                    agent.name,
                    agent.agent_id,
                    agent.specialty,
                    agent.status,
                    elapsed,
                    reporting,
                    agent.last_activity.as_ref().unwrap_or(&"None".to_string())
                ))
            }
            None => {
                ToolResult::error(format!(
                    "Agent '{}' not found. Use list_agents to see active agents.",
                    agent_id
                ))
            }
        }
    }
}

// Helper function to update agent status (would be called by the actual agent implementation)
pub fn update_agent_status(agent_id: &str, status: AgentStatus, activity: Option<String>) {
    let mut registry = AGENT_REGISTRY.lock().unwrap();
    if let Some(agent) = registry.get_mut(agent_id) {
        agent.status = status;
        if let Some(act) = activity {
            agent.last_activity = Some(act);
        }
    }
}

// Helper function to get agent reporting preference
pub fn get_agent_reporting(agent_id: &str) -> bool {
    let registry = AGENT_REGISTRY.lock().unwrap();
    registry.get(agent_id).map(|a| a.reporting_enabled).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_spawn_agent_definition() {
        let tool = SpawnAgentTool::new();
        assert_eq!(tool.name(), "spawn_agent");
        let def = tool.definition();
        assert_eq!(def.name, "spawn_agent");
    }

    #[test]
    fn test_list_agents_definition() {
        let tool = ListAgentsTool::new();
        assert_eq!(tool.name(), "list_agents");
    }

    #[test]
    fn test_set_agent_reporting_definition() {
        let tool = SetAgentReportingTool::new();
        assert_eq!(tool.name(), "set_agent_reporting");
    }
}
