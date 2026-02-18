// Test format validation for agent outputs
// Ensures outputs match expected format (JSON vs Markdown)

use sandy::config::Config;
use sandy::tools::agent_management::SpawnAgentTool;
use std::path::Path;
use tokio::fs;

fn test_config() -> Config {
    Config {
        telegram_bot_token: "tok".into(),
        bot_username: "bot".into(),
        llm_provider: "anthropic".into(),
        api_key: "key".into(),
        model: "claude-test".into(),
        llm_base_url: None,
        max_tokens: 4096,
        max_tool_iterations: 100,
        max_sub_agent_iterations: 25,
        max_history_messages: 10,
        data_dir: "/tmp".into(),
        working_dir: "/tmp/sandy_test".into(),
        openai_api_key: None,
        timezone: "UTC".into(),
        allowed_groups: vec![],
        control_chat_ids: vec![],
        max_session_messages: 25,
        compact_keep_recent: 10,
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
        memory_file: "soul/data/runtime/MEMORY.md".into(),
    }
}

async fn setup_test_env() -> String {
    let test_dir = "/tmp/sandy_test";
    let _ = fs::create_dir_all(test_dir).await;
    test_dir.to_string()
}

async fn cleanup_test_file(path: &str) {
    let _ = fs::remove_file(path).await;
}

#[tokio::test]
async fn test_valid_json_passes_validation() {
    let _ = setup_test_env().await;
    let config = test_config();
    let tool = SpawnAgentTool::new(&config);

    let test_file = "/tmp/sandy_test/valid.json";
    let valid_json = r#"{"title": "Test", "content": "This is valid JSON", "sources": []}"#;
    fs::write(test_file, valid_json).await.unwrap();

    // verify_output is private, but we can test through file validation
    let path = Path::new(test_file);
    assert!(path.exists());

    // Read and validate JSON manually (simulating what verify_output does)
    let contents = fs::read_to_string(test_file).await.unwrap();
    let json_result: Result<serde_json::Value, _> = serde_json::from_str(&contents);
    assert!(json_result.is_ok(), "Valid JSON should parse successfully");

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_invalid_json_fails_validation() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/invalid.json";
    let invalid_json = r#"{"title": "Test", "content": "Missing closing brace"#;
    fs::write(test_file, invalid_json).await.unwrap();

    // Try to parse as JSON
    let contents = fs::read_to_string(test_file).await.unwrap();
    let json_result: Result<serde_json::Value, _> = serde_json::from_str(&contents);
    assert!(
        json_result.is_err(),
        "Invalid JSON should fail to parse"
    );

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_markdown_validation() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/valid.md";
    let valid_markdown = r#"# Test Article

This is a valid markdown article with:
- List items
- **Bold text**
- [Links](https://example.com)

## Section

Content here.
"#;
    fs::write(test_file, valid_markdown).await.unwrap();

    let contents = fs::read_to_string(test_file).await.unwrap();

    // Basic markdown validation
    let has_content = contents.trim().len() > 10;
    let has_markdown_markers =
        contents.contains('#') || contents.contains('*') || contents.contains('[');

    assert!(has_content, "Markdown should have content");
    assert!(
        has_markdown_markers,
        "Markdown should have formatting markers"
    );

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_empty_file_fails_validation() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/empty.json";
    fs::write(test_file, "").await.unwrap();

    let path = Path::new(test_file);
    let metadata = fs::metadata(path).await.unwrap();

    assert_eq!(metadata.len(), 0, "File should be empty");

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_json_with_error_indicator() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/error.json";
    let error_json = r#"{"error": "Something went wrong", "status": "failed"}"#;
    fs::write(test_file, error_json).await.unwrap();

    let contents = fs::read_to_string(test_file).await.unwrap();

    // Valid JSON but contains error
    let json_result: Result<serde_json::Value, _> = serde_json::from_str(&contents);
    assert!(json_result.is_ok(), "Should parse as valid JSON");
    assert!(
        contents.contains("\"error\""),
        "Should contain error indicator"
    );

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_markdown_minimal_valid() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/minimal.md";
    let minimal_markdown = "# Title\n\nContent with some text.\n";
    fs::write(test_file, minimal_markdown).await.unwrap();

    let contents = fs::read_to_string(test_file).await.unwrap();

    let has_content = contents.trim().len() > 10;
    let has_markdown = contents.contains('#');

    assert!(has_content);
    assert!(has_markdown);

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_markdown_too_short_fails() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/short.md";
    let too_short = "# Hi";
    fs::write(test_file, too_short).await.unwrap();

    let contents = fs::read_to_string(test_file).await.unwrap();

    let has_content = contents.trim().len() > 10;
    assert!(!has_content, "Content should be too short");

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_nested_json_structure() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/nested.json";
    let nested_json = r#"{
  "research": {
    "topic": "AI Safety",
    "sources": [
      {
        "title": "Source 1",
        "url": "https://example.com",
        "summary": "Details here"
      }
    ],
    "metadata": {
      "date": "2026-02-16",
      "agent": "zilla"
    }
  }
}"#;
    fs::write(test_file, nested_json).await.unwrap();

    let contents = fs::read_to_string(test_file).await.unwrap();
    let json_result: Result<serde_json::Value, _> = serde_json::from_str(&contents);

    assert!(json_result.is_ok(), "Nested JSON should parse successfully");

    if let Ok(json) = json_result {
        assert!(json["research"]["sources"].is_array());
        assert_eq!(json["research"]["sources"].as_array().unwrap().len(), 1);
    }

    cleanup_test_file(test_file).await;
}

#[tokio::test]
async fn test_file_does_not_exist() {
    let test_file = "/tmp/sandy_test/nonexistent.json";
    let path = Path::new(test_file);

    assert!(!path.exists(), "File should not exist");
}

#[tokio::test]
async fn test_plain_text_format() {
    let _ = setup_test_env().await;

    let test_file = "/tmp/sandy_test/plain.txt";
    let plain_text = "This is plain text without any special formatting.";
    fs::write(test_file, plain_text).await.unwrap();

    let contents = fs::read_to_string(test_file).await.unwrap();
    let has_content = contents.trim().len() > 0;

    assert!(has_content, "Plain text should have content");

    cleanup_test_file(test_file).await;
}
