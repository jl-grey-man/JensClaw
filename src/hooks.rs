pub mod loop_detect;
pub mod memory_inject;

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::tools::ToolResult;

/// Context passed to hooks before tool execution.
pub struct PreHookContext {
    pub tool_name: String,
    pub input: serde_json::Value,
}

/// Context passed to hooks after tool execution.
pub struct PostHookContext {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub result: ToolResult,
    pub duration: std::time::Duration,
}

/// A hook that runs before tool execution. Can block execution by returning Err.
#[async_trait]
pub trait PreHook: Send + Sync {
    /// Returns Ok(possibly modified input) to proceed, or Err(ToolResult) to block.
    async fn before_execute(
        &self,
        ctx: &PreHookContext,
    ) -> Result<Option<serde_json::Value>, ToolResult>;
}

/// A hook that runs after tool execution. Cannot block, only observe/log.
#[async_trait]
pub trait PostHook: Send + Sync {
    async fn after_execute(&self, ctx: &PostHookContext);
}

/// Registry of pre/post hooks applied to all tool executions.
pub struct HookRegistry {
    pre_hooks: Vec<Arc<dyn PreHook>>,
    post_hooks: Vec<Arc<dyn PostHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
        }
    }

    pub fn add_pre_hook(&mut self, hook: Arc<dyn PreHook>) {
        self.pre_hooks.push(hook);
    }

    pub fn add_post_hook(&mut self, hook: Arc<dyn PostHook>) {
        self.post_hooks.push(hook);
    }

    /// Run all pre-hooks. Returns the (possibly modified) input, or a blocking ToolResult.
    pub async fn run_pre_hooks(
        &self,
        tool_name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, ToolResult> {
        let mut current_input = input;
        for hook in &self.pre_hooks {
            let ctx = PreHookContext {
                tool_name: tool_name.to_string(),
                input: current_input.clone(),
            };
            match hook.before_execute(&ctx).await {
                Ok(Some(modified)) => current_input = modified,
                Ok(None) => {} // no modification
                Err(block) => return Err(block),
            }
        }
        Ok(current_input)
    }

    /// Run all post-hooks.
    pub async fn run_post_hooks(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        result: &ToolResult,
        duration: std::time::Duration,
    ) {
        let ctx = PostHookContext {
            tool_name: tool_name.to_string(),
            input: input.clone(),
            result: ToolResult {
                content: result.content.clone(),
                is_error: result.is_error,
            },
            duration,
        };
        for hook in &self.post_hooks {
            hook.after_execute(&ctx).await;
        }
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Measure tool execution with hooks applied.
pub async fn execute_with_hooks(
    hooks: &HookRegistry,
    tool_name: &str,
    input: serde_json::Value,
    execute_fn: impl std::future::Future<Output = ToolResult>,
) -> ToolResult {
    // Run pre-hooks
    let input = match hooks.run_pre_hooks(tool_name, input).await {
        Ok(input) => input,
        Err(blocked) => return blocked,
    };

    let start = Instant::now();
    let result = execute_fn.await;
    let duration = start.elapsed();

    // Run post-hooks
    hooks
        .run_post_hooks(tool_name, &input, &result, duration)
        .await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BlockingHook;
    #[async_trait]
    impl PreHook for BlockingHook {
        async fn before_execute(
            &self,
            _ctx: &PreHookContext,
        ) -> Result<Option<serde_json::Value>, ToolResult> {
            Err(ToolResult::error("Blocked by hook".into()))
        }
    }

    struct PassthroughHook;
    #[async_trait]
    impl PreHook for PassthroughHook {
        async fn before_execute(
            &self,
            _ctx: &PreHookContext,
        ) -> Result<Option<serde_json::Value>, ToolResult> {
            Ok(None)
        }
    }

    struct CountingPostHook {
        count: Arc<tokio::sync::Mutex<u32>>,
    }
    #[async_trait]
    impl PostHook for CountingPostHook {
        async fn after_execute(&self, _ctx: &PostHookContext) {
            let mut c = self.count.lock().await;
            *c += 1;
        }
    }

    #[tokio::test]
    async fn test_hook_registry_empty() {
        let registry = HookRegistry::new();
        let input = serde_json::json!({"key": "value"});
        let result = registry.run_pre_hooks("test_tool", input.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), input);
    }

    #[tokio::test]
    async fn test_hook_registry_passthrough() {
        let mut registry = HookRegistry::new();
        registry.add_pre_hook(Arc::new(PassthroughHook));
        let input = serde_json::json!({"key": "value"});
        let result = registry.run_pre_hooks("test_tool", input.clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hook_registry_blocking() {
        let mut registry = HookRegistry::new();
        registry.add_pre_hook(Arc::new(BlockingHook));
        let input = serde_json::json!({});
        let result = registry.run_pre_hooks("test_tool", input).await;
        assert!(result.is_err());
        let blocked = result.unwrap_err();
        assert!(blocked.is_error);
        assert!(blocked.content.contains("Blocked"));
    }

    #[tokio::test]
    async fn test_post_hook_called() {
        let count = Arc::new(tokio::sync::Mutex::new(0u32));
        let mut registry = HookRegistry::new();
        registry.add_post_hook(Arc::new(CountingPostHook {
            count: count.clone(),
        }));

        let result = ToolResult::success("ok".into());
        registry
            .run_post_hooks(
                "test",
                &serde_json::json!({}),
                &result,
                std::time::Duration::from_millis(10),
            )
            .await;

        assert_eq!(*count.lock().await, 1);
    }

    #[tokio::test]
    async fn test_execute_with_hooks_passthrough() {
        let registry = HookRegistry::new();
        let result = execute_with_hooks(
            &registry,
            "test",
            serde_json::json!({}),
            async { ToolResult::success("done".into()) },
        )
        .await;
        assert!(!result.is_error);
        assert_eq!(result.content, "done");
    }

    #[tokio::test]
    async fn test_execute_with_hooks_blocked() {
        let mut registry = HookRegistry::new();
        registry.add_pre_hook(Arc::new(BlockingHook));
        let result = execute_with_hooks(
            &registry,
            "test",
            serde_json::json!({}),
            async { ToolResult::success("should not reach".into()) },
        )
        .await;
        assert!(result.is_error);
        assert!(result.content.contains("Blocked"));
    }
}
