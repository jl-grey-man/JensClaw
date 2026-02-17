use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

use crate::claude::ToolDefinition;
use crate::tools::{schema_object, ToolResult};

pub struct TranscribeTool {
    api_key: String,
}

impl TranscribeTool {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl crate::tools::Tool for TranscribeTool {
    fn name(&self) -> &str {
        "transcribe_audio"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "transcribe_audio".into(),
            description: "Transcribe audio files to text using Whisper. Accepts path to audio file (mp3, m4a, ogg, wav, etc.)".into(),
            input_schema: schema_object(
                json!({
                    "audio_path": {
                        "type": "string",
                        "description": "Path to the audio file to transcribe"
                    }
                }),
                &["audio_path"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let audio_path = match input.get("audio_path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return ToolResult::error("Missing audio_path parameter".into()),
        };

        let path = PathBuf::from(audio_path);
        if !path.exists() {
            return ToolResult::error(format!("Audio file not found: {}", audio_path));
        }

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(&self.api_key);
        let client = async_openai::Client::with_config(config);

        let request = match async_openai::types::CreateTranscriptionRequestArgs::default()
            .file(audio_path)
            .model("whisper-1")
            .build()
        {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Failed to build request: {e}")),
        };

        match client.audio().transcribe(request).await {
            Ok(response) => ToolResult::success(format!("Transcription:\n\n{}", response.text)),
            Err(e) => ToolResult::error(format!("Transcription failed: {e}")),
        }
    }
}
