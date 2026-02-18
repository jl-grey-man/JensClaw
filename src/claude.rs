use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: serde_json::Value,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessagesResponse {
    pub content: Vec<ResponseContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}

// --- SSE streaming event types ---

/// Top-level SSE event wrapper
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: StreamMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize, content_block: ContentBlockStartData },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaData, usage: Option<MessageDeltaUsage> },
    #[serde(rename = "message_stop")]
    MessageStop {},
    #[serde(rename = "ping")]
    Ping {},
    #[serde(rename = "error")]
    Error { error: StreamError },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StreamMessage {
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum ContentBlockStartData {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessageDeltaData {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessageDeltaUsage {
    pub output_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StreamError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_content_block_text_serialization() {
        let block = ContentBlock::Text {
            text: "hello".into(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "hello");
    }

    #[test]
    fn test_content_block_tool_use_serialization() {
        let block = ContentBlock::ToolUse {
            id: "id_123".into(),
            name: "bash".into(),
            input: json!({"command": "ls"}),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "id_123");
        assert_eq!(json["name"], "bash");
        assert_eq!(json["input"]["command"], "ls");
    }

    #[test]
    fn test_content_block_tool_result_serialization() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "id_123".into(),
            content: "output".into(),
            is_error: Some(true),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "id_123");
        assert_eq!(json["is_error"], true);
    }

    #[test]
    fn test_content_block_tool_result_skips_none_is_error() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "id_123".into(),
            content: "output".into(),
            is_error: None,
        };
        let json = serde_json::to_value(&block).unwrap();
        assert!(json.get("is_error").is_none());
    }

    #[test]
    fn test_message_content_text_serialization() {
        let msg = Message {
            role: "user".into(),
            content: MessageContent::Text("hello".into()),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "hello");
    }

    #[test]
    fn test_message_content_blocks_serialization() {
        let msg = Message {
            role: "assistant".into(),
            content: MessageContent::Blocks(vec![ContentBlock::Text {
                text: "thinking...".into(),
            }]),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "assistant");
        assert!(json["content"].is_array());
        assert_eq!(json["content"][0]["type"], "text");
    }

    #[test]
    fn test_messages_response_deserialization() {
        let json = json!({
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });
        let resp: MessagesResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(resp.content.len(), 1);
        match &resp.content[0] {
            ResponseContentBlock::Text { text } => assert_eq!(text, "Hello!"),
            _ => panic!("Expected Text block"),
        }
        assert_eq!(resp.usage.as_ref().unwrap().input_tokens, 10);
        assert_eq!(resp.usage.as_ref().unwrap().output_tokens, 5);
    }

    #[test]
    fn test_response_content_block_tool_use_deserialization() {
        let json = json!({
            "type": "tool_use",
            "id": "tu_abc",
            "name": "bash",
            "input": {"command": "echo hi"}
        });
        let block: ResponseContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ResponseContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "tu_abc");
                assert_eq!(name, "bash");
                assert_eq!(input["command"], "echo hi");
            }
            _ => panic!("Expected ToolUse block"),
        }
    }

    #[test]
    fn test_messages_request_serialization() {
        let req = MessagesRequest {
            model: "claude-sonnet-4-5-20250929".into(),
            max_tokens: 4096,
            system: json!("You are helpful."),
            messages: vec![Message {
                role: "user".into(),
                content: MessageContent::Text("hi".into()),
            }],
            tools: None,
            stream: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-5-20250929");
        assert_eq!(json["max_tokens"], 4096);
        assert!(json.get("tools").is_none()); // skip_serializing_if None
        assert!(json.get("stream").is_none()); // skip_serializing_if None
    }

    #[test]
    fn test_messages_request_with_tools() {
        let req = MessagesRequest {
            model: "test".into(),
            max_tokens: 100,
            system: json!("sys"),
            messages: vec![],
            tools: Some(vec![ToolDefinition {
                name: "bash".into(),
                description: "Run bash".into(),
                input_schema: json!({"type": "object"}),
            }]),
            stream: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json["tools"].is_array());
        assert_eq!(json["tools"][0]["name"], "bash");
    }

    #[test]
    fn test_content_block_image_serialization() {
        let block = ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".into(),
                media_type: "image/jpeg".into(),
                data: "abc123".into(),
            },
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["source"]["type"], "base64");
        assert_eq!(json["source"]["media_type"], "image/jpeg");
        assert_eq!(json["source"]["data"], "abc123");
    }

    #[test]
    fn test_image_source_serialization() {
        let source = ImageSource {
            source_type: "base64".into(),
            media_type: "image/png".into(),
            data: "ABCDEF".into(),
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["type"], "base64");
        assert_eq!(json["media_type"], "image/png");
    }

    #[test]
    fn test_usage_with_cache_fields() {
        let json = json!({
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 80,
            "cache_read_input_tokens": 20
        });
        let usage: Usage = serde_json::from_value(json).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.cache_creation_input_tokens, 80);
        assert_eq!(usage.cache_read_input_tokens, 20);
    }

    #[test]
    fn test_usage_without_cache_fields_defaults_to_zero() {
        let json = json!({
            "input_tokens": 100,
            "output_tokens": 50
        });
        let usage: Usage = serde_json::from_value(json).unwrap();
        assert_eq!(usage.cache_creation_input_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, 0);
    }

    #[test]
    fn test_stream_event_message_start_deserialization() {
        let json = json!({
            "type": "message_start",
            "message": {
                "usage": {
                    "input_tokens": 500,
                    "output_tokens": 0
                }
            }
        });
        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::MessageStart { message } => {
                assert_eq!(message.usage.unwrap().input_tokens, 500);
            }
            _ => panic!("Expected MessageStart"),
        }
    }

    #[test]
    fn test_stream_event_content_block_delta_text() {
        let json = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });
        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    ContentDelta::TextDelta { text } => assert_eq!(text, "Hello"),
                    _ => panic!("Expected TextDelta"),
                }
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }

    #[test]
    fn test_stream_event_message_delta() {
        let json = json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": "end_turn"
            },
            "usage": {
                "output_tokens": 42
            }
        });
        let event: StreamEvent = serde_json::from_value(json).unwrap();
        match event {
            StreamEvent::MessageDelta { delta, usage } => {
                assert_eq!(delta.stop_reason.as_deref(), Some("end_turn"));
                assert_eq!(usage.unwrap().output_tokens, 42);
            }
            _ => panic!("Expected MessageDelta"),
        }
    }

    #[test]
    fn test_messages_request_with_stream() {
        let req = MessagesRequest {
            model: "test".into(),
            max_tokens: 100,
            system: json!("sys"),
            messages: vec![],
            tools: None,
            stream: Some(true),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
    }
}
