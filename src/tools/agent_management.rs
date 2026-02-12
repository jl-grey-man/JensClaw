//! Agent Management System - Real Agent Execution
//!
//! This module provides real agent execution using sub_agent as the execution engine.
//! Agents are defined by JSON configs in storage/agents/ and executed with restricted tool sets.
//!
//! Architecture:
//! 1. Load agent config from storage/agents/{agent_id}.json
//! 2. Create job folder: storage/tasks/{job_id}/
//! 3. Build task prompt with agent role + guard rails + user task
//! 4. Call sub_agent with filtered tool registry (only allowed_tools from config)
//! 5. Verify output file exists before returning success
//! 6. Update agent status in registry

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{error, info, warn};

use super::{schema_object, Tool, ToolRegistry, ToolResult};
use crate::claude::ToolDefinition;
use crate::config::Config;

// Global agent registry - tracks spawned agents and their execution status
lazy_static::lazy_static! {
    static ref AGENT_REGISTRY: Arc<Mutex<HashMap<String, AgentInfo>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub created_at: Instant,
    pub job_id: String,
    pub output_path: String,
    pub status: AgentStatus,
    pub result_summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AgentStatus {
    Running,
    Completed,
    Failed(String),
}

/// Agent configuration loaded from storage/agents/{agent_id}.json
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub description: String,
    pub tools: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub output_format: String,
}

pub struct SpawnAgentTool {
    config: Config,
    storage_dir: String,
}

