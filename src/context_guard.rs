use crate::claude::{ContentBlock, Message, MessageContent, ToolDefinition};

/// Approximate token count using ~4 chars per token heuristic.
pub fn estimate_tokens(messages: &[Message]) -> usize {
    let total_chars: usize = messages
        .iter()
        .map(|msg| match &msg.content {
            MessageContent::Text(t) => t.len(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => text.len(),
                    ContentBlock::ToolUse { name, input, .. } => {
                        name.len() + input.to_string().len()
                    }
                    ContentBlock::ToolResult { content, .. } => content.len(),
                    ContentBlock::Image { .. } => 1000, // rough estimate for image tokens
                })
                .sum(),
        })
        .sum();

    total_chars / 4
}

/// Result of a context window check.
#[derive(Debug, PartialEq)]
pub enum ContextStatus {
    /// Under 80% capacity - no action needed
    Ok,
    /// Between 80-95% - should auto-compact
    ShouldCompact,
    /// Over 95% - must compact immediately, drop oldest messages if needed
    MustCompact,
}

/// Check context window status against a model's max context.
/// `max_context_tokens` is the model's context window size.
pub fn check_context(messages: &[Message], max_context_tokens: usize) -> ContextStatus {
    let estimated = estimate_tokens(messages);

    let ratio = estimated as f64 / max_context_tokens as f64;

    if ratio >= 0.95 {
        ContextStatus::MustCompact
    } else if ratio >= 0.80 {
        ContextStatus::ShouldCompact
    } else {
        ContextStatus::Ok
    }
}

/// Estimate tokens for a system prompt string.
pub fn estimate_system_tokens(system: &str) -> usize {
    system.len() / 4
}

/// Estimate tokens for tool definitions by serializing to JSON.
pub fn estimate_tool_tokens(tools: &[ToolDefinition]) -> usize {
    let total_chars: usize = tools
        .iter()
        .map(|t| t.name.len() + t.description.len() + t.input_schema.to_string().len())
        .sum();
    total_chars / 4
}

/// Log token budget breakdown for observability.
/// Called on the first LLM iteration per user message.
pub fn log_token_budget(system_tokens: usize, tool_tokens: usize, message_tokens: usize) {
    let total = system_tokens + tool_tokens + message_tokens;
    tracing::info!(
        "token_budget system={} tools={} messages={} total={}",
        system_tokens,
        tool_tokens,
        message_tokens,
        total
    );
}

/// Emergency trim: keep only the most recent N messages.
/// Used when context hits the hard 95% limit.
pub fn emergency_trim(messages: &[Message], keep_recent: usize) -> Vec<Message> {
    if messages.len() <= keep_recent {
        return messages.to_vec();
    }
    messages[messages.len() - keep_recent..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{ImageSource, Message, MessageContent};

    fn text_msg(role: &str, text: &str) -> Message {
        Message {
            role: role.into(),
            content: MessageContent::Text(text.into()),
        }
    }

    #[test]
    fn test_estimate_tokens_simple() {
        let messages = vec![text_msg("user", "hello world")]; // 11 chars -> ~2 tokens
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 2); // 11 / 4 = 2
    }

    #[test]
    fn test_estimate_tokens_empty() {
        let tokens = estimate_tokens(&[]);
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_estimate_tokens_multiple_messages() {
        let messages = vec![
            text_msg("user", &"a".repeat(400)),    // 100 tokens
            text_msg("assistant", &"b".repeat(800)), // 200 tokens
        ];
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 300);
    }

    #[test]
    fn test_estimate_tokens_with_blocks() {
        let messages = vec![Message {
            role: "user".into(),
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "a".repeat(100).into(),
                },
                ContentBlock::ToolResult {
                    tool_use_id: "t1".into(),
                    content: "b".repeat(200).into(),
                    is_error: None,
                },
            ]),
        }];
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 75); // (100 + 200) / 4
    }

    #[test]
    fn test_estimate_tokens_with_image() {
        let messages = vec![Message {
            role: "user".into(),
            content: MessageContent::Blocks(vec![ContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".into(),
                    media_type: "image/png".into(),
                    data: "AAAA".into(),
                },
            }]),
        }];
        let tokens = estimate_tokens(&messages);
        assert_eq!(tokens, 250); // 1000 / 4
    }

    #[test]
    fn test_check_context_ok() {
        let messages = vec![text_msg("user", &"a".repeat(400))]; // 100 tokens
        assert_eq!(check_context(&messages, 1000), ContextStatus::Ok);
    }

    #[test]
    fn test_check_context_should_compact() {
        let messages = vec![text_msg("user", &"a".repeat(3400))]; // 850 tokens
        assert_eq!(check_context(&messages, 1000), ContextStatus::ShouldCompact);
    }

    #[test]
    fn test_check_context_must_compact() {
        let messages = vec![text_msg("user", &"a".repeat(3900))]; // 975 tokens
        assert_eq!(check_context(&messages, 1000), ContextStatus::MustCompact);
    }

    #[test]
    fn test_emergency_trim() {
        let messages: Vec<Message> = (0..10)
            .map(|i| text_msg("user", &format!("msg {}", i)))
            .collect();
        let trimmed = emergency_trim(&messages, 3);
        assert_eq!(trimmed.len(), 3);
        if let MessageContent::Text(t) = &trimmed[0].content {
            assert_eq!(t, "msg 7");
        }
    }

    #[test]
    fn test_emergency_trim_no_trim_needed() {
        let messages = vec![text_msg("user", "hello")];
        let trimmed = emergency_trim(&messages, 5);
        assert_eq!(trimmed.len(), 1);
    }

    #[test]
    fn test_estimate_system_tokens() {
        // 400 chars / 4 = 100 tokens
        let system = "a".repeat(400);
        assert_eq!(estimate_system_tokens(&system), 100);
    }

    #[test]
    fn test_estimate_system_tokens_empty() {
        assert_eq!(estimate_system_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tool_tokens() {
        let tools = vec![
            ToolDefinition {
                name: "bash".into(),                              // 4 chars
                description: "Execute a bash command.".into(),    // 22 chars
                input_schema: serde_json::json!({"type": "object", "properties": {"cmd": {"type": "string"}}}),
            },
        ];
        let tokens = estimate_tool_tokens(&tools);
        // name(4) + description(22) + schema JSON string length, all / 4
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tool_tokens_empty() {
        assert_eq!(estimate_tool_tokens(&[]), 0);
    }

    #[test]
    fn test_log_token_budget_does_not_panic() {
        // Just verify it doesn't panic — actual log output goes to tracing
        log_token_budget(1000, 500, 2000);
        log_token_budget(0, 0, 0);
    }
}
