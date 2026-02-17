use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;
use std::path::PathBuf;
use tokio::sync::Mutex;

use crate::hooks::{PostHook, PostHookContext};

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_ROTATIONS: u32 = 3;

#[derive(Serialize)]
struct LogEntry {
    timestamp: String,
    tool_name: String,
    duration_ms: u64,
    success: bool,
    error_type: Option<String>,
}

/// PostHook that logs every tool execution to a JSONL file with rotation.
pub struct ExecLogHook {
    log_path: PathBuf,
    inner: Mutex<()>, // Serialize writes
}

impl ExecLogHook {
    pub fn new(data_dir: &str) -> Self {
        let log_path = PathBuf::from(data_dir).join("exec_log.jsonl");
        Self {
            log_path,
            inner: Mutex::new(()),
        }
    }

    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            log_path: path,
            inner: Mutex::new(()),
        }
    }

    fn rotate(&self) -> std::io::Result<()> {
        // Check file size
        let metadata = match std::fs::metadata(&self.log_path) {
            Ok(m) => m,
            Err(_) => return Ok(()), // File doesn't exist yet
        };

        if metadata.len() < MAX_FILE_SIZE {
            return Ok(());
        }

        // Rotate: .3 -> delete, .2 -> .3, .1 -> .2, current -> .1
        for i in (1..MAX_ROTATIONS).rev() {
            let from = self.log_path.with_extension(format!("jsonl.{}", i));
            let to = self.log_path.with_extension(format!("jsonl.{}", i + 1));
            if from.exists() {
                let _ = std::fs::rename(&from, &to);
            }
        }

        // Current -> .1
        let rotated = self.log_path.with_extension("jsonl.1");
        std::fs::rename(&self.log_path, &rotated)?;

        Ok(())
    }

    fn classify_error(content: &str) -> Option<String> {
        if content.contains("Permission denied") {
            Some("permission".into())
        } else if content.contains("not found") || content.contains("Unknown tool") {
            Some("not_found".into())
        } else if content.contains("timeout") || content.contains("Timeout") {
            Some("timeout".into())
        } else if content.contains("Loop detected") {
            Some("loop".into())
        } else {
            Some("other".into())
        }
    }
}

#[async_trait]
impl PostHook for ExecLogHook {
    async fn after_execute(&self, ctx: &PostHookContext) {
        let _lock = self.inner.lock().await;

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339(),
            tool_name: ctx.tool_name.clone(),
            duration_ms: ctx.duration.as_millis() as u64,
            success: !ctx.result.is_error,
            error_type: if ctx.result.is_error {
                Self::classify_error(&ctx.result.content)
            } else {
                None
            },
        };

        // Rotate if needed
        if let Err(e) = self.rotate() {
            tracing::warn!("Log rotation failed: {}", e);
        }

        // Ensure parent dir exists
        if let Some(parent) = self.log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Append JSONL entry
        let line = match serde_json::to_string(&entry) {
            Ok(json) => format!("{}\n", json),
            Err(e) => {
                tracing::warn!("Failed to serialize log entry: {}", e);
                return;
            }
        };

        if let Err(e) = append_to_file(&self.log_path, &line) {
            tracing::warn!("Failed to write exec log: {}", e);
        }
    }
}

fn append_to_file(path: &PathBuf, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolResult;
    use std::time::Duration;

    fn temp_log_path() -> PathBuf {
        std::env::temp_dir().join(format!("sandy_exec_log_test_{}.jsonl", uuid::Uuid::new_v4()))
    }

    #[tokio::test]
    async fn test_exec_log_writes_jsonl() {
        let path = temp_log_path();
        let hook = ExecLogHook::with_path(path.clone());

        let ctx = PostHookContext {
            tool_name: "bash".into(),
            input: serde_json::json!({"command": "ls"}),
            result: ToolResult::success("file1\nfile2".into()),
            duration: Duration::from_millis(42),
        };

        hook.after_execute(&ctx).await;

        let content = std::fs::read_to_string(&path).unwrap();
        let entry: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry["tool_name"], "bash");
        assert_eq!(entry["duration_ms"], 42);
        assert_eq!(entry["success"], true);
        assert!(entry["error_type"].is_null());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_exec_log_error_entry() {
        let path = temp_log_path();
        let hook = ExecLogHook::with_path(path.clone());

        let ctx = PostHookContext {
            tool_name: "read_file".into(),
            input: serde_json::json!({"path": "/nonexistent"}),
            result: ToolResult::error("File not found".into()),
            duration: Duration::from_millis(5),
        };

        hook.after_execute(&ctx).await;

        let content = std::fs::read_to_string(&path).unwrap();
        let entry: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry["success"], false);
        assert_eq!(entry["error_type"], "not_found");

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_exec_log_multiple_entries() {
        let path = temp_log_path();
        let hook = ExecLogHook::with_path(path.clone());

        for i in 0..3 {
            let ctx = PostHookContext {
                tool_name: format!("tool_{}", i),
                input: serde_json::json!({}),
                result: ToolResult::success("ok".into()),
                duration: Duration::from_millis(i as u64),
            };
            hook.after_execute(&ctx).await;
        }

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_rotation_logic() {
        let path = temp_log_path();
        let hook = ExecLogHook::with_path(path.clone());

        // Create a file that exceeds MAX_FILE_SIZE
        let big_content = "x".repeat((MAX_FILE_SIZE + 1) as usize);
        std::fs::write(&path, &big_content).unwrap();

        hook.rotate().unwrap();

        // Original should be gone, .1 should exist
        assert!(!path.exists());
        let rotated = path.with_extension("jsonl.1");
        assert!(rotated.exists());

        let _ = std::fs::remove_file(&rotated);
    }

    #[test]
    fn test_classify_error() {
        assert_eq!(
            ExecLogHook::classify_error("Permission denied: chat 123"),
            Some("permission".into())
        );
        assert_eq!(
            ExecLogHook::classify_error("File not found"),
            Some("not_found".into())
        );
        assert_eq!(
            ExecLogHook::classify_error("Loop detected: tool bash"),
            Some("loop".into())
        );
        assert_eq!(
            ExecLogHook::classify_error("Something went wrong"),
            Some("other".into())
        );
    }
}
