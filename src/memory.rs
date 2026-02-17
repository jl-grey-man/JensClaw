use std::path::{Path, PathBuf};

pub struct MemoryManager {
    base_dir: PathBuf,
    groups_dir: PathBuf,
}

impl MemoryManager {
    pub fn new(data_dir: &str) -> Self {
        let base = PathBuf::from(data_dir);
        MemoryManager {
            groups_dir: base.join("groups"),
            base_dir: base,
        }
    }

    fn global_memory_path(&self) -> PathBuf {
        self.groups_dir.join("AGENTS.md")
    }

    fn chat_memory_path(&self, chat_id: i64) -> PathBuf {
        self.groups_dir.join(chat_id.to_string()).join("AGENTS.md")
    }

    pub fn read_global_memory(&self) -> Option<String> {
        let path = self.global_memory_path();
        std::fs::read_to_string(path).ok()
    }

    pub fn read_chat_memory(&self, chat_id: i64) -> Option<String> {
        let path = self.chat_memory_path(chat_id);
        std::fs::read_to_string(path).ok()
    }

    #[allow(dead_code)]
    pub fn write_global_memory(&self, content: &str) -> std::io::Result<()> {
        let path = self.global_memory_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    #[allow(dead_code)]
    pub fn write_chat_memory(&self, chat_id: i64, content: &str) -> std::io::Result<()> {
        let path = self.chat_memory_path(chat_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    /// Read the last N entries from a runtime memory file (insights, solutions, patterns, errors)
    fn read_runtime_memory_entries(&self, filename: &str, max_entries: usize) -> Option<String> {
        let memory_dir = self.base_dir.join("memory");
        let path = memory_dir.join(format!("{}.md", filename));
        
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Split by "## " headers (timestamp markers)
            let entries: Vec<&str> = content.split("\n## ").collect();
            
            if entries.len() <= 1 {
                return None; // No entries
            }
            
            // Take last N entries (skip first empty split)
            let start_idx = if entries.len() > max_entries + 1 { 
                entries.len() - max_entries 
            } else { 
                1 
            };
            
            let last_entries: Vec<String> = entries[start_idx..]
                .iter()
                .map(|e| format!("## {}", e.trim()))
                .collect();
            
            if last_entries.is_empty() {
                None
            } else {
                Some(last_entries.join("\n\n"))
            }
        } else {
            None
        }
    }

    pub fn build_memory_context(&self, chat_id: i64) -> String {
        let mut context = String::new();

        // Add AGENTS.md (global and chat-specific)
        if let Some(global) = self.read_global_memory() {
            if !global.trim().is_empty() {
                context.push_str("<global_memory>\n");
                context.push_str(&global);
                context.push_str("\n</global_memory>\n\n");
            }
        }

        if let Some(chat) = self.read_chat_memory(chat_id) {
            if !chat.trim().is_empty() {
                context.push_str("<chat_memory>\n");
                context.push_str(&chat);
                context.push_str("\n</chat_memory>\n\n");
            }
        }

        // Inject last 5 insights at conversation start
        if let Some(insights) = self.read_runtime_memory_entries("insights", 5) {
            context.push_str("<recent_insights>\n");
            context.push_str("Most recent learnings and rules:\n\n");
            context.push_str(&insights);
            context.push_str("\n</recent_insights>\n\n");
        }

        context
    }

    #[allow(dead_code)]
    pub fn groups_dir(&self) -> &Path {
        &self.groups_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_memory_manager() -> (MemoryManager, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("microclaw_mem_test_{}", uuid::Uuid::new_v4()));
        let mm = MemoryManager::new(dir.to_str().unwrap());
        (mm, dir)
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_global_memory_path() {
        let (mm, dir) = test_memory_manager();
        let path = mm.global_memory_path();
        assert!(path.ends_with("groups/AGENTS.md"));
        cleanup(&dir);
    }

    #[test]
    fn test_chat_memory_path() {
        let (mm, dir) = test_memory_manager();
        let path = mm.chat_memory_path(12345);
        assert!(path.to_str().unwrap().contains("groups/12345/AGENTS.md"));
        cleanup(&dir);
    }

    #[test]
    fn test_read_nonexistent_memory() {
        let (mm, dir) = test_memory_manager();
        assert!(mm.read_global_memory().is_none());
        assert!(mm.read_chat_memory(100).is_none());
        cleanup(&dir);
    }

    #[test]
    fn test_write_and_read_global_memory() {
        let (mm, dir) = test_memory_manager();
        mm.write_global_memory("global notes").unwrap();
        let content = mm.read_global_memory().unwrap();
        assert_eq!(content, "global notes");
        cleanup(&dir);
    }

    #[test]
    fn test_write_and_read_chat_memory() {
        let (mm, dir) = test_memory_manager();
        mm.write_chat_memory(42, "chat 42 notes").unwrap();
        let content = mm.read_chat_memory(42).unwrap();
        assert_eq!(content, "chat 42 notes");

        // Different chat should be empty
        assert!(mm.read_chat_memory(99).is_none());
        cleanup(&dir);
    }

    #[test]
    fn test_build_memory_context_empty() {
        let (mm, dir) = test_memory_manager();
        let ctx = mm.build_memory_context(100);
        assert!(ctx.is_empty() || ctx.contains("<recent_insights>"));
        cleanup(&dir);
    }

    #[test]
    fn test_build_memory_context_with_global_only() {
        let (mm, dir) = test_memory_manager();
        mm.write_global_memory("I am global memory").unwrap();
        let ctx = mm.build_memory_context(100);
        assert!(ctx.contains("<global_memory>"));
        assert!(ctx.contains("I am global memory"));
        assert!(ctx.contains("</global_memory>"));
        assert!(!ctx.contains("<chat_memory>"));
        cleanup(&dir);
    }

    #[test]
    fn test_build_memory_context_with_both() {
        let (mm, dir) = test_memory_manager();
        mm.write_global_memory("global stuff").unwrap();
        mm.write_chat_memory(100, "chat stuff").unwrap();
        let ctx = mm.build_memory_context(100);
        assert!(ctx.contains("<global_memory>"));
        assert!(ctx.contains("global stuff"));
        assert!(ctx.contains("<chat_memory>"));
        assert!(ctx.contains("chat stuff"));
        cleanup(&dir);
    }

    #[test]
    fn test_build_memory_context_ignores_whitespace_only() {
        let (mm, dir) = test_memory_manager();
        mm.write_global_memory("   \n  ").unwrap();
        let ctx = mm.build_memory_context(100);
        // Whitespace-only content should be ignored, but insights might be present
        assert!(!ctx.contains("<global_memory>"));
        cleanup(&dir);
    }

    #[test]
    fn test_groups_dir() {
        let (mm, dir) = test_memory_manager();
        assert!(mm.groups_dir().ends_with("groups"));
        cleanup(&dir);
    }
}