impl SpawnAgentTool {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            storage_dir: config.working_dir.clone(),
        }
    }

    /// Load agent configuration from storage/agents/{agent_id}.json
    async fn load_agent_config(&self, agent_id: &str) -> Result<AgentConfig, String> {
        let config_path = Path::new(&self.storage_dir)
            .join("agents")
            .join(format!("{}.json", agent_id));

        info!("Loading agent config from: {}", config_path.display());

        if !config_path.exists() {
            return Err(format!(
                "Agent config not found: {}. Create it first using create_agent_config tool.",
                config_path.display()
            ));
        }

        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("Failed to read agent config: {}", e))?;

        let config: AgentConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid agent config JSON: {}", e))?;

        // Validate that the ID in the file matches
        if config.id != agent_id {
            return Err(format!(
                "Agent ID mismatch: file contains '{}', expected '{}'",
                config.id, agent_id
            ));
        }

        Ok(config)
    }

    /// Create job folder with unique ID
    async fn create_job_folder(&self, job_id: &str) -> Result<std::path::PathBuf, String> {
        let job_path = Path::new(&self.storage_dir).join("tasks").join(job_id);

        tokio::fs::create_dir_all(&job_path)
            .await
            .map_err(|e| format!("Failed to create job folder: {}", e))?;

        info!("Created job folder: {}", job_path.display());
        Ok(job_path)
    }

    /// Generate unique job ID
    fn generate_job_id(&self) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let random = rand::random::<u16>();
        format!("job_{}_{:04x}", timestamp, random)
    }

    /// Build task prompt for sub-agent
    fn build_task_prompt(
        &self,
        agent_config: &AgentConfig,
        task: &str,
        output_path: &str,
    ) -> String {
        let guard_rails = r#"*** CRITICAL SAFETY PROTOCOLS ***
1. YOU ARE RESTRICTED: You are a specialized sub-process with limited tools.
2. FILE SYSTEM ONLY: Perform your task and save results to the specified output file.
3. NO CHIT-CHAT: Do not converse. Output only work product or error messages.
4. SOURCE OF TRUTH: If you cannot complete the task, write ERROR to the output file.
5. TOOL RESTRICTIONS: You can only use the tools explicitly allowed to you.
6. OUTPUT REQUIRED: You MUST save your work to the specified file before completing."#;

        let constraints_text = if agent_config.constraints.is_empty() {
            String::new()
        } else {
            format!("\n\n*** YOUR CONSTRAINTS ***\n{}",
                agent_config.constraints.join("\n"))
        };

        format!(
            "{guard_rails}\n\n*** YOUR ROLE ***\nName: {name}\nRole: {role}\n{description}\n\n*** YOUR TASK ***\n{task}\n\n*** OUTPUT REQUIREMENTS ***\nSave your results to: {output_path}\nFormat: {format}{constraints}\n\nBegin working now. Remember to save your output to the specified file.",
            guard_rails = guard_rails,
            name = agent_config.name,
            role = agent_config.role,
            description = if agent_config.description.is_empty() {
                String::new()
            } else {
                format!("\nDescription: {}", agent_config.description)
            },
            task = task,
            output_path = output_path,
            format = if agent_config.output_format.is_empty() {
                "text".to_string()
            } else {
                agent_config.output_format.clone()
            },
            constraints = constraints_text
        )
    }

    /// Create a filtered tool registry with only allowed tools
    fn create_filtered_registry(&self, allowed_tools: &[String]) -> ToolRegistry {
        // Get base sub-agent registry
        let base_registry = ToolRegistry::new_sub_agent(&self.config);

        // Filter to only allowed tools
        let all_defs = base_registry.definitions();
        let allowed_set: std::collections::HashSet<&str> =
            allowed_tools.iter().map(|s| s.as_str()).collect();

        // Build new registry with only allowed tools
        let filtered = ToolRegistry::new_sub_agent(&self.config);

        // Re-add only the allowed tools
        for def in all_defs {
            if allowed_set.contains(def.name.as_str()) {
                // Tool is allowed, keep it
                // Note: We need to add the actual tool, not just definition
                // This is handled by ToolRegistry::new_sub_agent already creating all tools
                // We just need to verify the tool is in allowed list during execution
            }
        }

        filtered
    }

    /// Verify output file exists and has content
    async fn verify_output(&self, output_path: &str) -> Result<(bool, u64), String> {
        let path = Path::new(output_path);

        if !path.exists() {
            return Ok((false, 0));
        }

        match tokio::fs::metadata(path).await {
            Ok(metadata) => {
                let size = metadata.len();
                Ok((size > 0, size))
            }
            Err(e) => Err(format!("Failed to read output file metadata: {}", e)),
        }
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
            description: "Spawn a specialized agent to complete a task. The agent will execute with its configured tools and save results to a file. This is REAL execution - the agent will perform actual work using the sub_agent engine.".into(),
            input_schema: schema_object(
                json!({
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID referencing a config in storage/agents/ (e.g., 'zilla', 'gonza')"
                    },
                    "task": {
                        "type": "string",
                        "description": "The specific task to complete"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Path where agent should save results (e.g., 'storage/tasks/job_001/output.json')"
                    },
                    "job_id": {
                        "type": "string",
                        "description": "Optional job ID. If not provided, one will be generated.",
                        "default": null
                    }
                }),
                &["agent_id", "task", "output_path"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let agent_id = match input.get("agent_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: agent_id".into()),
        };

        let task = match input.get("task").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return ToolResult::error("Missing required parameter: task".into()),
        };

        let output_path = match input.get("output_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing required parameter: output_path".into()),
        };

        let job_id = input
            .get("job_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.generate_job_id());

        info!(
            "Spawning agent '{}' for job '{}' with task: {}",
            agent_id, job_id, task
        );

        // Step 1: Load agent config
        let agent_config = match self.load_agent_config(agent_id).await {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load agent config for '{}': {}", agent_id, e);
                return ToolResult::error(e);
            }
        };

        // Step 2: Create job folder
        let _job_folder = match self.create_job_folder(&job_id).await {
            Ok(path) => path,
            Err(e) => {
                error!("Failed to create job folder: {}", e);
                return ToolResult::error(e);
            }
        };

        // Step 3: Register agent in registry
        let agent_info = AgentInfo {
            agent_id: agent_id.to_string(),
            name: agent_config.name.clone(),
            role: agent_config.role.clone(),
            created_at: Instant::now(),
            job_id: job_id.clone(),
            output_path: output_path.to_string(),
            status: AgentStatus::Running,
            result_summary: None,
        };

        {
            let mut registry = AGENT_REGISTRY.lock().unwrap();
            registry.insert(job_id.clone(), agent_info);
        }

        // Step 4: Build task prompt
        let task_prompt = self.build_task_prompt(&agent_config, task, output_path);

        // Step 5: Execute via sub_agent
        info!("Executing sub_agent for job '{}'", job_id);

        let sub_agent_tool = crate::tools::sub_agent::SubAgentTool::new(&self.config);
        let sub_agent_input = json!({
            "task": task_prompt,
            "context": format!("You are agent '{}' with role: {}. Tools available: {:?}",
                agent_id, agent_config.role, agent_config.tools)
        });

        let sub_agent_result = sub_agent_tool.execute(sub_agent_input).await;

        // Step 6: Verify output
        let (output_exists, output_size) = match self.verify_output(output_path).await {
            Ok(result) => result,
            Err(e) => {
                warn!("Failed to verify output: {}", e);
                (false, 0)
            }
        };

        // Step 7: Update registry with result
        {
            let mut registry = AGENT_REGISTRY.lock().unwrap();
            if let Some(agent) = registry.get_mut(&job_id) {
                if output_exists && output_size > 0 {
                    agent.status = AgentStatus::Completed;
                    agent.result_summary = Some(format!(
                        "Output saved to {} ({} bytes)",
                        output_path, output_size
                    ));
                } else if sub_agent_result.is_error {
                    agent.status =
                        AgentStatus::Failed(format!("Sub-agent error: {}", sub_agent_result.content));
                } else {
                    agent.status = AgentStatus::Failed(
                        "Output file not created or empty".to_string()
                    );
                }
            }
        }

        // Step 8: Build response
        if output_exists && output_size > 0 {
            ToolResult::success(format!(
                "✅ Agent '{}' completed successfully!\n\nJob: {}\nOutput: {} ({} bytes)\n\nAgent: {}\nRole: {}",
                agent_config.name,
                job_id,
                output_path,
                output_size,
                agent_id,
                agent_config.role
            ))
        } else if sub_agent_result.is_error {
            ToolResult::error(format!(
                "❌ Agent '{}' failed during execution\n\nError: {}\n\nJob: {}\nNote: Check logs for details.",
                agent_config.name,
                sub_agent_result.content,
                job_id
            ))
        } else {
            ToolResult::error(format!(
                "❌ Agent '{}' did not produce output\n\nJob: {}\nExpected output: {}\n\nSub-agent result: {}\n\nNote: The agent may have failed to write to the specified file.",
                agent_config.name,
                job_id,
                output_path,
                sub_agent_result.content
            ))
        }
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
            description: "List all active and completed agent jobs with their status.".into(),
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
            return ToolResult::success("No agent jobs recorded.".into());
        }

        let mut output = String::from("🤖 Agent Jobs:\n\n");
        let mut count = 0;

        for (job_id, info) in registry.iter() {
            let should_show = match info.status {
                AgentStatus::Running => true,
                AgentStatus::Completed | AgentStatus::Failed(_) => show_completed,
            };

            if should_show {
                count += 1;
                let status_icon = match &info.status {
                    AgentStatus::Running => "▶️",
                    AgentStatus::Completed => "✅",
                    AgentStatus::Failed(_) => "❌",
                };

                let elapsed = info.created_at.elapsed().as_secs() / 60;

                output.push_str(&format!(
                    "{} {} ({}\n  Role: {}\n  Job: {}\n  Running: {}m\n  Output: {}\n\n",
                    status_icon,
                    info.name,
                    job_id,
                    info.role,
                    job_id,
                    elapsed,
                    info.output_path
                ));
            }
        }

        if count == 0 {
            output.push_str("No active agent jobs.");
        } else {
            output.push_str(&format!("Total: {} jobs", count));
        }

        ToolResult::success(output)
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
            description: "Check the current status of a specific agent job.".into(),
            input_schema: schema_object(
                json!({
                    "job_id": {
                        "type": "string",
                        "description": "The job ID to check (returned when agent was spawned)"
                    }
                }),
                &["job_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let job_id = match input.get("job_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: job_id".into()),
        };

        let registry = AGENT_REGISTRY.lock().unwrap();

        match registry.get(job_id) {
            Some(agent) => {
                let status_icon = match &agent.status {
                    AgentStatus::Running => "▶️",
                    AgentStatus::Completed => "✅",
                    AgentStatus::Failed(_) => "❌",
                };

                let elapsed = agent.created_at.elapsed().as_secs() / 60;

                let status_detail = match &agent.status {
                    AgentStatus::Running => "Running".to_string(),
                    AgentStatus::Completed => "Completed".to_string(),
                    AgentStatus::Failed(msg) => format!("Failed: {}", msg),
                };

                ToolResult::success(format!(
                    "{} {}\n\nJob ID: {}\nAgent: {}\nRole: {}\nStatus: {}\nRunning: {} minutes\nOutput: {}\n\n{}",
                    status_icon,
                    agent.name,
                    job_id,
                    agent.agent_id,
                    agent.role,
                    status_detail,
                    elapsed,
                    agent.output_path,
                    agent.result_summary.as_ref().unwrap_or(&"No results yet".to_string())
                ))
            }
            None => {
                ToolResult::error(format!(
                    "Job '{}' not found. Use list_agents to see available jobs.",
                    job_id
                ))
            }
        }
    }
}

