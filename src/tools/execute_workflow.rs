//! Workflow Execution System - Sequential Agent Workflows
//!
//! This module provides sequential workflow execution where multiple agents
//! run in sequence with verification between each step.
//!
//! Example workflow: Zilla (research) → verify → Gonza (write) → verify
//!
//! If any step fails verification, the workflow stops and reports the error.

use async_trait::async_trait;
use serde_json::json;
use std::path::Path;
use tracing::{error, info};

use super::{schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;
use crate::config::Config;

/// Workflow step definition
#[derive(Clone, Debug)]
pub struct WorkflowStep {
    pub step_number: usize,
    pub agent_id: String,
    pub task: String,
    pub output_path: String,
    pub input_file: Option<String>,
    pub verify_output: bool,
}

pub struct ExecuteWorkflowTool {
    config: Config,
    storage_dir: String,
}

impl ExecuteWorkflowTool {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            storage_dir: config.working_dir.clone(),
        }
    }

    /// Generate unique workflow ID
    fn generate_workflow_id(&self) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let random = rand::random::<u16>();
        format!("workflow_{}_{:04x}", timestamp, random)
    }

    /// Create workflow folder
    async fn create_workflow_folder(&self, workflow_id: &str) -> Result<std::path::PathBuf, String> {
        let workflow_path = Path::new(&self.storage_dir)
            .join("tasks")
            .join(workflow_id);

        tokio::fs::create_dir_all(&workflow_path)
            .await
            .map_err(|e| format!("Failed to create workflow folder: {}", e))?;

        info!("Created workflow folder: {}", workflow_path.display());
        Ok(workflow_path)
    }

    /// Verify file exists, has content, and optionally matches expected format.
    async fn verify_file(&self, path: &str, expected_format: Option<&str>) -> Result<(bool, u64, Option<String>), String> {
        let file_path = Path::new(path);

        if !file_path.exists() {
            return Ok((false, 0, Some(format!("File does not exist: {}", path))));
        }

        match tokio::fs::metadata(file_path).await {
            Ok(metadata) => {
                let size = metadata.len();
                if size == 0 {
                    return Ok((false, 0, Some("File exists but is empty".to_string())));
                }

                // Try to read and validate content
                match tokio::fs::read_to_string(file_path).await {
                    Ok(content) => {
                        if content.starts_with("ERROR:") || content.contains("\"error\"") {
                            return Ok((false, size, Some(format!("File contains error: {}", &content[..content.len().min(200)]))));
                        }

                        // Format validation (step 17)
                        if let Some(format) = expected_format {
                            match format {
                                "structured_json" | "json" => {
                                    if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                                        return Ok((false, size, Some(format!(
                                            "Output file is not valid JSON (expected {})", format
                                        ))));
                                    }
                                }
                                "markdown_article" | "markdown" | "md" => {
                                    let has_content = content.trim().len() > 10;
                                    let has_markdown = content.contains('#')
                                        || content.contains('*')
                                        || content.contains('[')
                                        || content.contains('\n');
                                    if !has_content || !has_markdown {
                                        return Ok((false, size, Some(format!(
                                            "Output does not appear to be valid markdown (expected {})", format
                                        ))));
                                    }
                                }
                                _ => {} // text or unknown: just check content exists
                            }
                        }

                        Ok((true, size, None))
                    }
                    Err(_) => {
                        // File exists but can't be read as text (might be binary)
                        Ok((true, size, None))
                    }
                }
            }
            Err(e) => Err(format!("Failed to read file metadata: {}", e)),
        }
    }

    /// Execute a single workflow step using spawn_agent
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        workflow_id: &str,
        spawn_tool: &super::agent_management::SpawnAgentTool,
    ) -> Result<(bool, String), String> {
        info!(
            "Executing workflow step {}: agent='{}' task='{}'",
            step.step_number, step.agent_id, step.task
        );

        // Build task with input file if specified
        let task_with_context = if let Some(input_file) = &step.input_file {
            format!(
                "{}\n\n*** INPUT FILE ***\nRead the input from: {}\nUse this as your source material.",
                step.task, input_file
            )
        } else {
            step.task.clone()
        };

        let spawn_input = json!({
            "agent_id": step.agent_id,
            "task": task_with_context,
            "output_path": step.output_path,
            "job_id": format!("{}_step{}", workflow_id, step.step_number)
        });

        let result = spawn_tool.execute(spawn_input).await;

        if result.is_error {
            return Ok((false, format!("Step {} failed: {}", step.step_number, result.content)));
        }

        // Verify output if required
        if step.verify_output {
            info!("Verifying output for step {}: {}", step.step_number, step.output_path);

            // Load agent config to get output_format for validation
            let expected_format = spawn_tool.load_agent_config(&step.agent_id).await
                .ok()
                .and_then(|c| if c.output_format.is_empty() { None } else { Some(c.output_format) });

            match self.verify_file(&step.output_path, expected_format.as_deref()).await {
                Ok((exists, size, error)) => {
                    if !exists {
                        let error_msg = error.unwrap_or_else(|| "Unknown verification error".to_string());
                        return Ok((false, format!(
                            "Step {} verification failed: {}\nOutput path: {}",
                            step.step_number, error_msg, step.output_path
                        )));
                    }

                    info!(
                        "Step {} verified: {} ({} bytes)",
                        step.step_number, step.output_path, size
                    );
                    Ok((true, format!("Step {} completed successfully ({} bytes)", step.step_number, size)))
                }
                Err(e) => {
                    Ok((false, format!("Step {} verification error: {}", step.step_number, e)))
                }
            }
        } else {
            Ok((true, format!("Step {} completed (verification skipped)", step.step_number)))
        }
    }

    /// Parse workflow steps from input
    fn parse_steps(&self, steps_value: &serde_json::Value) -> Result<Vec<WorkflowStep>, String> {
        let steps_array = steps_value
            .as_array()
            .ok_or_else(|| "steps must be an array".to_string())?;

        if steps_array.is_empty() {
            return Err("workflow must have at least one step".to_string());
        }

        let mut steps = Vec::new();

        for (i, step_value) in steps_array.iter().enumerate() {
            let step_number = i + 1;

            let agent_id = step_value
                .get("agent_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Step {}: missing agent_id", step_number))?;

            let task = step_value
                .get("task")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Step {}: missing task", step_number))?;

            let output_path = step_value
                .get("output_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Step {}: missing output_path", step_number))?;

            let input_file = step_value
                .get("input_file")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let verify_output = step_value
                .get("verify_output")
                .and_then(|v| v.as_bool())
                .unwrap_or(true); // Default to verifying

            steps.push(WorkflowStep {
                step_number,
                agent_id: agent_id.to_string(),
                task: task.to_string(),
                output_path: output_path.to_string(),
                input_file,
                verify_output,
            });
        }

        Ok(steps)
    }
}

