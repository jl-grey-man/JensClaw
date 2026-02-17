use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, warn};

use std::collections::HashSet;

use crate::claude::{
    ContentBlock, ImageSource, Message, MessageContent, MessagesRequest, MessagesResponse,
    ResponseContentBlock, ToolDefinition, Usage,
};
use crate::config::Config;
use crate::error::MicroClawError;

/// Remove orphaned `ToolResult` blocks whose `tool_use_id` does not match any
/// `ToolUse` block in the conversation.  This can happen after session
/// compaction splits a tool_use / tool_result pair.
fn sanitize_messages(messages: Vec<Message>) -> Vec<Message> {
    // Collect all tool_use IDs from assistant messages (owned to avoid borrow conflicts).
    let known_ids: HashSet<String> = messages
        .iter()
        .filter(|m| m.role == "assistant")
        .flat_map(|m| match &m.content {
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse { id, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect();

    messages
        .into_iter()
        .filter_map(|msg| {
            if msg.role != "user" {
                return Some(msg);
            }
            match msg.content {
                MessageContent::Blocks(blocks) => {
                    let filtered: Vec<ContentBlock> = blocks
                        .into_iter()
                        .filter(|b| match b {
                            ContentBlock::ToolResult { tool_use_id, .. } => {
                                known_ids.contains(tool_use_id)
                            }
                            _ => true,
                        })
                        .collect();
                    if filtered.is_empty() {
                        None // Drop entirely empty user messages
                    } else {
                        Some(Message {
                            role: msg.role,
                            content: MessageContent::Blocks(filtered),
                        })
                    }
                }
                other => Some(Message {
                    role: msg.role,
                    content: other,
                }),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn send_message(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<MessagesResponse, MicroClawError>;
}

pub fn create_provider(config: &Config) -> Box<dyn LlmProvider> {
    use crate::llm_retry::RetryLlmProvider;

    // Create base provider
    let base_provider: Box<dyn LlmProvider> = match config.llm_provider.trim().to_lowercase().as_str() {
        "anthropic" => Box::new(AnthropicProvider::new(config)),
        _ => Box::new(OpenAiProvider::new(config)),
    };

    // Wrap with retry logic
    Box::new(RetryLlmProvider::new(base_provider))
}

// ---------------------------------------------------------------------------
// Anthropic provider
// ---------------------------------------------------------------------------

pub struct AnthropicProvider {
    http: reqwest::Client,
    api_key: String,
    model: String,
    max_tokens: u32,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(config: &Config) -> Self {
        AnthropicProvider {
            http: reqwest::Client::new(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            base_url: config
                .llm_base_url
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".into()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicApiError {
    error: AnthropicApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AnthropicApiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn send_message(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<MessagesResponse, MicroClawError> {
        let messages = sanitize_messages(messages);

        let request = MessagesRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: system.to_string(),
            messages,
            tools,
        };

        let mut retries = 0u32;
        let max_retries = 3;

        loop {
            let response = self
                .http
                .post(&self.base_url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&request)
                .send()
                .await?;

            let status = response.status();

            if status.is_success() {
                let body = response.text().await?;
                let parsed: MessagesResponse = serde_json::from_str(&body).map_err(|e| {
                    MicroClawError::LlmApi(format!("Failed to parse response: {e}\nBody: {body}"))
                })?;
                return Ok(parsed);
            }

            if status.as_u16() == 429 && retries < max_retries {
                retries += 1;
                let delay = std::time::Duration::from_secs(2u64.pow(retries));
                warn!(
                    "Rate limited, retrying in {:?} (attempt {retries}/{max_retries})",
                    delay
                );
                tokio::time::sleep(delay).await;
                continue;
            }

            let body = response.text().await.unwrap_or_default();
            if let Ok(api_err) = serde_json::from_str::<AnthropicApiError>(&body) {
                return Err(MicroClawError::LlmApi(format!(
                    "{}: {}",
                    api_err.error.error_type, api_err.error.message
                )));
            }
            return Err(MicroClawError::LlmApi(format!("HTTP {status}: {body}")));
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible provider  (OpenAI, OpenRouter, DeepSeek, Groq, Ollama …)
// ---------------------------------------------------------------------------

pub struct OpenAiProvider {
    http: reqwest::Client,
    api_key: String,
    model: String,
    fallback_models: Vec<String>,
    max_tokens: u32,
    chat_url: String,
}

impl OpenAiProvider {
    pub fn new(config: &Config) -> Self {
        let base = config
            .llm_base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1");
        let chat_url = format!("{}/chat/completions", base.trim_end_matches('/'));

        OpenAiProvider {
            http: reqwest::Client::new(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            fallback_models: config.fallback_models.clone(),
            max_tokens: config.max_tokens,
            chat_url,
        }
    }
}

// --- OpenAI response types ---

#[derive(Debug, Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    usage: Option<OaiUsage>,
}

#[derive(Debug, Deserialize)]
struct OaiChoice {
    message: OaiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaiMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OaiToolCall {
    id: String,
    function: OaiFunction,
}

#[derive(Debug, Deserialize)]
struct OaiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OaiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<OaiPromptTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct OaiPromptTokensDetails {
    #[serde(default)]
    cached_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OaiErrorResponse {
    error: OaiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OaiErrorDetail {
    message: String,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn send_message(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<MessagesResponse, MicroClawError> {
        let oai_messages = translate_messages_to_oai(system, &messages);

        // Try primary model first, then fallbacks
        let models_to_try = std::iter::once(&self.model)
            .chain(self.fallback_models.iter())
            .collect::<Vec<_>>();

        let mut last_error = None;

        for (attempt, model) in models_to_try.iter().enumerate() {
            info!("OpenRouter request: model={}, system_len={}, messages={}, max_tokens={} (attempt {}/{})",
                model, system.len(), messages.len(), self.max_tokens, attempt + 1, models_to_try.len());

            let mut body = json!({
                "model": model,
                "max_tokens": self.max_tokens,
                "messages": oai_messages,
                "provider": {
                    "order": ["Anthropic"],
                    "allow_fallbacks": false
                }
            });

        if let Some(ref tool_defs) = tools {
            if !tool_defs.is_empty() {
                body["tools"] = json!(translate_tools_to_oai(tool_defs));
            }
        }

        let mut retries = 0u32;
        let max_retries = 3;

        loop {
            let mut req = self
                .http
                .post(&self.chat_url)
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://github.com/jl-grey-man/JensClaw")
                .header("X-Title", "Sandy ADHD Coach")
                .json(&body);
            if !self.api_key.trim().is_empty() {
                req = req.header("Authorization", format!("Bearer {}", self.api_key));
            }
            let response = req.send().await?;

            let status = response.status();

            let text = response.text().await?;

            // Log response status (body logged on error only)
            info!("OpenRouter response: status={}, body_len={}", status, text.len());

            if status.is_success() {
                // Try parsing as a normal response first
                match serde_json::from_str::<OaiResponse>(&text) {
                    Ok(oai) => return Ok(translate_oai_response(oai)),
                    Err(parse_err) => {
                        // Failed to parse as valid response - check if it's an error wrapped in 200
                        if let Ok(err) = serde_json::from_str::<OaiErrorResponse>(&text) {
                            error!("OpenRouter returned 200 with error: {} | full body: {}",
                                err.error.message, &text[..text.len().min(500)]);

                            // If it's an overload error and we have more models to try, continue
                            if err.error.message.contains("Overloaded") && attempt + 1 < models_to_try.len() {
                                warn!("Model {} overloaded, trying fallback model...", model);
                                last_error = Some(MicroClawError::LlmApi(format!(
                                    "Provider error: {}",
                                    err.error.message
                                )));
                                break; // Break inner loop to try next model
                            }

                            return Err(MicroClawError::LlmApi(format!(
                                "Provider error: {}",
                                err.error.message
                            )));
                        }
                        error!("Failed to parse OpenRouter response: {} | body: {}",
                            parse_err, &text[..text.len().min(500)]);
                        return Err(MicroClawError::LlmApi(format!(
                            "Failed to parse response: {parse_err}\nBody: {text}"
                        )));
                    }
                }
            }

            if status.as_u16() == 429 && retries < max_retries {
                retries += 1;
                let delay = std::time::Duration::from_secs(2u64.pow(retries));
                warn!(
                    "Rate limited, retrying in {:?} (attempt {retries}/{max_retries})",
                    delay
                );
                tokio::time::sleep(delay).await;
                continue;
            }

            error!("OpenRouter error: status={}, body={}", status, &text[..text.len().min(1000)]);
            if let Ok(err) = serde_json::from_str::<OaiErrorResponse>(&text) {
                last_error = Some(MicroClawError::LlmApi(format!("OpenRouter: {} (HTTP {})", err.error.message, status)));
                break; // Try next model
            }
            last_error = Some(MicroClawError::LlmApi(format!("OpenRouter HTTP {status}: {text}")));
            break; // Try next model
        }
        // End of retry loop

        // If we got here without returning, we exhausted retries for this model - try next
        if attempt + 1 < models_to_try.len() {
            warn!("Model {} failed, trying fallback model...", model);
            continue;
        }
    }

    // All models failed
    Err(last_error.unwrap_or_else(|| MicroClawError::LlmApi("All models failed".into())))
    }
}

// ---------------------------------------------------------------------------
// Format translation helpers  (internal Anthropic-style ↔ OpenAI)
// ---------------------------------------------------------------------------

fn translate_messages_to_oai(system: &str, messages: &[Message]) -> Vec<serde_json::Value> {
    // Collect all tool_use IDs present in assistant messages so we can
    // skip orphaned tool_results (e.g. after session compaction).
    let known_tool_ids: std::collections::HashSet<&str> = messages
        .iter()
        .filter(|m| m.role == "assistant")
        .flat_map(|m| match &m.content {
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::ToolUse { id, .. } => Some(id.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect();

    let mut out: Vec<serde_json::Value> = Vec::new();

    // System message — split on cache marker for Anthropic prompt caching
    if !system.is_empty() {
        const CACHE_MARKER: &str = "\n---CACHE_BREAK---\n";
        if let Some(pos) = system.find(CACHE_MARKER) {
            let static_part = &system[..pos];
            let dynamic_part = &system[pos + CACHE_MARKER.len()..];
            // Use content array format with cache_control on the static part
            let mut content = vec![
                json!({
                    "type": "text",
                    "text": static_part,
                    "cache_control": {"type": "ephemeral"}
                }),
            ];
            if !dynamic_part.is_empty() {
                content.push(json!({"type": "text", "text": dynamic_part}));
            }
            out.push(json!({"role": "system", "content": content}));
        } else {
            // No marker — send as plain string (backward compatible)
            out.push(json!({"role": "system", "content": system}));
        }
    }

    for msg in messages {
        match &msg.content {
            MessageContent::Text(text) => {
                out.push(json!({"role": msg.role, "content": text}));
            }
            MessageContent::Blocks(blocks) => {
                if msg.role == "assistant" {
                    // Collect text and tool_calls
                    let text: String = blocks
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    let tool_calls: Vec<serde_json::Value> = blocks
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::ToolUse { id, name, input } => Some(json!({
                                "id": id,
                                "type": "function",
                                "function": {
                                    "name": name,
                                    "arguments": serde_json::to_string(input).unwrap_or_default()
                                }
                            })),
                            _ => None,
                        })
                        .collect();

                    let mut m = json!({"role": "assistant"});
                    if !text.is_empty() || tool_calls.is_empty() {
                        m["content"] = json!(text);
                    }
                    if !tool_calls.is_empty() {
                        m["tool_calls"] = json!(tool_calls);
                    }
                    out.push(m);
                } else {
                    // User role — tool_results, images, or text
                    let has_tool_results = blocks
                        .iter()
                        .any(|b| matches!(b, ContentBlock::ToolResult { .. }));

                    if has_tool_results {
                        // Each tool result → separate "tool" message
                        // Skip orphaned tool_results whose IDs are not in any assistant message
                        for block in blocks {
                            if let ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } = block
                            {
                                if !known_tool_ids.contains(tool_use_id.as_str()) {
                                    continue;
                                }
                                let c = if is_error == &Some(true) {
                                    format!("[Error] {content}")
                                } else {
                                    content.clone()
                                };
                                out.push(json!({
                                    "role": "tool",
                                    "tool_call_id": tool_use_id,
                                    "content": c,
                                }));
                            }
                        }
                    } else {
                        // Images + text → multipart content array
                        let has_images = blocks
                            .iter()
                            .any(|b| matches!(b, ContentBlock::Image { .. }));
                        if has_images {
                            let parts: Vec<serde_json::Value> = blocks
                                .iter()
                                .filter_map(|b| match b {
                                    ContentBlock::Text { text } => {
                                        Some(json!({"type": "text", "text": text}))
                                    }
                                    ContentBlock::Image {
                                        source:
                                            ImageSource {
                                                media_type, data, ..
                                            },
                                    } => {
                                        let url = format!("data:{media_type};base64,{data}");
                                        Some(json!({
                                            "type": "image_url",
                                            "image_url": {"url": url}
                                        }))
                                    }
                                    _ => None,
                                })
                                .collect();
                            out.push(json!({"role": "user", "content": parts}));
                        } else {
                            let text: String = blocks
                                .iter()
                                .filter_map(|b| match b {
                                    ContentBlock::Text { text } => Some(text.as_str()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            out.push(json!({"role": "user", "content": text}));
                        }
                    }
                }
            }
        }
    }

    out
}

fn translate_tools_to_oai(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect()
}

fn translate_oai_response(oai: OaiResponse) -> MessagesResponse {
    let choice = match oai.choices.into_iter().next() {
        Some(c) => c,
        None => {
            return MessagesResponse {
                content: vec![ResponseContentBlock::Text {
                    text: "(empty response)".into(),
                }],
                stop_reason: Some("end_turn".into()),
                usage: None,
            };
        }
    };

    let mut content = Vec::new();

    if let Some(text) = choice.message.content {
        if !text.is_empty() {
            content.push(ResponseContentBlock::Text { text });
        }
    }

    if let Some(tool_calls) = choice.message.tool_calls {
        for tc in tool_calls {
            let input: serde_json::Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_default();
            content.push(ResponseContentBlock::ToolUse {
                id: tc.id,
                name: tc.function.name,
                input,
            });
        }
    }

    if content.is_empty() {
        content.push(ResponseContentBlock::Text {
            text: String::new(),
        });
    }

    let stop_reason = match choice.finish_reason.as_deref() {
        Some("tool_calls") => Some("tool_use".into()),
        Some("length") => Some("max_tokens".into()),
        _ => Some("end_turn".into()),
    };

    let usage = oai.usage.map(|u| {
        let cached = u.prompt_tokens_details.as_ref().map_or(0, |d| d.cached_tokens);
        if cached > 0 {
            info!("Token usage: prompt={}, completion={}, cached={}",
                u.prompt_tokens, u.completion_tokens, cached);
        }
        Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        }
    });

    MessagesResponse {
        content,
        stop_reason,
        usage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // translate_messages_to_oai
    // -----------------------------------------------------------------------

    #[test]
    fn test_translate_messages_system_only() {
        let msgs: Vec<Message> = vec![];
        let out = translate_messages_to_oai("You are a bot.", &msgs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "system");
        assert_eq!(out[0]["content"], "You are a bot.");
    }

    #[test]
    fn test_translate_messages_empty_system_omitted() {
        let msgs: Vec<Message> = vec![];
        let out = translate_messages_to_oai("", &msgs);
        assert!(out.is_empty());
    }

    #[test]
    fn test_translate_messages_text_roundtrip() {
        let msgs = vec![
            Message {
                role: "user".into(),
                content: MessageContent::Text("hello".into()),
            },
            Message {
                role: "assistant".into(),
                content: MessageContent::Text("hi".into()),
            },
        ];
        let out = translate_messages_to_oai("sys", &msgs);
        assert_eq!(out.len(), 3); // system + user + assistant
        assert_eq!(out[1]["role"], "user");
        assert_eq!(out[1]["content"], "hello");
        assert_eq!(out[2]["role"], "assistant");
        assert_eq!(out[2]["content"], "hi");
    }

    #[test]
    fn test_translate_messages_assistant_tool_use() {
        let msgs = vec![Message {
            role: "assistant".into(),
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "Let me check.".into(),
                },
                ContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "bash".into(),
                    input: json!({"command": "ls"}),
                },
            ]),
        }];
        let out = translate_messages_to_oai("", &msgs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "assistant");
        assert_eq!(out[0]["content"], "Let me check.");
        let tc = out[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tc.len(), 1);
        assert_eq!(tc[0]["id"], "t1");
        assert_eq!(tc[0]["function"]["name"], "bash");
    }

    #[test]
    fn test_translate_messages_tool_result() {
        let msgs = vec![
            Message {
                role: "assistant".into(),
                content: MessageContent::Blocks(vec![ContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "glob".into(),
                    input: json!({}),
                }]),
            },
            Message {
                role: "user".into(),
                content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                    tool_use_id: "t1".into(),
                    content: "file1.rs\nfile2.rs".into(),
                    is_error: None,
                }]),
            },
        ];
        let out = translate_messages_to_oai("", &msgs);
        // assistant + tool = 2 messages
        assert_eq!(out.len(), 2);
        assert_eq!(out[1]["role"], "tool");
        assert_eq!(out[1]["tool_call_id"], "t1");
        assert_eq!(out[1]["content"], "file1.rs\nfile2.rs");
    }

    #[test]
    fn test_translate_messages_tool_result_error() {
        let msgs = vec![
            Message {
                role: "assistant".into(),
                content: MessageContent::Blocks(vec![ContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "glob".into(),
                    input: json!({}),
                }]),
            },
            Message {
                role: "user".into(),
                content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                    tool_use_id: "t1".into(),
                    content: "not found".into(),
                    is_error: Some(true),
                }]),
            },
        ];
        let out = translate_messages_to_oai("", &msgs);
        assert_eq!(out[1]["content"], "[Error] not found");
    }

    #[test]
    fn test_translate_messages_orphaned_tool_result_skipped() {
        // tool_result without matching tool_use should be stripped
        let msgs = vec![Message {
            role: "user".into(),
            content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: "orphan_id".into(),
                content: "stale result".into(),
                is_error: None,
            }]),
        }];
        let out = translate_messages_to_oai("", &msgs);
        assert!(out.is_empty());
    }

    #[test]
    fn test_translate_messages_image_block() {
        let msgs = vec![Message {
            role: "user".into(),
            content: MessageContent::Blocks(vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".into(),
                        media_type: "image/png".into(),
                        data: "AAAA".into(),
                    },
                },
                ContentBlock::Text {
                    text: "describe".into(),
                },
            ]),
        }];
        let out = translate_messages_to_oai("", &msgs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "user");
        let content = out[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "image_url");
        assert!(content[0]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "describe");
    }

    // -----------------------------------------------------------------------
    // translate_tools_to_oai
    // -----------------------------------------------------------------------

    #[test]
    fn test_translate_tools_to_oai() {
        let tools = vec![ToolDefinition {
            name: "bash".into(),
            description: "Run bash".into(),
            input_schema: json!({"type": "object", "properties": {"cmd": {"type": "string"}}}),
        }];
        let out = translate_tools_to_oai(&tools);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["type"], "function");
        assert_eq!(out[0]["function"]["name"], "bash");
        assert_eq!(out[0]["function"]["description"], "Run bash");
    }

    // -----------------------------------------------------------------------
    // translate_oai_response
    // -----------------------------------------------------------------------

    #[test]
    fn test_translate_oai_response_text() {
        let oai = OaiResponse {
            choices: vec![OaiChoice {
                message: OaiMessage {
                    content: Some("Hello!".into()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: Some(OaiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                prompt_tokens_details: None,
            }),
        };
        let resp = translate_oai_response(oai);
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(resp.content.len(), 1);
        match &resp.content[0] {
            ResponseContentBlock::Text { text } => assert_eq!(text, "Hello!"),
            _ => panic!("Expected Text"),
        }
        let usage = resp.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 5);
    }

    #[test]
    fn test_translate_oai_response_tool_calls() {
        let oai = OaiResponse {
            choices: vec![OaiChoice {
                message: OaiMessage {
                    content: None,
                    tool_calls: Some(vec![OaiToolCall {
                        id: "call_1".into(),
                        function: OaiFunction {
                            name: "bash".into(),
                            arguments: r#"{"command":"ls"}"#.into(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".into()),
            }],
            usage: None,
        };
        let resp = translate_oai_response(oai);
        assert_eq!(resp.stop_reason.as_deref(), Some("tool_use"));
        match &resp.content[0] {
            ResponseContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "bash");
                assert_eq!(input["command"], "ls");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_translate_oai_response_empty_choices() {
        let oai = OaiResponse {
            choices: vec![],
            usage: None,
        };
        let resp = translate_oai_response(oai);
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
        match &resp.content[0] {
            ResponseContentBlock::Text { text } => assert_eq!(text, "(empty response)"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_translate_oai_response_length_stop() {
        let oai = OaiResponse {
            choices: vec![OaiChoice {
                message: OaiMessage {
                    content: Some("partial".into()),
                    tool_calls: None,
                },
                finish_reason: Some("length".into()),
            }],
            usage: None,
        };
        let resp = translate_oai_response(oai);
        assert_eq!(resp.stop_reason.as_deref(), Some("max_tokens"));
    }

    #[test]
    fn test_translate_oai_response_text_and_tool_calls() {
        let oai = OaiResponse {
            choices: vec![OaiChoice {
                message: OaiMessage {
                    content: Some("thinking...".into()),
                    tool_calls: Some(vec![OaiToolCall {
                        id: "c1".into(),
                        function: OaiFunction {
                            name: "read_file".into(),
                            arguments: r#"{"path":"/tmp/x"}"#.into(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".into()),
            }],
            usage: None,
        };
        let resp = translate_oai_response(oai);
        assert_eq!(resp.content.len(), 2);
        match &resp.content[0] {
            ResponseContentBlock::Text { text } => assert_eq!(text, "thinking..."),
            _ => panic!("Expected Text"),
        }
        match &resp.content[1] {
            ResponseContentBlock::ToolUse { name, .. } => assert_eq!(name, "read_file"),
            _ => panic!("Expected ToolUse"),
        }
    }

    // -----------------------------------------------------------------------
    // create_provider
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_provider_anthropic() {
        let config = Config {
            telegram_bot_token: "tok".into(),
            bot_username: "bot".into(),
            llm_provider: "anthropic".into(),
            api_key: "key".into(),
            model: "claude-sonnet-4-5-20250929".into(),
            llm_base_url: None,
            max_tokens: 8192,
            max_tool_iterations: 100,
            max_history_messages: 50,
            data_dir: "/tmp".into(),
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
            fallback_models: vec![],
            tavily_api_key: None,
            web_port: 3000,
            soul_file: "soul/SOUL.md".into(),
            identity_file: "soul/IDENTITY.md".into(),
            agents_file: "soul/AGENTS.md".into(),
            memory_file: "soul/data/MEMORY.md".into(),
        };
        // Should not panic
        let _provider = create_provider(&config);
    }

    #[test]
    fn test_create_provider_openai() {
        let config = Config {
            telegram_bot_token: "tok".into(),
            bot_username: "bot".into(),
            llm_provider: "openai".into(),
            api_key: "key".into(),
            model: "gpt-5.2".into(),
            llm_base_url: None,
            max_tokens: 8192,
            max_tool_iterations: 100,
            max_history_messages: 50,
            data_dir: "/tmp".into(),
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
            fallback_models: vec![],
            tavily_api_key: None,
            web_port: 3000,
            soul_file: "soul/SOUL.md".into(),
            identity_file: "soul/IDENTITY.md".into(),
            agents_file: "soul/AGENTS.md".into(),
            memory_file: "soul/data/MEMORY.md".into(),
        };
        let _provider = create_provider(&config);
    }

    #[test]
    fn test_translate_messages_user_text_blocks_no_images_no_tool_results() {
        // User message with only text blocks (no images, no tool results) → plain text
        let msgs = vec![Message {
            role: "user".into(),
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "first".into(),
                },
                ContentBlock::Text {
                    text: "second".into(),
                },
            ]),
        }];
        let out = translate_messages_to_oai("", &msgs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "user");
        assert_eq!(out[0]["content"], "first\nsecond");
    }

    // -----------------------------------------------------------------------
    // sanitize_messages
    // -----------------------------------------------------------------------

    #[test]
    fn test_sanitize_messages_removes_orphaned_tool_results() {
        let msgs = vec![
            Message {
                role: "assistant".into(),
                content: MessageContent::Blocks(vec![ContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "bash".into(),
                    input: json!({}),
                }]),
            },
            Message {
                role: "user".into(),
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "t1".into(),
                        content: "ok".into(),
                        is_error: None,
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "orphan".into(),
                        content: "stale".into(),
                        is_error: None,
                    },
                ]),
            },
        ];
        let sanitized = sanitize_messages(msgs);
        assert_eq!(sanitized.len(), 2);
        // The user message should only contain t1's result
        if let MessageContent::Blocks(blocks) = &sanitized[1].content {
            assert_eq!(blocks.len(), 1);
            if let ContentBlock::ToolResult { tool_use_id, .. } = &blocks[0] {
                assert_eq!(tool_use_id, "t1");
            } else {
                panic!("Expected ToolResult");
            }
        } else {
            panic!("Expected Blocks");
        }
    }

    #[test]
    fn test_sanitize_messages_drops_empty_user_message() {
        // User message with only orphaned tool_results → dropped entirely
        let msgs = vec![Message {
            role: "user".into(),
            content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: "orphan".into(),
                content: "stale".into(),
                is_error: None,
            }]),
        }];
        let sanitized = sanitize_messages(msgs);
        assert!(sanitized.is_empty());
    }

    #[test]
    fn test_sanitize_messages_preserves_text_messages() {
        let msgs = vec![
            Message {
                role: "user".into(),
                content: MessageContent::Text("hello".into()),
            },
            Message {
                role: "assistant".into(),
                content: MessageContent::Text("hi".into()),
            },
        ];
        let sanitized = sanitize_messages(msgs);
        assert_eq!(sanitized.len(), 2);
    }
}