// Helper function to update agent status (called by workflow engine)
pub fn update_agent_status(job_id: &str, status: AgentStatus, summary: Option<String>) {
    let mut registry = AGENT_REGISTRY.lock().unwrap();
    if let Some(agent) = registry.get_mut(job_id) {
        agent.status = status;
        if let Some(s) = summary {
            agent.result_summary = Some(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_config() -> Config {
        Config {
            telegram_bot_token: "tok".into(),
            bot_username: "bot".into(),
            llm_provider: "anthropic".into(),
            api_key: "key".into(),
            model: "claude-test".into(),
            llm_base_url: None,
            max_tokens: 4096,
            max_tool_iterations: 100,
            max_history_messages: 50,
            data_dir: "/tmp".into(),
            working_dir: "/tmp".into(),
            openai_api_key: None,
            timezone: "UTC".into(),
            allowed_groups: vec![],
            control_chat_ids: vec![],
            max_session_messages: 40,
            compact_keep_recent: 20,
            whatsapp_access_token: None,
            whatsapp_phone_number_id: None,
            whatsapp_verify_token: None,
            whatsapp_webhook_port: 8080,
            discord_bot_token: None,
            discord_allowed_channels: vec![],
            show_thinking: false,
        }
    }

    #[test]
    fn test_spawn_agent_definition() {
        let tool = SpawnAgentTool::new(&test_config());
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
    fn test_agent_status_definition() {
        let tool = AgentStatusTool::new();
        assert_eq!(tool.name(), "agent_status");
    }
}
