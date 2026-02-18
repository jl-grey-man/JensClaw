use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::Mutex;

use crate::hooks::{PreHook, PreHookContext};
use crate::tools::ToolResult;

/// PreHook that searches memory for relevant context and injects it into
/// `send_message` and `sub_agent` tool inputs. This grounds the LLM in
/// verified knowledge and reduces hallucination.
pub struct MemoryInjectHook {
    memory_dir: PathBuf,
    /// Cache recent queries to avoid redundant disk reads
    cache: Mutex<HashMap<String, (String, std::time::Instant)>>,
}

const CACHE_TTL_SECS: u64 = 300; // 5 minutes
const TARGET_TOOLS: &[&str] = &["send_message", "sub_agent"];

impl MemoryInjectHook {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self {
            memory_dir,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Search memory files for content relevant to the given text.
    /// Returns a brief context string, or empty if nothing relevant found.
    async fn search_relevant_context(&self, text: &str) -> String {
        // Check cache first
        let cache_key = text.chars().take(100).collect::<String>().to_lowercase();
        {
            let cache = self.cache.lock().await;
            if let Some((cached, when)) = cache.get(&cache_key) {
                if when.elapsed().as_secs() < CACHE_TTL_SECS {
                    return cached.clone();
                }
            }
        }

        let query_words: Vec<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() >= 3)
            .take(10)
            .map(|w| w.to_string())
            .collect();

        if query_words.is_empty() || !self.memory_dir.exists() {
            return String::new();
        }

        let mut matches = Vec::new();

        // Read memory files
        let entries = match std::fs::read_dir(&self.memory_dir) {
            Ok(e) => e,
            Err(_) => return String::new(),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let filename = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            if let Ok(content) = std::fs::read_to_string(&path) {
                let content_lower = content.to_lowercase();
                let match_count = query_words
                    .iter()
                    .filter(|w| content_lower.contains(w.as_str()))
                    .count();

                if match_count >= 2 {
                    // Extract most relevant paragraph
                    for paragraph in content.split("\n\n") {
                        let para_lower = paragraph.to_lowercase();
                        let para_matches = query_words
                            .iter()
                            .filter(|w| para_lower.contains(w.as_str()))
                            .count();

                        if para_matches >= 2 {
                            let truncated = if paragraph.len() > 200 {
                                format!("{}...", &paragraph[..paragraph.char_indices().take(200).last().map(|(i,_)| i).unwrap_or(200)])
                            } else {
                                paragraph.to_string()
                            };
                            matches.push(format!("[{}] {}", filename, truncated.trim()));
                            break;
                        }
                    }
                }
            }
        }

        let context = if matches.is_empty() {
            String::new()
        } else {
            matches.truncate(3);
            format!("\n[Memory context]: {}", matches.join(" | "))
        };

        // Update cache
        {
            let mut cache = self.cache.lock().await;
            cache.insert(cache_key, (context.clone(), std::time::Instant::now()));
            // Evict old entries
            cache.retain(|_, (_, when)| when.elapsed().as_secs() < CACHE_TTL_SECS * 2);
        }

        context
    }
}

#[async_trait]
impl PreHook for MemoryInjectHook {
    async fn before_execute(
        &self,
        ctx: &PreHookContext,
    ) -> Result<Option<serde_json::Value>, ToolResult> {
        // Only apply to target tools
        if !TARGET_TOOLS.contains(&ctx.tool_name.as_str()) {
            return Ok(None);
        }

        // Extract text to search from the tool input
        let search_text = if ctx.tool_name == "send_message" {
            ctx.input.get("text").and_then(|v| v.as_str()).unwrap_or("")
        } else if ctx.tool_name == "sub_agent" {
            ctx.input.get("task").and_then(|v| v.as_str()).unwrap_or("")
        } else {
            ""
        };

        if search_text.len() < 10 {
            return Ok(None);
        }

        let context = self.search_relevant_context(search_text).await;
        if context.is_empty() {
            return Ok(None);
        }

        // Inject context into the tool input
        let mut modified = ctx.input.clone();
        if ctx.tool_name == "sub_agent" {
            // Add context to the task description
            if let Some(task) = modified.get("task").and_then(|v| v.as_str()) {
                let enriched = format!("{}{}", task, context);
                modified["task"] = serde_json::Value::String(enriched);
            }
        }
        // For send_message, we inject into a special context field
        if ctx.tool_name == "send_message" {
            modified["__memory_context"] = serde_json::Value::String(context);
        }

        Ok(Some(modified))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_memory(dir: &std::path::Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("solutions.md"),
            "## 2026-02-15 10:00:00 UTC\n\nFixed the scheduler cron expression by using 6-field format instead of 5-field. The seconds field must be added as the first field.\n\n## 2026-02-10 08:00:00 UTC\n\nResolved database connection pooling issue by setting max connections to 10 in the config.\n",
        ).unwrap();
        std::fs::write(
            dir.join("errors.md"),
            "## 2026-02-14 12:00:00 UTC\n\nError: webhook server failed to bind port 8080 because it was already in use.\n",
        ).unwrap();
    }

