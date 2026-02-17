use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use tokio::sync::Mutex;

use crate::hooks::{PreHook, PreHookContext};
use crate::tools::ToolResult;

/// PreHook that detects repeated tool calls and blocks loops.
/// Tracks a rolling window of (tool_name, hash(input)) pairs.
/// Blocks if the same call appears >= max_repeats times within the window.
pub struct LoopDetectHook {
    window: Mutex<VecDeque<(String, u64)>>,
    window_size: usize,
    max_repeats: usize,
}

impl LoopDetectHook {
    pub fn new() -> Self {
        Self {
            window: Mutex::new(VecDeque::new()),
            window_size: 8,
            max_repeats: 3,
        }
    }

    #[cfg(test)]
    pub fn with_params(window_size: usize, max_repeats: usize) -> Self {
        Self {
            window: Mutex::new(VecDeque::new()),
            window_size,
            max_repeats,
        }
    }

    fn hash_input(input: &serde_json::Value) -> u64 {
        let mut hasher = DefaultHasher::new();
        let s = input.to_string();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for LoopDetectHook {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PreHook for LoopDetectHook {
    async fn before_execute(
        &self,
        ctx: &PreHookContext,
    ) -> Result<Option<serde_json::Value>, ToolResult> {
        let input_hash = Self::hash_input(&ctx.input);
        let key = (ctx.tool_name.clone(), input_hash);

        let mut window = self.window.lock().await;

        // Count occurrences of this exact call in the window
        let count = window
            .iter()
            .filter(|(name, hash)| name == &ctx.tool_name && *hash == input_hash)
            .count();

        if count >= self.max_repeats {
            tracing::warn!(
                "Loop detected: tool '{}' called {} times with same input in last {} calls",
                ctx.tool_name,
                count + 1,
                self.window_size
            );
            return Err(ToolResult::error(format!(
                "Loop detected: '{}' has been called {} times with the same input. \
                 Try a different approach or modify your input.",
                ctx.tool_name,
                count + 1
            )));
        }

        // Add to window
        window.push_back(key);
        if window.len() > self.window_size {
            window.pop_front();
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_loop_detect_allows_varied_calls() {
        let hook = LoopDetectHook::new();
        for i in 0..10 {
            let ctx = PreHookContext {
                tool_name: "bash".into(),
                input: json!({"command": format!("cmd_{}", i)}),
            };
            let result = hook.before_execute(&ctx).await;
            assert!(result.is_ok(), "Call {} should be allowed", i);
        }
    }

    #[tokio::test]
    async fn test_loop_detect_blocks_after_3_repeats() {
        let hook = LoopDetectHook::new();
        let input = json!({"command": "ls"});

        // First 3 calls should be allowed (count goes 0, 1, 2)
        for i in 0..3 {
            let ctx = PreHookContext {
                tool_name: "bash".into(),
                input: input.clone(),
            };
            let result = hook.before_execute(&ctx).await;
            assert!(result.is_ok(), "Call {} should be allowed", i);
        }

        // 4th identical call should be blocked (count = 3)
        let ctx = PreHookContext {
            tool_name: "bash".into(),
            input: input.clone(),
        };
        let result = hook.before_execute(&ctx).await;
        assert!(result.is_err(), "4th identical call should be blocked");
        let err = result.unwrap_err();
        assert!(err.content.contains("Loop detected"));
    }

    #[tokio::test]
    async fn test_loop_detect_different_tools_not_blocked() {
        let hook = LoopDetectHook::new();
        let input = json!({"path": "/tmp"});

        for tool in &["bash", "read_file", "glob", "grep"] {
            let ctx = PreHookContext {
                tool_name: tool.to_string(),
                input: input.clone(),
            };
            let result = hook.before_execute(&ctx).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_loop_detect_window_slides() {
        let hook = LoopDetectHook::with_params(4, 2);
        let repeated = json!({"x": 1});

        // Fill window: [A, A]
        for _ in 0..2 {
            let ctx = PreHookContext {
                tool_name: "tool_a".into(),
                input: repeated.clone(),
            };
            assert!(hook.before_execute(&ctx).await.is_ok());
        }

        // 3rd identical should be blocked (count=2 >= max_repeats=2)
        let ctx = PreHookContext {
            tool_name: "tool_a".into(),
            input: repeated.clone(),
        };
        assert!(hook.before_execute(&ctx).await.is_err());

        // Push different calls to slide window past the old ones
        for i in 0..4 {
            let ctx = PreHookContext {
                tool_name: "other".into(),
                input: json!({"i": i}),
            };
            // Reset window state - we need a fresh hook for this test
            let _ = hook.before_execute(&ctx).await;
        }

        // Now original call should work again (old entries slid out)
        let ctx = PreHookContext {
            tool_name: "tool_a".into(),
            input: repeated.clone(),
        };
        assert!(hook.before_execute(&ctx).await.is_ok());
    }

    #[tokio::test]
    async fn test_loop_detect_same_tool_different_input() {
        let hook = LoopDetectHook::new();

        // Same tool, different inputs should all pass
        for i in 0..8 {
            let ctx = PreHookContext {
                tool_name: "bash".into(),
                input: json!({"command": format!("echo {}", i)}),
            };
            assert!(hook.before_execute(&ctx).await.is_ok());
        }
    }
}
