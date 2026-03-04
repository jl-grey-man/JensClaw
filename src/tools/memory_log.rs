use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::claude::ToolDefinition;
use super::{auth_context_from_input, schema_object, Tool, ToolResult};

lazy_static::lazy_static! {
    /// File-level lock for memory .md file appends.
    /// Prevents concurrent log_memory calls from overwriting each other.
    static ref MEMORY_LOG_LOCK: Mutex<()> = Mutex::new(());
}

pub struct MemoryLogTool {
    memory_dir: PathBuf,
}

impl MemoryLogTool {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self { memory_dir }
    }
    
    /// Extract the last N entries from a markdown memory file
    fn get_last_entries(content: &str, n: usize) -> String {
        let entries: Vec<&str> = content.split("\n## ").collect();
        
        if entries.len() <= 1 {
            // No entries or just header
            return content.to_string();
        }
        
        // Take last n entries (skip first empty split)
        let start_idx = if entries.len() > n + 1 { entries.len() - n } else { 1 };
        let last_entries: Vec<String> = entries[start_idx..]
            .iter()
            .map(|e| format!("## {}", e))
            .collect();
            
        last_entries.join("\n")
    }
}

#[async_trait]
impl Tool for MemoryLogTool {
    fn name(&self) -> &str {
        "log_memory"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "log_memory".into(),
            description: "Append to long-term memory with timestamp. Categories: solutions (MUST include verification proof), errors, patterns, insights. Be specific — no vague entries.".into(),
            input_schema: schema_object(
                json!({
                    "category": {
                        "type": "string",
                        "enum": ["solutions", "errors", "patterns", "insights"],
                        "description": "What type of memory to record. 'solutions' for fixes that worked (MUST include verification), 'errors' for problems encountered, 'patterns' for recurring behaviors, 'insights' for long-term learnings."
                    },
                    "content": {
                        "type": "string",
                        "description": "What to remember. MUST be specific and include context. For solutions: include what you did AND how you verified it worked. For errors: include full error message and context."
                    },
                    "verification": {
                        "type": "string",
                        "description": "REQUIRED for category='solutions'. Proof that the solution works (e.g., 'Ran test, output showed X', 'Checked file, contains Y', 'Service status shows running'). Leave empty for other categories."
                    }
                }),
                &["category", "content"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        if auth_context_from_input(&input).is_none() {
            return ToolResult::error("Permission denied: missing auth context".into());
        }

        let category = match input.get("category").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'category' parameter".into()),
        };

        let content = match input.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter".into()),
        };

        // Validate category
        if !["solutions", "errors", "patterns", "insights"].contains(&category) {
            return ToolResult::error(
                "Category must be one of: solutions, errors, patterns, insights".into(),
            );
        }

        // GUARDRAIL: Require verification for solutions
        if category == "solutions" {
            let verification = input.get("verification").and_then(|v| v.as_str());

            if verification.is_none() || verification.unwrap().trim().is_empty() {
                return ToolResult::error(
                    "⚠️ VERIFICATION REQUIRED for solutions! You must provide proof that the solution works. Example: 'Ran command X, output showed Y' or 'Checked service status, shows running'. This prevents logging false solutions.".into()
                );
            }

            // GUARDRAIL: Check for vague content
            let content_lower = content.to_lowercase();
            let vague_words = ["fixed", "resolved", "works now", "should be", "probably"];
            let is_vague = vague_words.iter().any(|&word| {
                content_lower.contains(word) && !content_lower.contains("verified") && !content_lower.contains("tested")
            });

            if is_vague && verification.unwrap().len() < 20 {
                return ToolResult::error(
                    "⚠️ Solution looks vague! Include specific details: what file you changed, what command you ran, what the output was. Vague logs lead to hallucinated solutions.".into()
                );
            }
        }

        // GUARDRAIL: Check content length (too short = probably vague)
        if content.len() < 30 {
            return ToolResult::error(
                "⚠️ Content too short! Be specific. Include: what, why, how, and context. Short entries are usually hallucinations.".into()
            );
        }

        // GUARDRAIL: Max length to prevent context exhaustion
        if content.len() > 5000 {
            return ToolResult::error(
                "⚠️ Content too long (max 5000 chars). Be concise and specific.".into()
            );
        }

        // GUARDRAIL: Reject content containing XML-like closing tags that match Sandy's prompt structure
        let forbidden_tags = [
            "</recent_solutions>", "</recent_insights>", "</recent_patterns>",
            "</recent_errors>", "</global_memory>", "</chat_memory>",
            "</user_message>", "</system>",
        ];
        let content_lower = content.to_lowercase();
        for tag in &forbidden_tags {
            if content_lower.contains(tag) {
                return ToolResult::error(format!(
                    "⚠️ Content contains forbidden XML tag '{}'. This looks like a prompt injection attempt.",
                    tag
                ));
            }
        }

        // Create memory directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(&self.memory_dir).await {
            return ToolResult::error(format!("Failed to create memory directory: {}", e));
        }

        let file_path = self.memory_dir.join(format!("{}.md", category));

        // Format entry with timestamp and verification
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let entry = if category == "solutions" {
            let verification = input.get("verification").and_then(|v| v.as_str()).unwrap();
            format!(
                "\n## {}\n\n{}\n\n**Verification:** {}\n",
                timestamp, content, verification
            )
        } else {
            format!("\n## {}\n\n{}\n", timestamp, content)
        };

        // Append to file (under lock to prevent concurrent write loss)
        let _lock = MEMORY_LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
        let new_content = format!("{}{}", existing, entry);

        // Get last entries BEFORE writing (to avoid borrow issues)
        let last_entries = Self::get_last_entries(&new_content, 5);

        if let Err(e) = std::fs::write(&file_path, new_content) {
            return ToolResult::error(format!("Failed to write to memory: {}", e));
        }

        // Return success message with last 5 entries for context refresh
        let suffix = if category == "solutions" { " with verification" } else { "" };

        let mut result = format!(
            "✅ Recorded to {}.md{}\n\n[MEMORY REFRESH - Last 5 entries from {}.md:]\n{}",
            category, suffix, category, last_entries
        );

        // Always include insights.md content so user preferences take effect immediately
        if category != "insights" {
            let insights_path = self.memory_dir.join("insights.md");
            if let Ok(insights_content) = std::fs::read_to_string(&insights_path) {
                if !insights_content.trim().is_empty() {
                    let insights_entries = Self::get_last_entries(&insights_content, 5);
                    result.push_str(&format!(
                        "\n\n[ACTIVE RULES from insights.md:]\n{}",
                        insights_entries
                    ));
                }
            }
        }

        ToolResult::success(result)
    }
}
