use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;
use std::path::PathBuf;

use super::{schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;
use crate::config::Config;

pub struct DoctorTool {
    config: Config,
    registered_tool_names: Vec<String>,
}

impl DoctorTool {
    /// Create DoctorTool with the actual registered tool names from the registry.
    pub fn new(config: &Config, registered_tool_names: Vec<String>) -> Self {
        Self {
            config: config.clone(),
            registered_tool_names,
        }
    }

    /// Extract tool names mentioned in AGENTS.md (finds all backtick-enclosed names per line)
    async fn extract_tools_from_agents_md(&self, registered_tools: &HashSet<String>) -> Result<HashSet<String>, String> {
        let agents_path = PathBuf::from(&self.config.agents_file);
        let content = tokio::fs::read_to_string(&agents_path)
            .await
            .map_err(|e| format!("Failed to read AGENTS.md: {}", e))?;

        let mut mentioned_tools = HashSet::new();
        let common_params = [
            "output_path", "input_file", "file_path", "job_id",
            "agent_id", "chat_id", "show_completed", "verify_output",
            "task_id", "content", "path", "query", "name",
        ];

        for line in content.lines() {
            let mut rest = line;
            while let Some(start) = rest.find('`') {
                let after_backtick = &rest[start + 1..];
                if let Some(end) = after_backtick.find('`') {
                    let tool_ref = &after_backtick[..end];
                    let tool_name = tool_ref.split('(').next().unwrap_or(tool_ref).trim();

                    if (tool_name.contains('_') || registered_tools.contains(tool_name))
                        && !common_params.contains(&tool_name)
                        && !tool_name.is_empty()
                    {
                        mentioned_tools.insert(tool_name.to_string());
                    }
                    rest = &after_backtick[end + 1..];
                } else {
                    break;
                }
            }
        }

        Ok(mentioned_tools)
    }

    /// Check agent config files for invalid tool references
    async fn validate_agent_configs(&self, valid_tools: &HashSet<String>) -> Result<Vec<String>, String> {
        let agents_dir = PathBuf::from(&self.config.working_dir).join("agents");
        if !agents_dir.exists() {
            return Ok(Vec::new());
        }

        let mut issues = Vec::new();
        let mut entries = tokio::fs::read_dir(&agents_dir)
            .await
            .map_err(|e| format!("Failed to read agents directory: {}", e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

                let config: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e))?;

                if let Some(tools_array) = config.get("tools").and_then(|v| v.as_array()) {
                    for tool in tools_array {
                        if let Some(tool_name) = tool.as_str() {
                            if !valid_tools.contains(tool_name) {
                                issues.push(format!(
                                    "❌ {} references invalid tool: '{}'",
                                    path.file_name().unwrap().to_string_lossy(),
                                    tool_name
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Attempt auto-fixes for common issues
    async fn auto_fix(&self) -> Vec<String> {
        let mut fixes = Vec::new();

        // Fix 1: Create missing memory directory
        let memory_dir = PathBuf::from(&self.config.data_dir).join("memory");
        if !memory_dir.exists() {
            match tokio::fs::create_dir_all(&memory_dir).await {
                Ok(()) => fixes.push(format!("✅ Created missing memory directory: {}", memory_dir.display())),
                Err(e) => fixes.push(format!("❌ Failed to create memory directory: {}", e)),
            }
        }

        // Fix 2: Create missing data subdirectories
        let subdirs = ["groups", "skills", "conversations"];
        for subdir in &subdirs {
            let dir = PathBuf::from(&self.config.data_dir).join(subdir);
            if !dir.exists() {
                match tokio::fs::create_dir_all(&dir).await {
                    Ok(()) => fixes.push(format!("✅ Created missing directory: {}", dir.display())),
                    Err(e) => fixes.push(format!("❌ Failed to create directory {}: {}", dir.display(), e)),
                }
            }
        }

        // Fix 3: Create missing working directory
        let working_dir = PathBuf::from(&self.config.working_dir);
        if !working_dir.exists() {
            match tokio::fs::create_dir_all(&working_dir).await {
                Ok(()) => fixes.push(format!("✅ Created missing working directory: {}", working_dir.display())),
                Err(e) => fixes.push(format!("❌ Failed to create working directory: {}", e)),
            }
        }

        // Fix 4: Reset corrupt memory files (if they contain invalid content)
        let memory_files = ["solutions.md", "errors.md", "patterns.md", "insights.md"];
        for file in &memory_files {
            let path = memory_dir.join(file);
            if path.exists() {
                match tokio::fs::read_to_string(&path).await {
                    Ok(content) => {
                        // Check for null bytes or other corruption
                        if content.contains('\0') {
                            let cleaned = content.replace('\0', "");
                            match tokio::fs::write(&path, cleaned).await {
                                Ok(()) => fixes.push(format!("✅ Cleaned null bytes from {}", file)),
                                Err(e) => fixes.push(format!("❌ Failed to clean {}: {}", file, e)),
                            }
                        }
                    }
                    Err(e) => {
                        // File exists but can't be read - recreate empty
                        match tokio::fs::write(&path, "").await {
                            Ok(()) => fixes.push(format!("✅ Reset unreadable file: {} (was: {})", file, e)),
                            Err(e2) => fixes.push(format!("❌ Failed to reset {}: {}", file, e2)),
                        }
                    }
                }
            }
        }

        // Fix 5: Create missing exec_log directory
        let log_dir = PathBuf::from(&self.config.data_dir);
        if !log_dir.exists() {
            match tokio::fs::create_dir_all(&log_dir).await {
                Ok(()) => fixes.push(format!("✅ Created data directory: {}", log_dir.display())),
                Err(e) => fixes.push(format!("❌ Failed to create data directory: {}", e)),
            }
        }

        if fixes.is_empty() {
            fixes.push("✅ No issues found - nothing to fix".into());
        }

        fixes
    }

    /// Main diagnostic function
    async fn diagnose(&self, do_auto_fix: bool) -> Result<String, String> {
        let mut report = String::new();
        let mut has_errors = false;
        let mut has_warnings = false;

        report.push_str("🩺 **Sandy System Diagnostic Report**\n\n");

        // 1. Registered tools (from actual registry)
        let registered_tools: HashSet<String> = self.registered_tool_names.iter().cloned().collect();
        report.push_str(&format!("**Registered Tools:** {} found\n", registered_tools.len()));

        // 2. Check AGENTS.md
        report.push_str("\n### AGENTS.md Tool References\n");
        match self.extract_tools_from_agents_md(&registered_tools).await {
            Ok(mentioned_tools) => {
                let invalid: Vec<_> = mentioned_tools
                    .iter()
                    .filter(|t| !registered_tools.contains(*t))
                    .collect();

                if invalid.is_empty() {
                    report.push_str("✅ All tool references are valid\n");
                } else {
                    has_warnings = true;
                    report.push_str(&format!("⚠️  Found {} invalid tool references:\n", invalid.len()));
                    for tool in invalid {
                        report.push_str(&format!("  - `{}`\n", tool));
                    }
                }
            }
            Err(e) => {
                has_errors = true;
                report.push_str(&format!("❌ Error reading AGENTS.md: {}\n", e));
            }
        }

        // 3. Check agent configs
        report.push_str("\n### Agent Configurations\n");
        match self.validate_agent_configs(&registered_tools).await {
            Ok(issues) => {
                if issues.is_empty() {
                    report.push_str("✅ All agent configs reference valid tools\n");
                } else {
                    has_warnings = true;
                    report.push_str(&format!("⚠️  Found {} issues:\n", issues.len()));
                    for issue in issues {
                        report.push_str(&format!("  {}\n", issue));
                    }
                }
            }
            Err(e) => {
                has_errors = true;
                report.push_str(&format!("❌ Error validating agent configs: {}\n", e));
            }
        }

        // 4. Check database file exists
        report.push_str("\n### Database\n");
        let db_locations = vec![
            PathBuf::from(&self.config.data_dir).join("sandy.db"),
            PathBuf::from(&self.config.data_dir).join("microclaw.db"),
            PathBuf::from(&self.config.data_dir).join("runtime/microclaw.db"),
        ];

        let mut db_found = false;
        for db_path in &db_locations {
            if db_path.exists() {
                report.push_str(&format!("✅ Database found at {}\n", db_path.display()));
                db_found = true;
                if let Ok(metadata) = tokio::fs::metadata(db_path).await {
                    let size_kb = metadata.len() / 1024;
                    report.push_str(&format!("   Size: {} KB\n", size_kb));
                }
                break;
            }
        }
        if !db_found {
            has_warnings = true;
            report.push_str("⚠️  Database not found in common locations\n");
        }

        // 5. Check memory directory
        report.push_str("\n### Memory System\n");
        let memory_dir = PathBuf::from(&self.config.data_dir).join("memory");
        if memory_dir.exists() {
            let mut file_count = 0;
            if let Ok(mut entries) = tokio::fs::read_dir(&memory_dir).await {
                while entries.next_entry().await.ok().flatten().is_some() {
                    file_count += 1;
                }
            }
            report.push_str(&format!("✅ Memory directory exists ({} files)\n", file_count));
        } else {
            has_warnings = true;
            report.push_str("⚠️  Memory directory not found (will be created on first use)\n");
        }

        // 6. Check execution log
        report.push_str("\n### Execution Log\n");
        let log_path = PathBuf::from(&self.config.data_dir).join("exec_log.jsonl");
        if log_path.exists() {
            if let Ok(metadata) = tokio::fs::metadata(&log_path).await {
                let size_kb = metadata.len() / 1024;
                report.push_str(&format!("✅ Execution log exists ({} KB)\n", size_kb));
            }
        } else {
            report.push_str("ℹ️  No execution log yet (will be created on first tool call)\n");
        }

        // 7. Auto-fix if requested
        if do_auto_fix {
            report.push_str("\n### Auto-Fix Results\n");
            let fixes = self.auto_fix().await;
            for fix in &fixes {
                report.push_str(&format!("{}\n", fix));
            }
        }

        // 8. Final summary
        report.push_str("\n### Summary\n");
        if has_errors {
            report.push_str("⚠️  **Issues found** - Review errors above\n");
        } else if has_warnings {
            report.push_str("⚠️  **Warnings found** - System functional but has minor issues\n");
        } else {
            report.push_str("✅ **System Healthy** - No issues detected\n");
        }

        Ok(report)
    }
}

#[async_trait]
impl Tool for DoctorTool {
    fn name(&self) -> &str {
        "doctor"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "doctor".into(),
            description: "Run system diagnostics to check for configuration issues, invalid tool references, and database integrity. Use auto_fix=true to attempt automatic repairs of common issues (missing directories, corrupt files).".into(),
            input_schema: schema_object(
                json!({
                    "auto_fix": {
                        "type": "boolean",
                        "description": "If true, attempt to automatically fix issues (create missing dirs, reset corrupt files). Default: false."
                    }
                }),
                &[],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let auto_fix = input.get("auto_fix").and_then(|v| v.as_bool()).unwrap_or(false);
        match self.diagnose(auto_fix).await {
            Ok(report) => ToolResult::success(report),
            Err(e) => ToolResult::error(format!("Doctor command failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            telegram_bot_token: "test".into(),
            bot_username: "test".into(),
            llm_provider: "anthropic".into(),
            api_key: "test".into(),
            model: "test".into(),
            fallback_models: vec![],
            llm_base_url: None,
            max_tokens: 4096,
            max_tool_iterations: 100,
            max_history_messages: 50,
            data_dir: "/tmp/sandy_doctor_test".into(),
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
            tavily_api_key: None,
            web_port: 3000,
            soul_file: "soul/SOUL.md".into(),
            identity_file: "soul/IDENTITY.md".into(),
            agents_file: "soul/AGENTS.md".into(),
            memory_file: "soul/data/MEMORY.md".into(),
        }
    }

    #[test]
    fn test_doctor_tool_definition() {
        let tool = DoctorTool::new(&test_config(), vec!["bash".into(), "read_file".into()]);
        assert_eq!(tool.name(), "doctor");
        let def = tool.definition();
        assert_eq!(def.name, "doctor");
        assert!(def.description.contains("auto_fix"));
    }

    #[test]
    fn test_doctor_receives_tool_names() {
        let names = vec!["bash".into(), "read_file".into(), "web_search".into()];
        let tool = DoctorTool::new(&test_config(), names.clone());
        assert_eq!(tool.registered_tool_names, names);
    }

    #[tokio::test]
    async fn test_doctor_auto_fix_creates_dirs() {
        let test_dir = format!("/tmp/sandy_doctor_autofix_{}", uuid::Uuid::new_v4());
        let mut config = test_config();
        config.data_dir = test_dir.clone();

        let tool = DoctorTool::new(&config, vec![]);
        let fixes = tool.auto_fix().await;

        // Should have created memory directory
        let memory_dir = PathBuf::from(&test_dir).join("memory");
        assert!(memory_dir.exists(), "Memory directory should have been created");

        // Check that fixes mention creation
        let fix_text = fixes.join("\n");
        assert!(fix_text.contains("Created"), "Should report created directories");

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_doctor_auto_fix_no_issues() {
        let test_dir = format!("/tmp/sandy_doctor_nofix_{}", uuid::Uuid::new_v4());
        let mut config = test_config();
        config.data_dir = test_dir.clone();

        // Pre-create all expected directories
        let _ = std::fs::create_dir_all(PathBuf::from(&test_dir).join("memory"));
        let _ = std::fs::create_dir_all(PathBuf::from(&test_dir).join("groups"));
        let _ = std::fs::create_dir_all(PathBuf::from(&test_dir).join("skills"));
        let _ = std::fs::create_dir_all(PathBuf::from(&test_dir).join("conversations"));

        let tool = DoctorTool::new(&config, vec![]);
        let fixes = tool.auto_fix().await;

        let fix_text = fixes.join("\n");
        assert!(fix_text.contains("nothing to fix"), "Should report nothing to fix");

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_doctor_execute_with_auto_fix() {
        let test_dir = format!("/tmp/sandy_doctor_exec_{}", uuid::Uuid::new_v4());
        let mut config = test_config();
        config.data_dir = test_dir.clone();

        let tool = DoctorTool::new(&config, vec!["bash".into()]);
        let result = tool.execute(json!({"auto_fix": true})).await;

        assert!(!result.is_error);
        assert!(result.content.contains("Auto-Fix Results"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_doctor_execute_without_auto_fix() {
        let tool = DoctorTool::new(&test_config(), vec!["bash".into()]);
        let result = tool.execute(json!({})).await;

        assert!(!result.is_error);
        assert!(!result.content.contains("Auto-Fix Results"));
    }
}
