use async_trait::async_trait;
use serde_json::json;
use teloxide::prelude::*;
use teloxide::types::InputFile;
use tracing::{error, info};

use super::{authorize_chat_access, schema_object, Tool, ToolResult};
use crate::claude::ToolDefinition;

pub struct SendFileTool {
    bot: Bot,
}

impl SendFileTool {
    pub fn new(bot: Bot) -> Self {
        SendFileTool { bot }
    }
}

#[async_trait]
impl Tool for SendFileTool {
    fn name(&self) -> &str {
        "send_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "send_file".into(),
            description: "Send a file to the user via Telegram. Use this to share research results, articles, or any output files with the user. The file will be sent as a document attachment.".into(),
            input_schema: schema_object(
                json!({
                    "chat_id": {
                        "type": "integer",
                        "description": "The chat ID to send the file to (from system prompt)"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to send (e.g., '/mnt/storage/tasks/output.md')"
                    },
                    "caption": {
                        "type": "string",
                        "description": "Optional caption/message to include with the file"
                    }
                }),
                &["chat_id", "file_path"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let chat_id = match input.get("chat_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: chat_id".into()),
        };

        let file_path = match input.get("file_path").and_then(|v| v.as_str()) {
            Some(path) => path,
            None => return ToolResult::error("Missing required parameter: file_path".into()),
        };

        let caption = input.get("caption").and_then(|v| v.as_str());

        // Check permissions
        if let Err(e) = authorize_chat_access(&input, chat_id) {
            return ToolResult::error(e);
        }

        // Check path safety (prevent leaking sensitive files)
        if let Err(e) = super::path_guard::check_path(file_path) {
            return ToolResult::error(e);
        }

        info!("Sending file to chat {}: {}", chat_id, file_path);

        // Check if file exists
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            error!("File not found: {}", file_path);
            return ToolResult::error(format!("File not found: {}", file_path));
        }

        // Get file size for logging
        let file_size = match std::fs::metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(e) => {
                error!("Failed to read file metadata: {}", e);
                return ToolResult::error(format!("Failed to read file metadata: {}", e));
            }
        };

        // Create InputFile from path
        let input_file = InputFile::file(path);

        // Send the document
        let send_result = if let Some(cap) = caption {
            self.bot
                .send_document(ChatId(chat_id), input_file)
                .caption(cap)
                .await
        } else {
            self.bot.send_document(ChatId(chat_id), input_file).await
        };

        match send_result {
            Ok(_) => {
                info!(
                    "Successfully sent file {} ({} bytes) to chat {}",
                    file_path, file_size, chat_id
                );
                ToolResult::success(format!(
                    "File sent successfully: {} ({} bytes)",
                    file_path, file_size
                ))
            }
            Err(e) => {
                error!("Failed to send file to Telegram: {}", e);
                ToolResult::error(format!("Failed to send file to Telegram: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_file_tool_name_and_definition() {
        let bot = Bot::new("test_token");
        let tool = SendFileTool::new(bot);

        assert_eq!(tool.name(), "send_file");

        let def = tool.definition();
        assert_eq!(def.name, "send_file");
        assert!(!def.description.is_empty());
        assert!(def.input_schema["properties"]["chat_id"].is_object());
        assert!(def.input_schema["properties"]["file_path"].is_object());
        assert!(def.input_schema["properties"]["caption"].is_object());

        let required = def.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
        assert!(required.contains(&json!("chat_id")));
        assert!(required.contains(&json!("file_path")));
    }
}