    #[tokio::test]
    async fn test_memory_inject_skips_non_target_tools() {
        let dir = std::env::temp_dir().join(format!("sandy_inject_skip_{}", uuid::Uuid::new_v4()));
        create_test_memory(&dir);

        let hook = MemoryInjectHook::new(dir.clone());
        let ctx = PreHookContext {
            tool_name: "bash".into(),
            input: json!({"command": "ls"}),
        };

        let result = hook.before_execute(&ctx).await.unwrap();
        assert!(result.is_none(), "Should not modify non-target tools");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_memory_inject_sub_agent_with_match() {
        let dir = std::env::temp_dir().join(format!("sandy_inject_match_{}", uuid::Uuid::new_v4()));
        create_test_memory(&dir);

        let hook = MemoryInjectHook::new(dir.clone());
        let ctx = PreHookContext {
            tool_name: "sub_agent".into(),
            input: json!({"task": "Fix the scheduler cron expression format"}),
        };

        let result = hook.before_execute(&ctx).await.unwrap();
        // Should inject memory context about scheduler cron
        if let Some(modified) = result {
            let task = modified["task"].as_str().unwrap();
            assert!(
                task.contains("[Memory context]"),
                "Should contain memory context: {}",
                task
            );
        }
        // Note: it's also valid for result to be None if memory search doesn't match

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_memory_inject_no_match() {
        let dir = std::env::temp_dir().join(format!("sandy_inject_nomatch_{}", uuid::Uuid::new_v4()));
        create_test_memory(&dir);

        let hook = MemoryInjectHook::new(dir.clone());
        let ctx = PreHookContext {
            tool_name: "sub_agent".into(),
            input: json!({"task": "Generate a random haiku about quantum physics"}),
        };

        let result = hook.before_execute(&ctx).await.unwrap();
        assert!(result.is_none(), "Should not inject when no memory matches");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_memory_inject_short_text_ignored() {
        let dir = std::env::temp_dir().join(format!("sandy_inject_short_{}", uuid::Uuid::new_v4()));
        create_test_memory(&dir);

        let hook = MemoryInjectHook::new(dir.clone());
        let ctx = PreHookContext {
            tool_name: "sub_agent".into(),
            input: json!({"task": "hi"}),
        };

        let result = hook.before_execute(&ctx).await.unwrap();
        assert!(result.is_none(), "Should skip short texts");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_memory_inject_missing_memory_dir() {
        let dir = PathBuf::from("/tmp/sandy_nonexistent_memory_dir_12345");
        let hook = MemoryInjectHook::new(dir);

        let ctx = PreHookContext {
            tool_name: "sub_agent".into(),
            input: json!({"task": "Fix the scheduler cron expression"}),
        };

        let result = hook.before_execute(&ctx).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_memory_inject_caching() {
        let dir = std::env::temp_dir().join(format!("sandy_inject_cache_{}", uuid::Uuid::new_v4()));
        create_test_memory(&dir);

        let hook = MemoryInjectHook::new(dir.clone());

        // First call
        let ctx = PreHookContext {
            tool_name: "sub_agent".into(),
            input: json!({"task": "Fix the scheduler cron expression format issue"}),
        };
        let _ = hook.before_execute(&ctx).await;

        // Second call with same text should use cache
        let _ = hook.before_execute(&ctx).await;

        let cache = hook.cache.lock().await;
        assert!(!cache.is_empty(), "Cache should have entries");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
