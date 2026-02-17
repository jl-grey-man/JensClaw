use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::path::PathBuf;

use crate::claude::ToolDefinition;
use super::{schema_object, Tool, ToolResult};

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
            description: "Record learnings to long-term memory. Use this to remember solutions, patterns, errors, or insights that should persist across sessions. This APPENDS to memory files with timestamps.

⚠️ CRITICAL GUARDRAILS:
1. ONLY log solutions AFTER verifying they work
2. NEVER log assumptions or guesses
3. Include evidence/proof in the content
4. Be specific: include file paths, commands, error messages
5. If unsure, test first, log second

Example BAD log: 'Fixed the scheduler'
Example GOOD log: 'Fixed scheduler by updating AGENTS.md line 25 to use list_scheduled_tasks instead of list_tasks. Verified with: sudo systemctl status sandy shows Running'".into(),
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

        // Append to file
        let existing = tokio::fs::read_to_string(&file_path).await.unwrap_or_default();
        let new_content = format!("{}{}", existing, entry);
        
        // Get last entries BEFORE writing (to avoid borrow issues)
        let last_entries = Self::get_last_entries(&new_content, 5);
        
        if let Err(e) = tokio::fs::write(&file_path, new_content).await {
            return ToolResult::error(format!("Failed to write to memory: {}", e));
        }

        // Return success message with last 5 entries for context refresh
        let suffix = if category == "solutions" { " with verification" } else { "" };
        
        ToolResult::success(format!(
            "✅ Recorded to {}.md{}\n\n[MEMORY REFRESH - Last 5 entries from {}.md:]\n{}",
            category, suffix, category, last_entries
        ))
    }
}