#[async_trait]
impl Tool for ExecuteWorkflowTool {
    fn name(&self) -> &str {
        "execute_workflow"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "execute_workflow".into(),
            description: "Execute a sequential workflow of multiple agents. Each step runs in order, and if verification fails at any step, the workflow stops. Perfect for multi-stage tasks like research → write → review.".into(),
            input_schema: schema_object(
                json!({
                    "name": {
                        "type": "string",
                        "description": "Name of the workflow (e.g., 'Research and Write Article')"
                    },
                    "steps": {
                        "type": "array",
                        "description": "Array of workflow steps to execute in order",
                        "items": {
                            "type": "object",
                            "properties": {
                                "agent_id": {
                                    "type": "string",
                                    "description": "Agent ID from storage/agents/ (e.g., 'zilla', 'gonza')"
                                },
                                "task": {
                                    "type": "string",
                                    "description": "Task description for this step"
                                },
                                "output_path": {
                                    "type": "string",
                                    "description": "Where to save this step's output"
                                },
                                "input_file": {
                                    "type": "string",
                                    "description": "(Optional) Path to input file from previous step",
                                    "default": null
                                },
                                "verify_output": {
                                    "type": "boolean",
                                    "description": "Whether to verify output file exists and is valid. Default: true",
                                    "default": true
                                }
                            },
                            "required": ["agent_id", "task", "output_path"]
                        }
                    }
                }),
                &["name", "steps"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let workflow_name = match input.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return ToolResult::error("Missing required parameter: name".into()),
        };

        let steps_value = match input.get("steps") {
            Some(s) => s,
            None => return ToolResult::error("Missing required parameter: steps".into()),
        };

        // Parse workflow steps
        let steps = match self.parse_steps(steps_value) {
            Ok(s) => s,
            Err(e) => return ToolResult::error(format!("Invalid workflow steps: {}", e)),
        };

        let workflow_id = self.generate_workflow_id();

        info!(
            "Starting workflow '{}' ({}): {} steps",
            workflow_name,
            workflow_id,
            steps.len()
        );

        // Create workflow folder
        if let Err(e) = self.create_workflow_folder(&workflow_id).await {
            return ToolResult::error(format!("Failed to create workflow folder: {}", e));
        }

        // Create spawn tool once for reuse across steps (step 15)
        let spawn_tool = super::agent_management::SpawnAgentTool::new(&self.config);

        // Execute steps sequentially
        let mut completed_steps = Vec::new();
        let mut failed_step = None;

        for step in &steps {
            info!("Workflow {}: Starting step {}/{}", workflow_id, step.step_number, steps.len());

            match self.execute_step(step, &workflow_id, &spawn_tool).await {
                Ok((success, message)) => {
                    if success {
                        completed_steps.push((step.step_number, step.output_path.clone(), message));
                        info!("Workflow {}: Step {} completed", workflow_id, step.step_number);
                    } else {
                        failed_step = Some((step.step_number, message.clone()));
                        error!(
                            "Workflow {}: Step {} failed - {}",
                            workflow_id, step.step_number, message
                        );
                        break;
                    }
                }
                Err(e) => {
                    failed_step = Some((step.step_number, e.clone()));
                    error!(
                        "Workflow {}: Step {} error - {}",
                        workflow_id, step.step_number, e
                    );
                    break;
                }
            }
        }

        // Build result
        if let Some((failed_num, error_msg)) = failed_step {
            let completed_count = completed_steps.len();
            let mut result = format!(
                "❌ Workflow '{}' failed at step {}/{}\n\n",
                workflow_name, failed_num, steps.len()
            );

            result.push_str(&format!("Workflow ID: {}\n\n", workflow_id));

            if completed_count > 0 {
                result.push_str("✅ Completed steps:\n");
                for (num, path, msg) in &completed_steps {
                    result.push_str(&format!("  Step {}: {}\n    {}\n", num, path, msg));
                }
                result.push('\n');
            }

            result.push_str(&format!(
                "❌ Failed at step {}:\n  Error: {}\n\n",
                failed_num, error_msg
            ));

            result.push_str("The workflow stopped due to this error. You can:\n");
            result.push_str("- Check the output files manually\n");
            result.push_str("- Fix any issues and restart the workflow\n");
            result.push_str("- Run steps individually using spawn_agent\n");

            ToolResult::error(result)
        } else {
            let mut result = format!(
                "✅ Workflow '{}' completed successfully!\n\n",
                workflow_name
            );

            result.push_str(&format!("Workflow ID: {}\n", workflow_id));
            result.push_str(&format!("Total steps: {}\n\n", steps.len()));
            result.push_str("Output files:\n");

            for (num, path, msg) in &completed_steps {
                result.push_str(&format!("  Step {}: {}\n    {}\n", num, path, msg));
            }

            ToolResult::success(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::tests::test_config;
    use serde_json::json;

    #[test]
    fn test_execute_workflow_definition() {
        let tool = ExecuteWorkflowTool::new(&test_config());
        assert_eq!(tool.name(), "execute_workflow");
        let def = tool.definition();
        assert_eq!(def.name, "execute_workflow");
    }

    #[test]
    fn test_parse_workflow_steps() {
        let tool = ExecuteWorkflowTool::new(&test_config());

        let steps_json = json!([
            {
                "agent_id": "zilla",
                "task": "Research AI news",
                "output_path": "storage/tasks/test/research.json"
            },
            {
                "agent_id": "gonza",
                "task": "Write article",
                "output_path": "storage/tasks/test/article.md",
                "input_file": "storage/tasks/test/research.json"
            }
        ]);

        let steps = tool.parse_steps(&steps_json).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].agent_id, "zilla");
        assert_eq!(steps[0].verify_output, true); // default
        assert_eq!(steps[1].input_file, Some("storage/tasks/test/research.json".to_string()));
    }

    #[test]
    fn test_parse_empty_steps() {
        let tool = ExecuteWorkflowTool::new(&test_config());
        let steps_json = json!([]);
        let result = tool.parse_steps(&steps_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least one step"));
    }
}
